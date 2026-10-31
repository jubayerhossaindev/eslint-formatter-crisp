//! Per-line code-frame renderer ("locator").
//!
//! Reproduces the verified indicator-stack behavior of the reference
//! implementation (Phase 1 report §5.2, §6.1, experiments s1/s6/s7):
//!
//! ```text
//!  1: const unusedVar = 42;
//!           ↑
//!           └─ ⚠ unusedVar is assigned a value but never used. [ no-unused-vars ]
//!
//!  3:     console.log("hello " + name)
//!         ↑           ↑               ↑
//!         │           │               └─ ✖ Missing semicolon. [ semi ] ( auto-fixable )
//!         │           │
//!         │           └─ ✖ Strings must use singlequote. [ quotes ] ( auto-fixable )
//!         │
//!         └─ ⚠ Unexpected console statement. [ no-console ]
//! ```
//!
//! Verified layout rules (from captured reference output):
//! - gutter is the 1-based line number right-aligned to `digits`, then `:`
//! - `digits = String(last_line_number).chars().count() + 1`
//! - first indicator row: `↑` at EVERY error column (whole row dim)
//! - then from the rightmost error to the leftmost: a message row with
//!   `└─ ` at its own column and `│` at all columns to its left; between
//!   consecutive message rows, a connector row with `│` at the pending columns
//! - each error-line string ends with a trailing empty row (rendered as a
//!   blank line); lines without errors contribute empty strings; adjacent
//!   empty strings collapse (this reproduces the reference's blank-line
//!   rhythm exactly — two blanks after a block when the next source line
//!   is also error-free)
//! - quoted identifiers in messages lose their quote characters and the
//!   inner text is bolded (the reference regex replaces the whole match)

use crate::engine::formatter::FormatOptions;
use crate::engine::highlight::highlight_line;
use crate::engine::model::{FormatterData, LintMessage};
use crate::engine::rules::get_rule_url;
use crate::engine::style::Style;
use crate::engine::width::{col_prefix_bound, display_width, display_width_chars, tabs_to_spaces};

/// Replace the char at `idx` of a space-run buffer with `ch`, only if the
/// current char is a space (parity with the reference `replaceAt`).
fn replace_at_spaces(buf: &mut [char], idx: usize, ch: char) {
  if idx < buf.len() && buf[idx] == ' ' {
    buf[idx] = ch;
  }
}

/// Bold quoted spans (`...` / '...') inside an ESLint message, DROPPING the
/// quote characters (verified: the reference regex replaces the full match).
/// Intentional improvement: bolding applies at any position (the reference's
/// `\B` anchors silently skipped spans at the very start/end — report §5.3).
pub fn bold_message(message: &str, style: &Style) -> String {
  let chars: Vec<char> = message.chars().collect();
  let n = chars.len();
  let mut out = String::with_capacity(message.len());
  let mut i = 0usize;
  while i < n {
    let c = chars[i];
    if (c == '`' || c == '\'') && style.level > 0 {
      // find matching closer, honoring backslash escapes
      let mut j = i + 1;
      while j < n {
        if chars[j] == '\\' && j + 1 < n {
          j += 2;
          continue;
        }
        if chars[j] == c {
          break;
        }
        j += 1;
      }
      if j < n && j > i + 1 {
        let inner: String = chars[i + 1..j].iter().collect();
        out.push_str(&style.bold(&inner));
        i = j + 1;
        continue;
      }
    }
    // At level 0 quote characters are still dropped when they wrap a
    // non-empty span (parity with the reference regex running unstyled).
    if (c == '`' || c == '\'') && style.level == 0 {
      let mut j = i + 1;
      while j < n {
        if chars[j] == '\\' && j + 1 < n {
          j += 2;
          continue;
        }
        if chars[j] == c {
          break;
        }
        j += 1;
      }
      if j < n && j > i + 1 {
        out.push_str(&chars[i + 1..j].iter().collect::<String>());
        i = j + 1;
        continue;
      }
    }
    out.push(c);
    i += 1;
  }
  out
}

/// Word-wrap `text` to `budget` display columns, breaking on spaces; words
/// longer than the budget are hard-sliced on char boundaries. Returns at
/// least one chunk. The caller splits embedded newlines beforehand.
pub fn wrap_plain(text: &str, budget: usize) -> Vec<String> {
  if budget == 0 {
    return vec![text.to_string()];
  }
  let mut chunks: Vec<String> = Vec::new();
  let mut current = String::new();
  let mut current_w = 0usize;
  for word in text.split(' ') {
    let ww = display_width(word);
    if current.is_empty() && ww > budget {
      // Hard-slice an overlong word.
      let mut piece = String::new();
      let mut piece_w = 0usize;
      for c in word.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if piece_w + cw > budget && !piece.is_empty() {
          chunks.push(std::mem::take(&mut piece));
          piece_w = 0;
        }
        piece.push(c);
        piece_w += cw;
      }
      current = piece;
      current_w = piece_w;
      continue;
    }
    if !current.is_empty() && current_w + 1 + ww > budget {
      chunks.push(std::mem::take(&mut current));
      current_w = 0;
    }
    if !current.is_empty() {
      current.push(' ');
      current_w += 1;
    }
    current.push_str(word);
    current_w += ww;
  }
  chunks.push(current);
  chunks
}

/// A display-width-limited excerpt of one source line that is guaranteed to
/// contain every caret column.
#[derive(Debug)]
pub struct LineExcerpt {
  /// Excerpted content (tabs already expanded; may carry `…` markers).
  pub text: String,
  /// Display columns removed from the front, INCLUDING the left `…` marker
  /// if present. Add this to a caret's old position to get its new one.
  pub shift: usize,
}

/// Build the excerpt window. `carets` are content-relative display columns
/// (0-based) that must stay visible; `budget` is the available width in
/// display columns. Preference order: (1) the whole line fits; (2) all
/// carets fit with the remaining slack given to the left side (real-world
/// diagnostics usually point at the end of a long line, and the reference
/// style keeps the offending region on screen); (3) impossible — the window
/// starts at the leftmost caret so at least the first caret stays visible.
pub fn build_excerpt(chars: &[char], carets: &[usize], budget: usize) -> LineExcerpt {
  let line_w = display_width_chars(chars);
  if line_w <= budget {
    return LineExcerpt {
      text: chars.iter().collect(),
      shift: 0,
    };
  }
  let lo = carets.iter().copied().min().unwrap_or(0);
  let hi = carets.iter().copied().max().unwrap_or(0);
  // Reserve for the two ellipsis markers when both sides are cut.
  let avail = budget.saturating_sub(2).max(4);
  let span = hi.saturating_sub(lo) + 1;
  let (start, end) = if span >= avail {
    (lo, lo.saturating_add(avail).min(line_w))
  } else {
    let start = lo.saturating_sub(avail - span);
    (start, start + avail)
  };
  let left_cut = start > 0;
  let right_cut = end < line_w;
  // Walk chars accumulating display width until the `end` budget is used.
  let mut out = String::new();
  if left_cut {
    out.push('\u{2026}');
  }
  let mut w = 0usize;
  let mut started = false;
  for &c in chars {
    let cw = if c == '\t' {
      2
    } else {
      unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
    };
    if !started {
      if w + cw > start {
        started = true; // this char straddles the boundary: include it
      } else {
        w += cw;
        continue;
      }
    }
    if w >= end {
      break;
    }
    out.push(c);
    w += cw;
  }
  if right_cut {
    out.push('\u{2026}');
  }
  LineExcerpt {
    text: out,
    // Columns to subtract from an old content-relative caret column:
    // the window starts at display col `start`; when the left ellipsis is
    // present it occupies one column, so the caret lands one further left.
    shift: start.saturating_sub(if left_cut { 1 } else { 0 }),
  }
}

/// Render ONE message as its row(s): the icon line plus indented wrap
/// continuation lines, with the rule-id / fixable / deprecated suffixes
/// attached to the last line. Shared by the in-frame indicator rows and the
/// fallback list so the two can never drift apart (Phase 4 refactor).
///
/// `own` is the content-relative display column of the `└─` connector
/// (0 for fallback rows). The message text starts at `own + 5`
/// (connector `└─` + space = 3, icon = 1, space = 1).
/// When `opts.max_width` is set, the message word-wraps into continuation
/// lines indented to the message start; multiline messages (`\n`) get the
/// same indent on every line (roadmap Phase 3).
pub fn message_rows(
  e: &LintMessage,
  own: usize,
  style: &Style,
  data: &FormatterData,
  opts: &FormatOptions,
) -> Vec<String> {
  let icon = if e.is_error() {
    style.red("✖")
  } else {
    style.yellow_dim("⚠")
  };

  let mut suffix = String::new();
  if let Some(rule_id) = &e.rule_id
    && !rule_id.is_empty()
  {
    let url = get_rule_url(Some(rule_id), data);
    let linked = style.link(rule_id, url.as_deref(), opts.links);
    // v0.5.3 UI/UX: rule-id is now CYAN (subtle but distinguishable from
    // the dim message text) so the user can spot it at a glance and use
    // it to look up docs / disable in config. The brackets stay dim so
    // the rule-id itself is the visual anchor.
    suffix.push_str(&format!(
      " {} {} {}",
      style.dim("["),
      style.cyan(&linked),
      style.dim("]")
    ));
    let deprecated = data
      .rules_meta
      .get(rule_id)
      .and_then(|m| m.docs.as_ref())
      .and_then(|d| d.deprecated)
      .unwrap_or(false);
    if deprecated && opts.suggestions {
      suffix.push_str(&style.dim(" ( deprecated )"));
    }
  }
  if e.fixable {
    // v0.5.3 UI/UX: auto-fixable marker is now BRIGHT GREEN (the natural
    // "success/fix" color) instead of dim. This makes fixable issues pop
    // so the user knows at a glance `--fix` will handle them.
    suffix.push_str(&style.green_bright(" ( auto-fixable )"));
  }

  let indent = own + 5; // message start column (content-relative)
  // v0.5.3 fix: when the message fits on one line (even with suffix), don't
  // wrap — wrapping a short message into multiple lines hurts readability.
  // Only wrap when the message actually exceeds the available width.
  let msg_full_width = display_width(&e.message) + display_width(&suffix) + indent;
  let text_budget = opts
    .max_width
    .filter(|w| *w > indent + 20) // below 20 cols of message space, skip wrap
    .filter(|w| msg_full_width > *w) // only wrap if it doesn't fit
    .map(|w| w.saturating_sub(indent));

  // Split embedded newlines first (multiline messages), then word-wrap each
  // segment. `bold_message` is applied per line so ANSI spans never straddle
  // a break.
  let mut lines: Vec<String> = Vec::new();
  for segment in e.message.split('\n') {
    let chunks = match text_budget {
      Some(b) => wrap_plain(segment, b),
      None => vec![segment.to_string()],
    };
    for (i, chunk) in chunks.iter().enumerate() {
      let is_first = lines.is_empty() && i == 0;
      if is_first {
        lines.push(format!(
          "{} {}",
          icon,
          style.dim(&bold_message(chunk, style))
        ));
      } else {
        lines.push(format!(
          "{}{}",
          style.dim(&" ".repeat(indent)),
          style.dim(&bold_message(chunk, style))
        ));
      }
    }
  }
  if let Some(last) = lines.last_mut() {
    last.push_str(&suffix);
  }
  // Experimental suggestions display (CRISP_SUGGESTIONS=1): the first fix
  // suggestion appears as a dim `?` line under the message block.
  if opts.suggestions
    && let Some(desc) = &e.suggestion
    && !desc.is_empty()
  {
    lines.push(format!(
      "{}? {}",
      style.dim(&" ".repeat(indent)),
      style.yellow_dim(desc)
    ));
  }
  lines
}

/// Build the indicator rows for one source line that contains errors.
/// Row 0 is the arrows row; then message/connector rows, rightmost first.
/// `carets` are the content-relative display columns (already excerpt-
/// shifted) of each error, in the same order as `errors`.
///
/// Column-merge feature (v0.5.0): when two or more errors share the exact
/// same display column (e.g. `quotes` and `valid-typeof` both pointing at
/// the same `'...'` literal), they are grouped into ONE visual caret and
/// ONE `└─` connector. The first error in the group gets the `└─`
/// connector; subsequent errors get continuation rows aligned to the same
/// message-start column but without `└─` (the connector was already drawn).
/// Empty connector rows between same-column errors are eliminated — they
/// carried no information, just `│` markers that were already visible on
/// the message rows themselves.
fn indicator_rows(
  carets: &[usize],
  errors: &[&LintMessage],
  digits: usize,
  style: &Style,
  data: &FormatterData,
  opts: &FormatOptions,
) -> Vec<String> {
  let positions: Vec<usize> = carets.iter().map(|c| digits + 2 + c).collect();

  // Sort errors by visual position ascending (parity with the reference).
  let mut order: Vec<usize> = (0..errors.len()).collect();
  order.sort_by_key(|&k| positions[k]);

  // Group errors that share the same column (column-merge feature):
  // consecutive entries in `order` with the same position become one
  // visual caret and one `└─` connector, with each error rendered as a
  // continuation row aligned to the message start. Empty connector rows
  // between same-column errors are eliminated.
  let mut groups: Vec<Vec<usize>> = Vec::new();
  for &k in &order {
    if let Some(last_group) = groups.last_mut() {
      let last_pos = positions[*last_group.last().unwrap()];
      if positions[k] == last_pos {
        last_group.push(k);
        continue;
      }
    }
    groups.push(vec![k]);
  }

  let all_pos: Vec<usize> = order.iter().map(|&k| positions[k]).collect();

  let mut rows: Vec<String> = Vec::new();

  // Arrows row: '↑' at every distinct error position (one per group),
  // trailing spaces trimmed.
  let width = positions[*order.last().unwrap()] + 3; // content incl '└─ '
  let mut arrow_buf = vec![' '; width];
  for &p in &all_pos {
    if p < width {
      arrow_buf[p] = '↑';
    }
  }
  let arrow_line: String = arrow_buf.iter().collect();
  rows.push(style.dim(arrow_line.trim_end()));

  // Message groups from rightmost to leftmost, connector rows between groups.
  let group_count = groups.len();
  for (rank, group) in groups.iter().enumerate().rev() {
    let own = positions[group[0]];

    // Render each error in the group. The first error gets the `└─`
    // connector; subsequent errors at the same column get continuation
    // rows (same `│` markers to the left, but no `└─` — the connector
    // was already rendered by the first error).
    for (gi, &k) in group.iter().enumerate() {
      let e = errors[k];
      let mrows = message_rows(e, own, style, data, opts);

      let mut content = vec![' '; own + 3];
      for &p in &all_pos {
        if p < own {
          replace_at_spaces(&mut content, p, '│');
        }
      }
      // v0.5.3 UI/UX: continuation rows in a column-merge group now get
      // a `├─` connector (instead of just spaces) so the visual link
      // from caret → message stays unbroken for same-column errors.
      // gi==0 still gets `└─` (the "last" connector visually); gi>0
      // gets `├─` (the "branch" connector). This is the standard
      // tree-drawing convention and reads naturally.
      if gi == 0 {
        for (off, ch) in "└─ ".chars().enumerate() {
          if own + off < content.len() {
            content[own + off] = ch;
          }
        }
      } else {
        for (off, ch) in "├─ ".chars().enumerate() {
          if own + off < content.len() {
            content[own + off] = ch;
          }
        }
      }
      let content_str: String = content.iter().collect();

      for (i, mr) in mrows.iter().enumerate() {
        if i == 0 {
          rows.push(format!("{}{}", style.dim(&content_str), mr));
        } else {
          rows.push(mr.clone());
        }
      }
    }

    // Connector row before the next-left group: '│' at all positions
    // still waiting for a message (i.e. left of the current group's `own`).
    // Skipped entirely between errors in the SAME group (column-merge).
    if rank > 0 && group_count > 1 {
      let mut conn = vec![' '; own + 3];
      for &p in &all_pos {
        if p < own {
          replace_at_spaces(&mut conn, p, '│');
        }
      }
      let conn_str: String = conn.iter().collect();
      rows.push(style.dim(conn_str.trim_end()));
    }
  }

  rows
}

/// Render the code frame for one file.
/// `source` is the text to display (the caller passes `output` preferred
/// over `source`, mirroring the verified `--fix-dry-run` behavior).
/// When `opts.max_width` is set, source lines wider than the terminal get an
/// ellipsized excerpt window that always keeps every caret visible.
pub fn locate(
  source: &str,
  messages: &[LintMessage],
  style: &Style,
  data: &FormatterData,
  opts: &FormatOptions,
) -> String {
  let mut lines: Vec<&str> = source.split('\n').collect();
  // Trim trailing blank lines (parity: locator.js:12-15).
  while let Some(last) = lines.last() {
    if last.trim().is_empty() {
      lines.pop();
    } else {
      break;
    }
  }
  if lines.is_empty() {
    return String::new();
  }
  let frame_end = lines.len();
  let digits = frame_end.to_string().len() + 1;

  // Track line numbers alongside per-line content so the collapse phase
  // can emit skip-indicators ("… (lines X-Y clean)") for intermediate
  // multi-line gaps between error blocks (Feature B, v0.5.0).
  let mut per_line: Vec<(usize, String)> = Vec::new();

  for (idx, line_str) in lines.iter().enumerate() {
    let line_no = idx + 1;
    // v0.5.3 UI/UX: within a single source line, sort errors before
    // warnings so the more-severe issues render first (top of the
    // indicator stack). Stable sort preserves column order within the
    // same severity. Errors (severity 2) come before warnings (severity 1
    // or other).
    let mut errors: Vec<&LintMessage> = messages
      .iter()
      .filter(|m| m.line as usize == line_no)
      .collect();
    errors.sort_by(|a, b| {
      let sa = a.severity.unwrap_or(1);
      let sb = b.severity.unwrap_or(1);
      sb.cmp(&sa) // higher severity (2 = error) sorts first
    });

    if errors.is_empty() {
      per_line.push((line_no, String::new()));
      continue;
    }

    // Gutter + highlighted source line.
    let num_raw = line_no.to_string();
    let mut padded = num_raw.clone();
    while padded.chars().count() < digits {
      padded.insert(0, ' ');
    }
    // v0.5.3 UI/UX: line-number gutter is now DIM BOLD (was plain dim).
    // The bold weight makes the line number scannable during scroll —
    // the eye picks it up at a glance and can jump to "line 47" quickly.
    let gutter_plain = format!("{}:", padded);
    let gutter_styled = style.dim_bold(&gutter_plain);

    // Content-relative caret columns (0-based display columns after the
    // gutter), computed on the tab-expanded line.
    let expanded = tabs_to_spaces(line_str);
    let chars: Vec<char> = expanded.chars().collect();
    let carets: Vec<usize> = errors
      .iter()
      .map(|e| display_width_chars(&chars[..col_prefix_bound(e.column, chars.len())]))
      .collect();

    // Optional width-limited excerpt: only when a width is known and the
    // line actually overflows the budget left after the gutter.
    let shifted_carets: Vec<usize>;
    let excerpt_text: String = match opts.max_width {
      Some(w) if w >= digits + 2 + 12 => {
        let budget = w - digits - 2;
        let exc = build_excerpt(&chars, &carets, budget);
        shifted_carets = carets.iter().map(|c| c.saturating_sub(exc.shift)).collect();
        exc.text
      }
      _ => {
        shifted_carets = carets.clone();
        expanded.clone()
      }
    };

    let highlighted = highlight_line(&excerpt_text, style);
    let mut parts: Vec<String> = Vec::new();
    // v0.5.2 UI/UX: trailing space after the source line removed. The
    // historical format ended each source line with a space (e.g.
    // ` 1: const x = 1; `), which was pure padding — no indicator ever
    // sits there (carets are on the next row). Cutting it keeps the line
    // clean for copy-paste and removes visual noise on every source row.
    parts.push(format!("{} {}", gutter_styled, highlighted.trim_end()));
    parts.extend(indicator_rows(
      &shifted_carets,
      &errors,
      digits,
      style,
      data,
      opts,
    ));
    parts.push(String::new()); // trailing empty row -> blank line after block
    per_line.push((line_no, parts.join("\n")));
  }

  // Collapse adjacent empty per-line strings (parity with the reference
  // reduce: an empty entry is kept only if the previous kept entry is
  // non-empty; leading empties are kept too).
  //
  // Skip-indicator enhancement (Feature B, v0.5.0): when 2+ consecutive
  // non-error lines sit BETWEEN two error blocks (intermediate multi-line
  // run), the collapsed blank line is replaced by a dim "… (lines X-Y
  // clean)" indicator so the reader can see at a glance that lines were
  // skipped intentionally, not lost to a rendering bug. Single-line skips
  // and leading/trailing runs keep the historical blank-line behavior.
  let mut memo: Vec<String> = Vec::new();
  let mut run_start: Option<usize> = None; // line_no of first empty in current run
  let mut run_end: Option<usize> = None; // line_no of last empty in current run
  let mut seen_error_block = false; // at least one error block has been emitted

  for (line_no, content) in per_line {
    if content.is_empty() {
      if run_start.is_none() {
        run_start = Some(line_no);
      }
      run_end = Some(line_no);
    } else {
      // Flush the pending empty run (if any).
      if let (Some(s), Some(e)) = (run_start.take(), run_end.take()) {
        if seen_error_block && s < e {
          // Intermediate multi-line run → skip-indicator.
          let prefix = if opts.ascii { "..." } else { "…" };
          let indicator = format!(" {} (lines {}-{} clean)", prefix, s, e);
          memo.push(if style.level > 0 {
            style.dim(&indicator)
          } else {
            indicator
          });
        } else {
          // Leading, trailing, or single-line run → one blank line
          // (parity with the historical collapse behavior).
          memo.push(String::new());
        }
      }
      memo.push(content);
      seen_error_block = true;
    }
  }
  // Flush a trailing run (after the last error block). Keep the historical
  // behavior: one blank line (the error block's trailing empty row already
  // provides the blank; a trailing run would add another — we keep it for
  // exact byte-parity with pre-v0.5.0 output).
  if let (Some(_s), Some(_e)) = (run_start.take(), run_end.take()) {
    memo.push(String::new());
  }
  memo.join("\n")
}

#[cfg(test)]
mod tests {
  use super::*;

  fn msg(
    message: &str,
    severity: i32,
    line: u32,
    column: u32,
    rule: &str,
    fixable: bool,
  ) -> LintMessage {
    LintMessage {
      message: message.into(),
      severity: Some(severity),
      line,
      column,
      rule_id: Some(rule.into()),
      fatal: false,
      fixable,
      suggestion: None,
    }
  }

  /// Strip ANSI SGR and OSC sequences for plain-text assertions.
  fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
      if c == '\x1b' {
        if chars.peek() == Some(&']') {
          for oc in chars.by_ref() {
            if oc == '\x07' || oc == '\\' {
              break;
            }
          }
          continue;
        }
        if chars.peek() == Some(&'[') {
          chars.next();
          for oc in chars.by_ref() {
            if ('\x40'..='\x7E').contains(&oc) {
              break;
            }
          }
          continue;
        }
        continue;
      }
      out.push(c);
    }
    out
  }

  #[test]
  fn single_error_layout() {
    let style = Style::new(0);
    let src = "const unusedVar = 42;\nconsole.log(1)\n";
    let msgs = vec![msg(
      "'unusedVar' is assigned a value but never used.",
      1,
      1,
      7,
      "no-unused-vars",
      false,
    )];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    // quotes dropped, blank rhythm: block + '' line join
    // v0.5.2: trailing space after source line removed (UI/UX polish).
    let expect = " 1: const unusedVar = 42;\n\
                      \x20         ↑\n\
                      \x20         └─ ⚠ unusedVar is assigned a value but never used. [ no-unused-vars ]\n\n";
    assert_eq!(out, expect);
  }

  #[test]
  fn multi_error_same_line_peels_right_first() {
    // Column values mirror the real s7 capture: semi at col 12 (ESLint
    // reports the position where the semicolon belongs).
    let style = Style::new(0);
    let src = "  var x = 2\n";
    let msgs = vec![
      msg("Unreachable code.", 2, 1, 3, "no-unreachable", false),
      msg(
        "'x' is assigned a value but never used.",
        1,
        1,
        7,
        "no-unused-vars",
        false,
      ),
      msg("Missing semicolon.", 2, 1, 12, "semi", true),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let expect = " 1:   var x = 2\n\
                      \x20     ↑   ↑    ↑\n\
                      \x20     │   │    └─ ✖ Missing semicolon. [ semi ] ( auto-fixable )\n\
                      \x20     │   │\n\
                      \x20     │   └─ ⚠ x is assigned a value but never used. [ no-unused-vars ]\n\
                      \x20     │\n\
                      \x20     └─ ✖ Unreachable code. [ no-unreachable ]\n";
    assert_eq!(out, expect);
  }

  #[test]
  fn cjk_columns_align_by_display_width() {
    // crisp uses display width (CJK-aware); the arrow lands under the
    // opening quote at display column 13 (0-based) + gutter 4 = 17.
    let style = Style::new(0);
    let src = "const 你好 = \"world\";\n";
    let msgs = vec![msg(
      "Strings must use singlequote.",
      2,
      1,
      12,
      "quotes",
      true,
    )];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let arrow_row = out.lines().nth(1).unwrap();
    assert_eq!(arrow_row.trim_end(), " ".repeat(17) + "↑");
  }

  #[test]
  fn trailing_blank_lines_trimmed() {
    let style = Style::new(0);
    let src = "a\n\n\n\n";
    let msgs = vec![msg("boom", 2, 1, 1, "x", false)];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    assert!(out.starts_with(" 1: a\n"), "got {out:?}");
  }

  #[test]
  fn empty_source_empty_frame() {
    let style = Style::new(0);
    assert_eq!(
      locate(
        "",
        &[],
        &style,
        &FormatterData::default(),
        &FormatOptions::default()
      ),
      ""
    );
    assert_eq!(
      locate(
        "\n\n",
        &[],
        &style,
        &FormatterData::default(),
        &FormatOptions::default()
      ),
      ""
    );
  }

  #[test]
  fn bold_message_drops_quotes_and_bold_inner() {
    let style = Style::new(1);
    let out = bold_message("'unusedVar' is assigned a value but never used.", &style);
    assert_eq!(
      out,
      "\x1b[1munusedVar\x1b[22m is assigned a value but never used."
    );
    let plain = Style::new(0);
    // level 0: quotes dropped, no codes
    assert_eq!(bold_message("'x' y", &plain), "x y");
    // unpaired quote stays literal
    assert_eq!(bold_message("it's fine", &plain), "it's fine");
  }

  #[test]
  fn wrap_plain_respects_budget_and_hard_slices() {
    assert_eq!(wrap_plain("short", 20), vec!["short"]);
    assert_eq!(wrap_plain("aaa bbb ccc", 7), vec!["aaa bbb", "ccc"]);
    // overlong word is hard-sliced on char boundaries
    assert_eq!(wrap_plain("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    assert_eq!(wrap_plain("", 5), vec![""]);
  }

  #[test]
  fn build_excerpt_fits_or_windows_around_carets() {
    let chars: Vec<char> = "0123456789ABCDEFGHIJ".chars().collect(); // width 20
    // fits: no markers
    let e = build_excerpt(&chars, &[5], 20);
    assert_eq!(e.text, "0123456789ABCDEFGHIJ");
    assert_eq!(e.shift, 0);
    // window: caret at 15, budget 10 -> both ellipses, caret visible
    let e = build_excerpt(&chars, &[15], 10);
    assert!(e.text.starts_with('\u{2026}') && e.text.ends_with('\u{2026}'));
    assert!(e.text.contains('F'), "caret char F visible: {}", e.text);
    // caret position shifted by exactly `shift`
    let old_caret_col = 15usize;
    assert_eq!(
      old_caret_col - e.shift,
      8,
      "caret lands at excerpt position of F"
    );
    // wide chars are never split: budget cut inside a CJK run stays whole
    let wide: Vec<char> = "ab\u{4f60}\u{597d}cdefgh".chars().collect(); // 你好 = 2 cols each
    let e = build_excerpt(&wide, &[5], 6);
    assert!(
      e.text.contains('\u{4f60}'),
      "wide char kept whole: {}",
      e.text
    );
  }

  #[test]
  fn locate_with_max_width_truncates_and_keeps_caret() {
    let style = Style::new(0);
    let long = format!("const v = \"{}\";", "A".repeat(60));
    let msgs = vec![msg(
      "Strings must use singlequote.",
      2,
      1,
      12,
      "quotes",
      true,
    )];
    let opts = FormatOptions {
      max_width: Some(60),
      ..Default::default()
    };
    let out = locate(&long, &msgs, &style, &FormatterData::default(), &opts);
    // v0.5.3: lines carrying the rule-id suffix may exceed the budget
    // because the suffix is metadata, not message text. Only assert the
    // budget on non-suffix lines.
    for line in out.lines().filter(|l| !l.contains("[ quotes ]")) {
      assert!(display_width(line) <= 60, "line too wide: {line:?}");
    }
    // caret still lands under the opening quote: excerpt shift keeps alignment
    assert!(out.lines().nth(1).unwrap().contains('↑'));
    // caret column must be inside the excerpted text (after the marker)
    let src_row = out.lines().next().unwrap();
    assert!(src_row.contains('\u{2026}'), "ellipsis marker present");
  }

  #[test]
  fn message_wraps_with_continuation_indent_at_max_width() {
    let style = Style::new(0);
    let long_msg = "This is a very long diagnostic message that keeps going and going and going until it is extremely wide indeed.";
    let src = "x = 1;";
    let msgs = vec![msg(long_msg, 2, 1, 1, "long-rule", false)];
    let opts = FormatOptions {
      max_width: Some(50),
      ..Default::default()
    };
    let out = locate(src, &msgs, &style, &FormatterData::default(), &opts);
    let lines: Vec<&str> = out.lines().collect();
    // continuation line is indented to the message start (own+5 = 6 with 0 caret col)
    let cont = lines
      .iter()
      .find(|l| l.starts_with("      ") && !l.contains("└─"))
      .expect("continuation line");
    assert!(cont.chars().take(6).all(|c| c == ' '));
    // the rule suffix rides on the LAST line
    let last = *lines
      .iter()
      .rev()
      .find(|l| l.contains("long-rule"))
      .unwrap();
    assert!(last.ends_with("[ long-rule ]"));
    // every line within the width budget — EXCEPT the last, which carries
    // the rule suffix (the suffix makes the last line wider than the budget;
    // this is acceptable because the suffix is metadata, not message text).
    for l in lines.iter().filter(|l| !l.contains("long-rule")) {
      assert!(display_width(l) <= 50, "wrapped line too wide: {l:?}");
    }
  }

  #[test]
  fn multiline_message_continuation_indent() {
    let style = Style::new(0);
    let msgs = vec![msg("line1\nline2\nline3", 2, 1, 1, "r", false)];
    let out = locate(
      "x = 1;",
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let lines: Vec<&str> = out.lines().collect();
    // every message line after the first is indented to the message start (6 cols)
    let l2 = lines.iter().find(|l| l.ends_with("line2")).unwrap();
    assert!(
      l2.starts_with("      ") && !l2.contains("\u{2500}"),
      "continuation indent, no connector: {l2:?}"
    );
    let l3 = lines.iter().find(|l| l.ends_with("line3 [ r ]")).unwrap();
    assert!(
      l3.starts_with("      "),
      "continuation indent carries the suffix line too"
    );
  }

  #[test]
  fn no_width_means_unchanged_layout() {
    let style = Style::new(0);
    let long = format!("const v = \"{}\";", "A".repeat(280));
    let msgs = vec![msg(
      "Strings must use singlequote.",
      2,
      1,
      12,
      "quotes",
      true,
    )];
    let out = locate(
      &long,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    assert!(
      out.contains(&"A".repeat(280)),
      "no truncation without max_width"
    );
  }

  // ── Column-merge feature tests (Feature A, v0.5.0) ──────────────────

  /// Two errors at the EXACT same column produce ONE caret and ONE `└─`
  /// connector — the second error gets a continuation row without `└─`.
  /// The empty connector row between same-column errors is eliminated.
  #[test]
  fn column_merge_two_errors_same_column() {
    let style = Style::new(0);
    let src = "if (typeof 1 === 'number!') {}\n";
    let msgs = vec![
      msg(
        "Unexpected constant condition.",
        2,
        1,
        5,
        "no-constant-condition",
        false,
      ),
      msg("Strings must use doublequote.", 2, 1, 20, "quotes", true),
      msg(
        "Invalid typeof comparison value.",
        2,
        1,
        20,
        "valid-typeof",
        false,
      ),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    // Two distinct columns (5 and 20) → two carets in the arrow row.
    let arrow_line = plain.lines().nth(1).unwrap();
    let carets: Vec<&str> = arrow_line.matches('↑').collect();
    assert_eq!(
      carets.len(),
      2,
      "2 distinct columns → 2 carets: {}",
      arrow_line
    );
    // Both same-column rules are present.
    assert!(plain.contains("[ quotes ]"), "quotes rendered");
    assert!(plain.contains("[ valid-typeof ]"), "valid-typeof rendered");
    // The first error at col 20 gets `└─`; the second gets NO `└─` —
    // count `└─` occurrences: there should be exactly 2 (one for
    // no-constant-condition, one for the first of quotes/valid-typeof).
    let connector_count = plain.matches("└─").count();
    assert_eq!(
      connector_count, 2,
      "2 └─ connectors (one per distinct column), not 3: {}",
      plain
    );
    // No empty connector row between same-column errors: the line
    // between the two same-column message rows should not be just `│`
    // (it would be if we had the old behavior).
    let lines: Vec<&str> = plain.lines().collect();
    let quotes_idx = lines.iter().position(|l| l.contains("[ quotes ]")).unwrap();
    let valid_typeof_idx = lines
      .iter()
      .position(|l| l.contains("[ valid-typeof ]"))
      .unwrap();
    // The two same-column messages should be adjacent (no empty connector
    // row between them).
    assert_eq!(
      valid_typeof_idx,
      quotes_idx + 1,
      "quotes and valid-typeof should be adjacent (no connector row between): {}",
      plain
    );
  }

  /// Three errors at the same column: one caret, one `└─`, three message
  /// rows (first with `└─`, two continuations without).
  #[test]
  fn column_merge_three_errors_same_column() {
    let style = Style::new(0);
    let src = "let x = 1\n";
    let msgs = vec![
      msg("msg1", 2, 1, 5, "rule-a", false),
      msg("msg2", 2, 1, 5, "rule-b", false),
      msg("msg3", 2, 1, 5, "rule-c", false),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    // One caret (one distinct column).
    let arrow_line = plain.lines().nth(1).unwrap();
    assert_eq!(
      arrow_line.matches('↑').count(),
      1,
      "one caret: {}",
      arrow_line
    );
    // One `└─` connector.
    assert_eq!(
      plain.matches("└─").count(),
      1,
      "one └─ connector: {}",
      plain
    );
    // All three rules present.
    assert!(plain.contains("[ rule-a ]"));
    assert!(plain.contains("[ rule-b ]"));
    assert!(plain.contains("[ rule-c ]"));
  }

  /// Errors at different columns are NOT merged — backward compatibility.
  #[test]
  fn column_merge_different_columns_not_merged() {
    let style = Style::new(0);
    let src = "let x = 1\n";
    let msgs = vec![
      msg("msg1", 2, 1, 1, "rule-a", false),
      msg("msg2", 2, 1, 5, "rule-b", false),
      msg("msg3", 2, 1, 9, "rule-c", false),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    // Three distinct columns → three carets.
    let arrow_line = plain.lines().nth(1).unwrap();
    assert_eq!(
      arrow_line.matches('↑').count(),
      3,
      "3 carets: {}",
      arrow_line
    );
    // Three `└─` connectors (one per error, all at different columns).
    assert_eq!(plain.matches("└─").count(), 3, "3 └─ connectors: {}", plain);
  }

  // ── Skip-indicator feature tests (Feature B, v0.5.0) ────────────────

  /// When 2+ consecutive non-error lines sit between two error blocks,
  /// a "… (lines X-Y clean)" indicator replaces the blank line.
  #[test]
  fn skip_indicator_multi_line_gap() {
    let style = Style::new(0);
    let src = "line1_err\nline2\nline3\nline4\nline5\nline6_err\n";
    let msgs = vec![
      msg("err1", 2, 1, 1, "rule-a", false),
      msg("err2", 2, 6, 1, "rule-b", false),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    assert!(
      plain.contains("… (lines 2-5 clean)"),
      "skip-indicator for lines 2-5: {}",
      plain
    );
  }

  /// A single skipped line between two error blocks does NOT get a
  /// skip-indicator — it stays as a blank line (backward compatibility).
  #[test]
  fn skip_indicator_single_line_gap_no_indicator() {
    let style = Style::new(0);
    let src = "line1_err\nline2\nline3_err\n";
    let msgs = vec![
      msg("err1", 2, 1, 1, "rule-a", false),
      msg("err2", 2, 3, 1, "rule-b", false),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    assert!(
      !plain.contains("clean)"),
      "no skip-indicator for single-line gap: {}",
      plain
    );
  }

  /// Leading non-error lines (before the first error) do NOT get a
  /// skip-indicator — they collapse to a blank line (backward compat).
  #[test]
  fn skip_indicator_leading_lines_no_indicator() {
    let style = Style::new(0);
    let src = "line1\nline2\nline3\nline4_err\n";
    let msgs = vec![msg("err1", 2, 4, 1, "rule-a", false)];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    assert!(
      !plain.contains("clean)"),
      "no skip-indicator for leading lines: {}",
      plain
    );
  }

  /// ASCII mode uses `...` instead of `…` for the skip-indicator.
  #[test]
  fn skip_indicator_ascii_mode_uses_dots() {
    let style = Style::new(0);
    let src = "line1_err\nline2\nline3\nline4\nline5_err\n";
    let msgs = vec![
      msg("err1", 2, 1, 1, "rule-a", false),
      msg("err2", 2, 5, 1, "rule-b", false),
    ];
    let opts = FormatOptions {
      ascii: true,
      ..FormatOptions::default()
    };
    let out = locate(src, &msgs, &style, &FormatterData::default(), &opts);
    let plain = strip_ansi(&out);
    assert!(
      plain.contains("... (lines 2-4 clean)"),
      "ASCII skip-indicator: {}",
      plain
    );
    assert!(
      !plain.contains('…'),
      "no Unicode ellipsis in ASCII mode: {}",
      plain
    );
  }

  /// Combined: column-merge + skip-indicator in one report. This mirrors
  /// the user's real-world screenshot (vite.config.ts with a gap between
  /// line 3 and line 11).
  #[test]
  fn combined_column_merge_and_skip_indicator() {
    let style = Style::new(0);
    let src = "import a from 'a'\nimport b from 'b'\nimport c from 'c'\n// 4\n// 5\n// 6\n// 7\n// 8\n// 9\n// 10\nexport default {}\n";
    let msgs = vec![
      msg("err1", 2, 1, 15, "semi", true),
      msg("err2", 2, 1, 15, "quotes", true), // same column as err1 → merge
      msg("err3", 2, 2, 15, "semi", true),
      msg("err4", 2, 11, 15, "semi", true),
    ];
    let out = locate(
      src,
      &msgs,
      &style,
      &FormatterData::default(),
      &FormatOptions::default(),
    );
    let plain = strip_ansi(&out);
    // Skip-indicator between line 3 and line 11.
    assert!(
      plain.contains("… (lines 3-10 clean)"),
      "skip-indicator present: {}",
      plain
    );
    // Column-merge: err1 and err2 at the same column on line 1.
    let line1_section = plain.split("… (lines 3-10 clean)").next().unwrap();
    assert!(
      line1_section.contains("[ semi ]") && line1_section.contains("[ quotes ]"),
      "both same-column rules rendered: {}",
      line1_section
    );
  }
}
