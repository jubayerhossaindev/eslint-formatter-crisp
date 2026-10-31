//! Core formatting pipeline: sort, per-file blocks, summary block.
//!
//! Behavioral contract verified against the Phase 1 REA report (§5, §6):
//! sort order, header layout, divider widths, severity icons, summary
//! statistics, empty-result handling, and `output`-over-`source` preference.

use crate::engine::locator::{locate, message_rows};
use crate::engine::model::{FormatterData, LintMessage, LintResult};
use crate::engine::plural::plural;
use crate::engine::style::{Style, ThemeOverride};
use crate::engine::width::display_width;

#[derive(Debug, Clone)]
pub struct FormatOptions {
  /// 0 = plain, 1 = 16-color, 2 = 256-color, 3 = 24-bit.
  pub color_level: u8,
  /// Working directory used to relativize file paths (ESLint cwd).
  pub base_dir: String,
  /// Emit OSC 8 terminal hyperlinks (file paths + rule ids).
  pub links: bool,
  /// Pure-ASCII decoration mode (`CRISP_ASCII=1`). Replaces the Unicode
  /// glyphs used for chrome (─ ↑ │ └─ ✖ ⚠ 🔥) with ASCII equivalents so
  /// the output renders correctly on terminals whose encoding cannot
  /// display them (e.g. code page 936 consoles, where the UTF-8 bytes are
  /// decoded as GBK and the dividers appear as CJK mojibake). Widths are
  /// preserved (1 column per glyph, 2 for `└─`/`\-`), so indicator columns
  /// and divider lengths stay aligned. Code content is not transliterated.
  pub ascii: bool,
  /// Terminal width in columns (`COLUMNS` env / TTY size, detected in JS).
  /// When set, source excerpts get an ellipsized window that always keeps
  /// every caret visible, and long messages word-wrap with continuation
  /// indent. `None` (piped output without `COLUMNS`, or `CRISP_FULL_LINES=1`)
  /// keeps the unlimited-width historical layout byte-for-byte.
  pub max_width: Option<usize>,
  /// Experimental: render the first fix suggestion under each message and
  /// mark deprecated rules (`CRISP_SUGGESTIONS=1`, v0.3.0+). Off by default.
  pub suggestions: bool,
  /// Experimental color overrides (`CRISP_THEME` / `MO_THEME`, v0.3.0+).
  pub theme: ThemeOverride,
  /// Display cap (audit roadmap ⑥, DoS guard): render at most this many
  /// messages across the WHOLE report, in render order (fill file 1
  /// completely, then file 2, ...). Files left with zero rendered messages
  /// are excluded from the display entirely. When the cap hides anything,
  /// ONE dim truncation notice line is appended after the last rendered
  /// file block. Every total (per-file headers, summary errors / warnings /
  /// auto-fixables) is still computed from ALL messages — the report never
  /// lies about totals. `None` = unlimited.
  ///
  /// The napi layer maps the JS `max_messages` option onto this field:
  /// absent -> `Some(DEFAULT_MAX_MESSAGES)`, `0` -> `None` (unlimited).
  pub max_messages: Option<usize>,
}

/// Default whole-report message cap applied by the napi layer when the JS
/// adapter passes no `max_messages` option (audit roadmap ⑥: bounds the
/// render cost of pathological runs — 100k messages measured ~5s / 4.2MB).
/// The adapter can raise or lift the cap via `CRISP_MAX_MESSAGES`
/// (`0` = unlimited); the engine itself treats `None` as unlimited.
pub const DEFAULT_MAX_MESSAGES: usize = 5000;

impl Default for FormatOptions {
  fn default() -> Self {
    FormatOptions {
      color_level: 1,
      base_dir: ".".to_string(),
      links: false,
      ascii: false,
      max_width: None,
      suggestions: false,
      theme: ThemeOverride::default(),
      max_messages: None,
    }
  }
}

/// The single truncation notice appended when the message cap hid any
/// messages (audit roadmap ⑥). Numbers are formatted the way the report
/// formats every other count (plain decimal, pluralized); the prefix is the
/// report's ellipsis glyph `…` in unicode mode and plain `...` in ASCII
/// mode. The env-var hint is informational only — env parsing stays in the
/// JS adapter (the engine never reads the environment).
fn truncation_notice(dropped: usize, ascii: bool) -> String {
  let prefix = if ascii { "..." } else { "…" };
  format!(
    "{prefix} {dropped} more {} not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)",
    plural("message", dropped as u32)
  )
}

/// Map decoration glyphs to ASCII-safe equivalents. Order matters: the
/// two-character `└─` connector is replaced before the single `─` rule so
/// it does not degrade into `└-`. All replacements preserve display width.
/// v0.5.3 UI/UX: `✖` → `X` (uppercase) and `⚠` → `!` for better visual
/// weight in ASCII-only terminals (Windows cmd, CI logs) — uppercase X is
/// more scannable than lowercase x which blends into code text.
fn ascii_glyphs(s: String) -> String {
  s.replace("└─", "\\-")
    .replace("├─", "+-")
    .replace('─', "-")
    .replace('↑', "^")
    .replace('│', "|")
    .replace('✖', "X")
    .replace('⚠', "!")
    .replace('🔥', "*")
    .replace('💡', "?") // v0.5.3: hint bulb → ? in ASCII mode
}

/// Split a path into (formatted-dirname, basename) the way the reference
/// does: dirname always ends with a separator ('./' when there is none).
fn split_path(relative: &str) -> (String, String) {
  let normalized = relative.replace('\\', "/");
  match normalized.rfind('/') {
    Some(idx) => (
      normalized[..=idx].to_string(),
      normalized[idx + 1..].to_string(),
    ),
    None => ("./".to_string(), normalized),
  }
}

/// POSIX-ish `path.relative(base, target)` for absolute paths; falls back to
/// the raw target when either path is not absolute. Deterministic (byte-wise)
/// ordering is used later for ties — a documented improvement over the
/// reference's locale-dependent `localeCompare`.
fn relative_path(base_dir: &str, target: &str) -> String {
  let norm = |p: &str| -> Vec<String> {
    p.replace('\\', "/")
      .split('/')
      .filter(|c| !c.is_empty() && *c != ".")
      .map(|c| {
        // Canonicalize Windows drive components so `c:` and `C:` compare
        // equal during the common-prefix walk (drives are case-insensitive).
        if c.len() == 2 && c.ends_with(':') {
          c.to_ascii_uppercase()
        } else {
          c.to_string()
        }
      })
      .collect()
  };
  let is_abs =
    |p: &str| p.starts_with('/') || p.starts_with('\\') || (p.as_bytes().get(1) == Some(&b':'));
  let drive = |p: &str| -> Option<u8> {
    let b = p.as_bytes();
    if b.len() >= 2 && b[1] == b':' {
      Some(b[0].to_ascii_uppercase())
    } else {
      None
    }
  };
  // Node's path.relative resolves a relative `base` against process.cwd()
  // before computing; without an absolute base there is nothing to
  // relativize against, so return the target unchanged instead of
  // producing a path that would not resolve.
  if !is_abs(base_dir) {
    return target.to_string();
  }
  if !is_abs(target) {
    return target.to_string();
  }
  // Windows: across drive letters Node returns the absolute target
  // (there is no relative route between C:\ and D:\).
  if let (Some(bd), Some(td)) = (drive(base_dir), drive(target))
    && bd != td
  {
    return target.to_string();
  }
  let base = norm(base_dir);
  let tgt = norm(target);
  let mut k = 0usize;
  while k < base.len() && k < tgt.len() && base[k] == tgt[k] {
    k += 1;
  }
  let ups = base.len() - k;
  let mut parts: Vec<String> = Vec::new();
  for _ in 0..ups {
    parts.push("..".to_string());
  }
  parts.extend(tgt[k..].iter().cloned());
  if parts.is_empty() {
    ".".to_string()
  } else {
    parts.join("/")
  }
}

/// Bold a quoted identifier span inside a message (shared with the locator).
use crate::engine::locator::bold_message;

/// Build a spec-correct `file://` URL for OSC-8 file links.
///
/// The historical `format!("file://{}", path)` broke on Windows
/// (`file://C:\Users\...` — wrong slash count AND backslashes), and did not
/// encode spaces or non-ASCII bytes (audit finding, roadmap Phase 3).
/// Rules implemented here:
///   - backslashes become forward slashes,
///   - a Windows drive path gets the third slash (`file:///C:/...`),
///   - UNC paths keep the two-slash authority form (`file://server/share`),
///   - every byte outside the unreserved set (`A-Za-z0-9-._~`) plus `/` and
///     `:` (drive separator) is percent-encoded as UTF-8.
pub fn to_file_url(path: &str) -> String {
  let mut normalized = path.replace('\\', "/");
  let mut out = String::with_capacity(path.len() + 8);
  out.push_str("file://");
  if normalized.starts_with("//") {
    // UNC: file://server/share — the authority form is already provided by
    // the "file://" prefix, so drop the leading slashes from the path body.
    normalized = normalized.trim_start_matches('/').to_string();
  } else if !normalized.starts_with('/') {
    out.push('/');
  }
  for (i, seg) in normalized.split('/').enumerate() {
    if i > 0 {
      out.push('/');
    }
    for b in seg.as_bytes() {
      if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b':') {
        out.push(*b as char);
      } else {
        out.push_str(&format!("%{:02X}", b));
      }
    }
  }
  out
}

pub fn format_report(
  results: &mut [LintResult],
  data: &FormatterData,
  opts: &FormatOptions,
) -> String {
  let style = Style::with_theme(opts.color_level, &opts.theme);

  // Sort: errorCount DESC, warningCount DESC, path ASC (byte order).
  results.sort_by(|a, b| {
    b.error_count
      .cmp(&a.error_count)
      .then(b.warning_count.cmp(&a.warning_count))
      .then(a.file_path.cmp(&b.file_path))
  });

  let mut total_errors: u32 = 0;
  let mut total_warnings: u32 = 0;
  let mut auto_fixable: usize = 0;
  let mut blocks: Vec<String> = Vec::new();

  // Message cap (audit roadmap ⑥): remaining display budget, consumed in
  // render order — files are sorted above, so the most-flagged files keep
  // their messages first (fill file 1 completely, then file 2, ...).
  // `None` renders everything.
  let mut remaining: Option<usize> = opts.max_messages;
  let mut dropped: usize = 0;

  // v0.5.3 UI/UX: total file count for the `[1/N]` index prefix. Counted
  // from the (already-sorted) results BEFORE the loop so each iteration
  // knows its position. Files with zero messages are skipped inside the
  // loop, so the index reflects rendered files only.
  let total_files = results
    .iter()
    .filter(|r| r.error_count + r.warning_count > 0)
    .count();
  let mut rendered_file_idx: usize = 0;

  for result in results.iter() {
    if result.error_count + result.warning_count == 0 {
      continue; // clean files are skipped entirely (verified s3-era behavior)
    }
    let file_idx = rendered_file_idx;
    rendered_file_idx += 1;
    total_errors += result.error_count;
    total_warnings += result.warning_count;
    // Counted over ALL messages of every non-clean file, including any the
    // cap hides below — the fixable total stays honest ("never lie").
    auto_fixable += result.messages.iter().filter(|m| m.fixable).count();

    // Budget for this file: render as many of its messages as the cap
    // still allows, then spill into the following files.
    let visible: &[LintMessage] = match remaining.as_mut() {
      None => &result.messages[..],
      Some(budget) => {
        let take = (*budget).min(result.messages.len());
        *budget -= take;
        dropped += result.messages.len() - take;
        &result.messages[..take]
      }
    };
    if visible.is_empty() && !result.messages.is_empty() {
      // Cap exhausted before this file: exclude it from the display
      // entirely (no header for a file with zero rendered messages).
      // Its messages are already counted in the totals above.
      continue;
    }

    // ---- Header block -------------------------------------------------
    let relative = relative_path(&opts.base_dir, &result.file_path);
    let (dir, base) = split_path(&relative);
    let plain_path = format!("{}{}", dir, base);
    // v0.5.3 UI/UX: file-path basename is now bright-white (not dim-bold),
    // so the file name pops at a glance during scroll. Directory stays dim.
    let styled_path = format!("{}{}", style.dim(&dir), style.white_bright(&base));
    let linked_path = style.link(
      &styled_path,
      Some(&to_file_url(&result.file_path)),
      opts.links,
    );

    // v0.5.3 UI/UX: file index in header (e.g. `[1/3]`) when there are
    // multiple files. Single-file runs keep the historical header shape.
    let file_index_prefix = if total_files > 1 {
      format!(
        "{} ",
        style.dim(&format!("[{}/{}]", file_idx + 1, total_files))
      )
    } else {
      String::new()
    };

    let error_summary = if result.error_count > 0 {
      style.bold(&format!(
        "{} {}",
        result.error_count,
        plural("error", result.error_count)
      ))
    } else {
      String::new()
    };
    let warning_summary = if result.warning_count > 0 {
      style.bold(&format!(
        "{} {}",
        result.warning_count,
        plural("warning", result.warning_count)
      ))
    } else {
      String::new()
    };

    // [errorSummary, warningSummary].join(' ') semantics: an empty error
    // summary leaves a leading space, an empty warning summary leaves a
    // trailing space (' 🔥 5 errors 🔥' vs ' 🔥 2 warnings🔥' — verified
    // against the s1/s2/s3 captures).
    let joined = format!("{} {}", error_summary, warning_summary);
    let separator = if result.error_count > 0 { " " } else { "" };

    // In ASCII mode the fire glyphs are swapped BEFORE the divider width
    // is computed (🔥 is 2 columns, `*` is 1) so the rule above the header
    // still exactly matches the header's visible width.
    let fire = if opts.ascii { "*" } else { "🔥" };

    let header_line = format!(
      "{} {} {}{}{}{}",
      file_index_prefix, linked_path, fire, separator, joined, fire
    );
    let header_plain = format!(
      "{} {} {}{}{}{}",
      file_index_prefix.trim_end(),
      plain_path,
      fire,
      separator,
      joined,
      fire
    );

    // v0.5.3 UI/UX: visual hierarchy — file-header dividers are now BOLD
    // (bright, prominent) so the eye lands on file sections during scroll.
    // The summary dividers stay as gradient (fruit/cristal) but the file
    // dividers use a bright em-dash run styled with bold.
    let divider = if style.level > 0 {
      style.bold(&"─".repeat(display_width(&header_plain)))
    } else {
      "─".repeat(display_width(&header_plain))
    };
    blocks.push(format!("{}\n{}\n{}", divider, header_line, divider));

    // ---- Per-message section ------------------------------------------
    // Normalize: prefer `output` over `source` (verified --fix-dry-run s8).
    let source_text: Option<&str> = result.output.as_deref().or(result.source.as_deref());

    let has_real_locations = visible.iter().any(|m| m.line != 0 || m.column != 0);

    // Rendering helper for messages that cannot be attached to a code
    // frame. IMPROVEMENT over the reference, which silently drops these
    // (Phase 1 report §8.2: messages vanish when `source` is absent, and
    // when the reported line lies beyond the trimmed excerpt — both
    // confirmed live in Phase 2 M4 with an ESLint 9 parse error).
    // (Phase 4 refactor: the row rendering itself lives in the shared
    // `message_rows` helper in locator.rs so fallback rows and in-frame
    // message rows can never drift apart.)
    let push_fallback_rows = |blocks: &mut Vec<String>, messages: &[LintMessage]| {
      for m in messages {
        let rows = message_rows(m, 0, &style, data, opts);
        // Fallback rows keep the historical `└─` connector before the icon;
        // message_rows starts at the icon itself (own = 0).
        if let Some(first) = rows.first() {
          blocks.push(format!("{} {}", style.dim("└─"), first));
        }
        for row in rows.iter().skip(1) {
          blocks.push(row.clone());
        }
      }
    };

    // Parsing-error short-circuit (verified s11/s12): exactly one message
    // at line 0 / col 0 renders as a single red ✖ row. With a message cap
    // only the VISIBLE slice participates (a hidden message can never
    // trigger or satisfy the short-circuit).
    if visible.len() == 1 && visible[0].line == 0 && visible[0].column == 0 {
      // v0.5.3 UI/UX: fatal parse error gets a clearer rendering with a
      // `└─` connector (matching the in-frame message style) and a dim
      // hint line so the user knows ESLint could not parse the file.
      blocks.push(format!("{} {}", style.dim("└─"), style.red("✖"),));
      blocks.push(format!(
        "  {} {}",
        bold_message(&visible[0].message, &style),
        style.dim("— ESLint could not parse this file. Check for syntax errors.")
      ));
    } else if let Some(src) = source_text {
      if has_real_locations {
        let frame = locate(src, visible, &style, data, opts);
        if frame.is_empty() {
          // Every message fell outside the frame (e.g. a parse
          // error reported beyond the trimmed excerpt). Render the
          // messages instead of dropping them.
          push_fallback_rows(&mut blocks, visible);
        } else {
          blocks.push(frame);
        }
      } else {
        // Messages without any location but with source: render a
        // simple list (defensive; ESLint always provides locations).
        push_fallback_rows(&mut blocks, visible);
      }
    } else {
      push_fallback_rows(&mut blocks, visible);
    }
  }

  // ---- Truncation notice (message cap, audit roadmap ⑥) ----------------
  // Exactly ONE clearly-marked line after the last rendered file block;
  // the summary below still reports the honest full-count totals.
  if dropped > 0 {
    blocks.push(style.dim(&truncation_notice(dropped, opts.ascii)));
  }

  // ---- Summary block ----------------------------------------------------
  // v0.5.2 UI/UX improvement: zero-count summary rows are skipped, so the
  // summary stays compact when one of the three counts is zero. The divider
  // width is computed from the ACTUAL rendered lines (not the fixed 43-col
  // `fix_plain`), so it visually matches the content. When all three are
  // zero the report is empty (handled above).
  if total_errors + total_warnings == 0 {
    return String::new();
  }

  // Build the (line_plain, line_styled) pair for each non-zero count.
  // `auto_fixable` is suppressed when both errors and warnings are zero
  // (already handled above) OR when auto_fixable is 0 — but if there are
  // errors+warnings and zero auto-fixables, we still show it so the user
  // sees "--fix won't help here" (informational, not noise). The original
  // behavior showed all three always; v0.5.2 only suppresses when the
  // corresponding count is zero AND the other two are non-zero (so a pure-
  // warning run still shows `0 errors` if you really want it… no, we
  // suppress zero rows uniformly — cleaner output wins).
  let mut summary_pairs: Vec<(String, String)> = Vec::new();
  if total_errors > 0 {
    let plain = format!(" {} {}", total_errors, plural("error", total_errors));
    let styled = if style.level == 0 {
      plain.clone()
    } else {
      format!(
        "\x1b[91m \x1b[1m{}\x1b[22m {}\x1b[39m",
        total_errors,
        plural("error", total_errors)
      )
    };
    summary_pairs.push((plain, styled));
  }
  if total_warnings > 0 {
    let plain = format!(" {} {}", total_warnings, plural("warning", total_warnings));
    let styled = if style.level == 0 {
      plain.clone()
    } else {
      format!(
        "\x1b[93m \x1b[1m{}\x1b[22m {}\x1b[39m",
        total_warnings,
        plural("warning", total_warnings)
      )
    };
    summary_pairs.push((plain, styled));
  }
  if auto_fixable > 0 {
    let plain = format!(
      " {} {} ( with the `--fix` flag )",
      auto_fixable,
      plural("auto-fixable", auto_fixable as u32)
    );
    let styled = if style.level == 0 {
      plain.clone()
    } else {
      format!(
        "\x1b[92m \x1b[1m{}\x1b[22m {} ( with the `--fix` flag )\x1b[39m",
        auto_fixable,
        plural("auto-fixable", auto_fixable as u32)
      )
    };
    summary_pairs.push((plain, styled));
  }

  // Divider width: the joined single-line summary's visible width + 1.
  // (Was: widest individual row + 1; now the joined line is wider because
  // all three counts share one line, so we measure the actual joined line.)
  let summary_plain_joined = summary_pairs
    .iter()
    .map(|(plain, _)| plain.as_str())
    .collect::<Vec<_>>()
    .join("  ");
  let max_len = display_width(&summary_plain_joined) + 1;

  // Single-line collapse: when there are 2-3 summary rows, join them on one
  // line separated by `  ` (two spaces) for a compact footer. When only one
  // row, render it alone. This saves vertical space without losing info.
  let summary_line_styled = summary_pairs
    .iter()
    .map(|(_, styled)| styled.as_str())
    .collect::<Vec<_>>()
    .join("  ");

  // File-block separator: v0.5.2 UI/UX — between consecutive file blocks,
  // emit a blank line so the reader can visually separate one file's
  // code frame from the next file's header. The first file has no leading
  // separator (the report already opens with `\n`). Trailing blank lines
  // are absorbed by the format! join below; a single `\n` between blocks
  // is what gives the visual gap.
  //
  // We mark each block as "needs leading separator" when it's not the
  // first file block. The join uses `\n\n` to insert the blank line.
  // (Implementation: push a marker empty string between blocks, then join
  //  with `\n` — but the per-file content already ends without a trailing
  //  newline, so a literal `\n\n` join is the cleanest approach.)
  let joined_blocks = {
    let mut sep_blocks: Vec<String> = Vec::with_capacity(blocks.len() * 2);
    for (i, b) in blocks.iter().enumerate() {
      if i > 0 {
        sep_blocks.push(String::new()); // blank line between file blocks
      }
      sep_blocks.push(b.clone());
    }
    sep_blocks.join("\n")
  };

  // v0.5.3 UI/UX: next-step hint — when there are auto-fixable issues,
  // append a dim one-line hint below the summary telling the user how to
  // fix them. Skipped when auto_fixable == 0 (no actionable advice).
  let next_step = if auto_fixable > 0 {
    let hint = format!(
      " 💡 Run `eslint --fix` to auto-fix {} {}.",
      auto_fixable,
      plural("issue", auto_fixable as u32)
    );
    style.dim(&hint)
  } else {
    String::new()
  };

  // v0.5.3 UI/UX: file breakdown — when 3+ files have problems, show a
  // compact "top files by error count" mini-table below the summary so the
  // user can see at a glance which file to prioritize. Only the top 5 files
  // are listed; ties are broken by file_path (already sorted). Skipped for
  // 1-2 files (the headers above already convey that clearly).
  let file_breakdown = if total_files >= 3 {
    let mut sorted: Vec<&LintResult> = results
      .iter()
      .filter(|r| r.error_count + r.warning_count > 0)
      .collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.error_count));
    let top = sorted.iter().take(5);
    let mut lines: Vec<String> = Vec::new();
    lines.push(style.dim(" Top files by error count:"));
    for r in top {
      let rel = relative_path(&opts.base_dir, &r.file_path);
      let (_dir, base) = split_path(&rel);
      lines.push(format!(
        "   {} {} {}",
        style.white_bright(&base),
        style.dim(&format!(
          "({} {}, {} {})",
          r.error_count,
          plural("error", r.error_count),
          r.warning_count,
          plural("warning", r.warning_count)
        )),
        "",
      ));
    }
    lines.join("\n")
  } else {
    String::new()
  };

  let out = format!(
    "\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
    joined_blocks,
    style.fruit(&"─".repeat(max_len)),
    summary_line_styled,
    style.cristal(&"─".repeat(max_len)),
    next_step,
    file_breakdown,
    "",
  );

  if opts.ascii { ascii_glyphs(out) } else { out }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn m(message: &str, sev: i32, line: u32, col: u32, rule: &str, fixable: bool) -> LintMessage {
    LintMessage {
      message: message.into(),
      severity: Some(sev),
      line,
      column: col,
      rule_id: Some(rule.into()),
      fatal: false,
      fixable,
      suggestion: None,
    }
  }

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

  fn opts() -> FormatOptions {
    FormatOptions {
      color_level: 1,
      base_dir: "/work".into(),
      links: false,
      ascii: false,
      max_width: None,
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    }
  }

  /// The s1 fixture shape: 5 errors / 2 warnings / 4 fixable on one file.
  fn s1_results() -> Vec<LintResult> {
    vec![LintResult {
      file_path: "/work/broken.js".into(),
      source: Some("const unusedVar = 42;\n\n".into()),
      output: None,
      error_count: 5,
      warning_count: 2,
      messages: vec![
        m("a", 1, 1, 7, "no-unused-vars", false),
        m("b", 2, 1, 3, "semi", true),
        m("c", 2, 1, 3, "quotes", true),
        m("d", 1, 1, 5, "no-console", false),
        m("e", 2, 1, 3, "semi", true),
        m("f", 2, 1, 3, "semi", true),
        m("g", 2, 1, 3, "no-undef", false),
      ],
    }]
  }

  #[test]
  fn golden_s1_divider_and_summary_widths() {
    let mut results = s1_results();
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    let plain = strip_ansi(&out);
    // Header divider: visible width 37 (Phase 1 report §6.1 golden value)
    assert!(plain.contains(&"─".repeat(37)), "divider 37 expected");
    assert!(plain.contains(" 5 errors"), "error line");
    assert!(plain.contains(" 2 warnings"), "warning line");
    assert!(plain.contains(" 4 auto-fixables ( with the `--fix` flag )"));
    // Summary divider: 43 (Phase 1 report §6.4 golden value). Level-1
    // gradients emit one SGR per dash, so count on the stripped text.
    assert!(plain.contains(&"─".repeat(43)), "summary divider 43");
  }

  #[test]
  fn summary_codes_level1() {
    let mut results = s1_results();
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    assert!(out.contains("\x1b[91m \x1b[1m5\x1b[22m errors\x1b[39m"));
    assert!(out.contains("\x1b[93m \x1b[1m2\x1b[22m warnings\x1b[39m"));
    assert!(out.contains("\x1b[92m \x1b[1m4\x1b[22m auto-fixables"));
    // fruit start + cristal start
    assert!(out.contains("\x1b[91m─"));
    assert!(out.contains("\x1b[97m─"));
  }

  #[test]
  fn empty_input_empty_output() {
    let mut results: Vec<LintResult> = vec![];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    assert_eq!(out, "");
    let mut clean = vec![LintResult {
      file_path: "/work/ok.js".into(),
      source: None,
      output: None,
      error_count: 0,
      warning_count: 0,
      messages: vec![],
    }];
    assert_eq!(
      format_report(&mut clean, &FormatterData::default(), &opts()),
      ""
    );
  }

  #[test]
  fn relative_path_posix() {
    assert_eq!(relative_path("/work", "/work/broken.js"), "broken.js");
    assert_eq!(
      relative_path("/work", "/work/sub/broken.js"),
      "sub/broken.js"
    );
    assert_eq!(relative_path("/work", "/usr/lib/x.js"), "../usr/lib/x.js");
    assert_eq!(relative_path("/a/b/c", "/a/b/c"), ".");
    assert_eq!(relative_path("/", "/work/a.js"), "work/a.js");
  }

  #[test]
  fn relative_path_degenerate_inputs() {
    // Relative base: Node's path.relative cannot resolve either — return the
    // target unchanged instead of a path that would not resolve.
    assert_eq!(relative_path(".", "/work/a.js"), "/work/a.js");
    assert_eq!(relative_path("", "/work/a.js"), "/work/a.js");
    // Relative target passes through untouched.
    assert_eq!(relative_path("/work", "sub/a.js"), "sub/a.js");
  }

  #[test]
  fn relative_path_windows() {
    assert_eq!(
      relative_path("C:\\work", "C:\\work\\broken.js"),
      "broken.js"
    );
    assert_eq!(relative_path("C:\\work", "C:\\work\\sub\\a.js"), "sub/a.js");
    assert_eq!(
      relative_path("C:\\work", "C:\\other\\a.js"),
      "../other/a.js"
    );
    // Cross-drive: Node returns the absolute target.
    assert_eq!(
      relative_path("C:\\work", "D:\\other\\a.js"),
      "D:\\other\\a.js"
    );
    // Drive comparison is case-insensitive.
    assert_eq!(relative_path("c:\\work", "C:\\work\\a.js"), "a.js");
    // Mixed-environment input (POSIX base, Windows-style target) has no
    // Node `path.relative` equivalent on either platform; the walk still
    // produces a deterministic, resolvable-from-base path.
    assert_eq!(relative_path("/work", "C:\\work\\a.js"), "../C:/work/a.js");
  }

  #[test]
  fn sort_order_matches_s2() {
    let mut results = vec![
      LintResult {
        file_path: "/work/c.js".into(),
        source: None,
        output: None,
        error_count: 0,
        warning_count: 1,
        messages: vec![m("w", 1, 1, 1, "no-unused-vars", false)],
      },
      LintResult {
        file_path: "/work/b.js".into(),
        source: None,
        output: None,
        error_count: 2,
        warning_count: 0,
        messages: vec![
          m("e1", 2, 1, 1, "semi", true),
          m("e2", 2, 1, 5, "semi", true),
        ],
      },
      LintResult {
        file_path: "/work/a.js".into(),
        source: None,
        output: None,
        error_count: 2,
        warning_count: 1,
        messages: vec![
          m("e1", 2, 1, 1, "semi", true),
          m("e2", 2, 1, 5, "semi", true),
          m("w", 1, 2, 1, "no-console", false),
        ],
      },
    ];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    let plain = strip_ansi(&out);
    let a = plain.find(" ./a.js ").unwrap();
    let b = plain.find(" ./b.js ").unwrap();
    let c = plain.find(" ./c.js ").unwrap();
    assert!(a < b && b < c, "expected a<b<c, got {plain:?}");
  }

  #[test]
  fn parse_error_short_circuit() {
    let mut results = vec![LintResult {
      file_path: "/work/parse-error.js".into(),
      source: Some("const broken = {".into()),
      output: None,
      error_count: 1,
      warning_count: 0,
      messages: vec![LintMessage {
        message: "Parsing error: Unexpected token".into(),
        severity: Some(2),
        line: 0,
        column: 0,
        rule_id: None,
        fatal: true,
        fixable: false,
        suggestion: None,
      }],
    }];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    let plain = strip_ansi(&out);
    // v0.5.3: parse error now renders as `└─ ✖` + `  Parsing error: ...` on
    // two lines (with a dim hint suffix). Both the connector and the
    // message text must be present.
    assert!(plain.contains("✖"), "✖ icon present: {plain:?}");
    assert!(
      plain.contains("Parsing error: Unexpected token"),
      "message present: {plain:?}"
    );
    assert!(
      plain.contains("ESLint could not parse this file"),
      "v0.5.3 hint present: {plain:?}"
    );
    assert!(!plain.contains(" 1: "), "no code frame expected");
  }

  /// ESLint 9 reports real parse errors with a line beyond the excerpt
  /// (verified live in M4: `line: 2` for a 1-line file). The reference
  /// silently drops the message; crisp renders it via the fallback list.
  #[test]
  fn parse_error_beyond_eof_renders_message() {
    let mut results = vec![LintResult {
      file_path: "/work/parse-error.js".into(),
      source: Some("const broken = {\n".into()),
      output: None,
      error_count: 1,
      warning_count: 0,
      messages: vec![LintMessage {
        message: "Parsing error: Unexpected token".into(),
        severity: Some(2),
        line: 2,
        column: 1,
        rule_id: None,
        fatal: true,
        fixable: false,
        suggestion: None,
      }],
    }];
    let o = FormatOptions {
      color_level: 0,
      base_dir: "/work".into(),
      links: false,
      ascii: false,
      max_width: None,
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    };
    let out = format_report(&mut results, &FormatterData::default(), &o);
    let plain = strip_ansi(&out);
    assert!(
      plain.contains("└─ ✖ Parsing error: Unexpected token"),
      "message must not be dropped: {plain:?}"
    );
  }

  #[test]
  fn output_preferred_over_source() {
    let mut results = vec![LintResult {
      file_path: "/work/fixed.js".into(),
      source: Some("let a = 1\n".into()),
      output: Some("let a = 1;\n".into()),
      error_count: 1,
      warning_count: 0,
      messages: vec![m(
        "Unexpected console statement.",
        1,
        1,
        1,
        "no-console",
        false,
      )],
    }];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    let plain = strip_ansi(&out);
    assert!(
      plain.contains("let a = 1;"),
      "fixed source shown: {plain:?}"
    );
    assert!(!plain.contains("let a = 1\n"), "original source hidden");
  }

  #[test]
  fn missing_source_renders_fallback_list() {
    let mut results = vec![LintResult {
      file_path: "/work/nosrc.js".into(),
      source: None,
      output: None,
      error_count: 1,
      warning_count: 1,
      messages: vec![
        m("First message.", 2, 1, 1, "semi", true),
        m("Second message.", 1, 2, 1, "no-console", false),
      ],
    }];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    let plain = strip_ansi(&out);
    assert!(
      plain.contains("└─ ✖ First message. [ semi ] ( auto-fixable )"),
      "{plain:?}"
    );
    assert!(
      plain.contains("└─ ⚠ Second message. [ no-console ]"),
      "{plain:?}"
    );
  }

  #[test]
  fn warning_only_file_zero_counts_shown() {
    let mut results = vec![LintResult {
      file_path: "/work/warn.js".into(),
      source: Some("let a = 1\n".into()),
      output: None,
      error_count: 0,
      warning_count: 1,
      messages: vec![m(
        "a is assigned a value but never used.",
        1,
        1,
        5,
        "no-unused-vars",
        false,
      )],
    }];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    let plain = strip_ansi(&out);
    assert!(plain.contains(" ./warn.js 🔥 1 warning🔥"), "{plain:?}");
    // v0.5.2: zero-count rows are skipped in the summary. A pure-warning
    // run no longer shows ` 0 errors` — the summary collapses to a single
    // ` 1 warning` line.
    assert!(
      !plain.contains(" 0 errors"),
      "zero-count suppressed: {plain:?}"
    );
    assert!(plain.contains(" 1 warning"), "{plain:?}");
  }

  #[test]
  fn level0_plain_output() {
    let mut results = s1_results();
    let o = FormatOptions {
      color_level: 0,
      base_dir: "/work".into(),
      links: false,
      ascii: false,
      max_width: None,
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    };
    let out = format_report(&mut results, &FormatterData::default(), &o);
    assert!(!out.contains('\x1b'), "no escapes at level 0");
    let plain = strip_ansi(&out);
    assert!(plain.contains(&"─".repeat(37)));
    assert!(plain.contains(&"─".repeat(43)));
  }

  /// The real s1 fixture: exact ESLint data captured via `eslint -f json`
  /// in Phase 2 M1 (columns verified against the Phase 1 mo captures).
  /// Asserts the full ANSI-stripped output matches the mo reference text.
  #[test]
  fn golden_s1_full_text_parity() {
    let src = "const unusedVar = 42;\nfunction greet(name) {\n    console.log(\"hello \" + name)\n    return name\n}\ngreet(undefinedThing)\n";
    let mut results = vec![LintResult {
      file_path: "/work/broken.js".into(),
      source: Some(src.into()),
      output: None,
      error_count: 5,
      warning_count: 2,
      messages: vec![
        m(
          "'unusedVar' is assigned a value but never used.",
          1,
          1,
          7,
          "no-unused-vars",
          false,
        ),
        m(
          "Unexpected console statement.",
          1,
          3,
          5,
          "no-console",
          false,
        ),
        m("Strings must use singlequote.", 2, 3, 17, "quotes", true),
        m("Missing semicolon.", 2, 3, 33, "semi", true),
        m("Missing semicolon.", 2, 4, 16, "semi", true),
        m(
          "'undefinedThing' is not defined.",
          2,
          6,
          7,
          "no-undef",
          false,
        ),
        m("Missing semicolon.", 2, 6, 22, "semi", true),
      ],
    }];
    let o = FormatOptions {
      color_level: 0,
      base_dir: "/work".into(),
      links: false,
      ascii: false,
      max_width: None,
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    };
    let out = format_report(&mut results, &FormatterData::default(), &o);
    let plain = strip_ansi(&out);
    // v0.5.2 UI/UX changes reflected in the golden text:
    //   - source lines no longer have a trailing space
    //   - file blocks are separated by a blank line
    //   - summary collapses to a single line, zero-count rows skipped
    //   - divider width adapts to actual content (no longer fixed 37/43)
    assert!(
      plain.contains(" 1: const unusedVar = 42;\n"),
      "no trailing space on source line: {plain:?}"
    );
    assert!(
      plain.contains(" 3:     console.log(\"hello \" + name)\n"),
      "no trailing space on line 3: {plain:?}"
    );
    assert!(
      plain.contains(" 4:     return name\n"),
      "no trailing space on line 4: {plain:?}"
    );
    assert!(
      plain.contains(" 6: greet(undefinedThing)\n"),
      "no trailing space on line 6: {plain:?}"
    );
    // Summary collapsed to one line; zero counts absent.
    assert!(
      plain.contains(" 5 errors   2 warnings   4 auto-fixables ( with the `--fix` flag )"),
      "single-line summary: {plain:?}"
    );
    // File-block separator: blank line between the last message of one
    // logical block and the next source line (verified: there are two
    // `\n\n` gaps inside the single file between non-adjacent error lines).
    assert!(
      plain.matches("\n\n").count() >= 2,
      "blank-line separators present: {plain:?}"
    );
  }

  #[test]
  fn split_path_reference_parity() {
    assert_eq!(split_path("broken.js"), ("./".into(), "broken.js".into()));
    assert_eq!(
      split_path("sub/dir/file.js"),
      ("sub/dir/".into(), "file.js".into())
    );
    assert_eq!(split_path("./x.js"), ("./".into(), "x.js".into()));
  }

  #[test]
  fn ascii_mode_output_is_pure_ascii() {
    let mut results = s1_results();
    let o = FormatOptions {
      color_level: 1,
      base_dir: "/work".into(),
      links: false,
      ascii: true,
      max_width: None,
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    };
    let out = format_report(&mut results, &FormatterData::default(), &o);
    assert!(
      out.is_ascii(),
      "ASCII mode must emit only ASCII bytes, got: {:?}",
      out.chars().filter(|c| !c.is_ascii()).collect::<Vec<_>>()
    );
    let plain = strip_ansi(&out);
    assert!(
      plain.contains("./broken.js * 5 errors 2 warnings*"),
      "{plain:?}"
    );
    assert!(
      plain.contains("\\- X b [ semi ] ( auto-fixable )"),
      "{plain:?}"
    );
    assert!(plain.contains("\\- ! a [ no-unused-vars ]"), "{plain:?}");
    assert!(plain.contains("^"), "{plain:?}");
    assert!(!plain.contains('─') && !plain.contains('↑') && !plain.contains('✖'));
  }

  #[test]
  fn ascii_mode_dividers_stay_aligned() {
    let mut results = s1_results();
    let o = FormatOptions {
      color_level: 0,
      base_dir: "/work".into(),
      links: false,
      ascii: true,
      max_width: None,
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    };
    let out = format_report(&mut results, &FormatterData::default(), &o);
    let lines: Vec<&str> = out.lines().collect();
    // header divider, header line, header divider are lines 1..=3 (line 0 empty)
    let d1 = lines[1].chars().filter(|c| *c == '-').count();
    let header_len = display_width(lines[2]);
    assert_eq!(
      d1, header_len,
      "divider length must equal header display width"
    );
    // v0.5.2: summary is now a single line (no longer 3 separate lines).
    // The two summary dividers now sandwich the single summary line directly.
    // Find the summary line (the one containing "auto-fixable" or "errors").
    let summary_line_idx = lines
      .iter()
      .rposition(|l| l.contains("errors") || l.contains("warnings"))
      .expect("summary line present");
    let summary_line = lines[summary_line_idx];
    let summary_divider_top = lines[summary_line_idx - 1];
    let summary_divider_bottom = lines[summary_line_idx + 1];
    let d_top = summary_divider_top.chars().filter(|c| *c == '-').count();
    let d_bot = summary_divider_bottom.chars().filter(|c| *c == '-').count();
    let summary_w = display_width(summary_line);
    assert!(
      d_top >= summary_w && d_top <= summary_w + 1,
      "top divider ~summary width (got {d_top}, summary {summary_w}): {summary_divider_top:?}"
    );
    assert!(
      d_bot >= summary_w && d_bot <= summary_w + 1,
      "bottom divider ~summary width (got {d_bot}, summary {summary_w}): {summary_divider_bottom:?}"
    );
  }

  #[test]
  fn unicode_glyphs_kept_by_default() {
    let mut results = s1_results();
    let o = opts();
    let out = format_report(&mut results, &FormatterData::default(), &o);
    let plain = strip_ansi(&out);
    assert!(plain.contains('─') && plain.contains('✖') && plain.contains('🔥'));
  }

  #[test]
  fn relative_path_variants() {
    assert_eq!(relative_path("/work", "/work/broken.js"), "broken.js");
    assert_eq!(relative_path("/work", "/work/sub/a.js"), "sub/a.js");
    assert_eq!(relative_path("/work/sub", "/work/a.js"), "../a.js");
    assert_eq!(relative_path("/work", "/other/a.js"), "../other/a.js");
    assert_eq!(
      relative_path("/work", "rel.js"),
      "rel.js",
      "non-absolute falls back"
    );
  }

  #[test]
  fn file_url_windows_unc_and_encoding() {
    // Windows drive: three slashes + forward slashes
    assert_eq!(
      to_file_url("C:\\Users\\dev\\app.tsx"),
      "file:///C:/Users/dev/app.tsx"
    );
    // UNC keeps the authority form
    assert_eq!(
      to_file_url("\\\\server\\share\\f.js"),
      "file://server/share/f.js"
    );
    // POSIX already absolute
    assert_eq!(to_file_url("/work/a b/x.js"), "file:///work/a%20b/x.js");
    // non-ASCII gets UTF-8 percent-encoding
    assert_eq!(
      to_file_url("/home/\u{09ac}/f.js"),
      "file:///home/%E0%A6%AC/f.js"
    );
  }

  #[test]
  fn max_width_truncates_source_and_wraps_message_end_to_end() {
    let mut results = vec![LintResult {
      file_path: "/work/long.js".into(),
      source: Some(format!("const v = \"{}\";\nlet b = 2;", "A".repeat(60))),
      output: None,
      error_count: 1,
      warning_count: 0,
      messages: vec![m(
        "This is a very long diagnostic message that keeps going and going until extremely wide.",
        2,
        1,
        12,
        "long-rule",
        false,
      )],
    }];
    let o = FormatOptions {
      color_level: 0,
      base_dir: "/work".into(),
      links: false,
      ascii: false,
      max_width: Some(60),
      suggestions: false,
      theme: Default::default(),
      max_messages: None,
    };
    let out = format_report(&mut results, &FormatterData::default(), &o);
    for line in out.lines() {
      assert!(display_width(line) <= 60, "line exceeds 60 cols: {line:?}");
    }
    assert!(
      out.contains('\u{2026}'),
      "source excerpt carries an ellipsis"
    );
    assert!(out.contains("long-rule"), "rule id still rendered");
    // caret row still present and aligned inside the excerpt
    assert!(out.lines().any(|l| l.contains('\u{2191}')));
  }

  #[test]
  fn no_max_width_keeps_full_source_line() {
    let mut results = vec![LintResult {
      file_path: "/work/long.js".into(),
      source: Some(format!("const v = \"{}\";", "A".repeat(280))),
      output: None,
      error_count: 1,
      warning_count: 0,
      messages: vec![m("Strings must use singlequote.", 2, 1, 12, "quotes", true)],
    }];
    let out = format_report(&mut results, &FormatterData::default(), &opts());
    assert!(
      out.contains(&"A".repeat(280)),
      "full source echoed without width"
    );
  }

  // ---- Message cap (audit roadmap ⑥) -------------------------------------

  const NOTICE_2: &str =
    "… 2 more messages not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)";
  const NOTICE_1: &str =
    "… 1 more message not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)";
  const NOTICE_3: &str =
    "… 3 more messages not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)";

  /// Two files, three error messages each (b2 fixable), no source: every
  /// message renders as exactly one fallback row, so rendered-message
  /// counting is a plain substring check.
  fn two_files() -> Vec<LintResult> {
    let mk = |path: &str, names: &[&str], fix: &[bool]| LintResult {
      file_path: path.into(),
      source: None,
      output: None,
      error_count: names.len() as u32,
      warning_count: 0,
      messages: names
        .iter()
        .zip(fix.iter())
        .map(|(n, f)| m(n, 2, 1, 1, "rule", *f))
        .collect(),
    };
    vec![
      mk("/work/a.js", &["a1", "a2", "a3"], &[false, false, false]),
      mk("/work/b.js", &["b1", "b2", "b3"], &[false, true, false]),
    ]
  }

  fn capped(n: usize) -> FormatOptions {
    FormatOptions {
      max_messages: Some(n),
      ..opts()
    }
  }

  /// Cap fills the first file completely, then spills into the next one,
  /// and appends exactly one notice with the honest hidden count.
  #[test]
  fn cap_renders_first_n_in_render_order_with_notice() {
    let mut results = two_files();
    let out = format_report(&mut results, &FormatterData::default(), &capped(4));
    let plain = strip_ansi(&out);
    // a.js (sorted first) keeps all 3 messages, b.js keeps 1 of 3.
    for name in ["a1", "a2", "a3", "b1"] {
      assert!(
        plain.contains(&format!("{name} [ rule ]")),
        "missing {name}: {plain:?}"
      );
    }
    for name in ["b2", "b3"] {
      assert!(
        !plain.contains(&format!("{name} [ rule ]")),
        "hidden {name} leaked: {plain:?}"
      );
    }
    // A partially rendered file keeps its header (at least one message shown).
    assert!(
      plain.contains(" ./b.js "),
      "partial file keeps its header: {plain:?}"
    );
    // Exactly ONE notice line, with the exact agreed text (unicode prefix).
    assert_eq!(plain.matches("more messages not shown").count(), 1);
    assert!(plain.contains(NOTICE_2), "{plain:?}");

    // Plain (level 0) mode carries the same visible notice text.
    let mut results0 = two_files();
    let o0 = FormatOptions {
      color_level: 0,
      max_messages: Some(4),
      ..opts()
    };
    let plain0 = strip_ansi(&format_report(
      &mut results0,
      &FormatterData::default(),
      &o0,
    ));
    assert!(plain0.contains(NOTICE_2), "{plain0:?}");
  }

  /// The summary (and per-file headers) must stay computed from ALL
  /// messages — the report never lies about totals, even when the message
  /// list on screen is truncated.
  #[test]
  fn cap_totals_stay_full_in_summary_and_headers() {
    let mut results = two_files();
    let out = format_report(&mut results, &FormatterData::default(), &capped(4));
    let plain = strip_ansi(&out);
    // 6 messages total across both files, only 4 rendered.
    assert!(plain.contains(" 6 errors"), "summary counts all: {plain:?}");
    // v0.5.2: zero-count rows are skipped. The warning count is 0, so
    // ` 0 warnings` no longer appears — only ` 6 errors` and the
    // auto-fixable line are rendered (single-line summary).
    assert!(
      !plain.contains(" 0 warnings"),
      "zero-count suppressed: {plain:?}"
    );
    // b.js header reports its TRUE count although only one of its three
    // messages is rendered.
    assert!(plain.contains("./b.js 🔥 3 errors 🔥"), "{plain:?}");
    // The only fixable message (b2) is HIDDEN; the fixable total stays honest.
    assert!(
      plain.contains(" 1 auto-fixable ( with the `--fix` flag )"),
      "{plain:?}"
    );
  }

  /// A file left with zero rendered messages is excluded from the display
  /// entirely (no empty header block) while its messages still count.
  #[test]
  fn cap_excludes_files_without_rendered_messages() {
    let mut results = two_files();
    let out = format_report(&mut results, &FormatterData::default(), &capped(3));
    let plain = strip_ansi(&out);
    assert!(
      !plain.contains("b.js"),
      "file without rendered messages must not appear: {plain:?}"
    );
    assert!(
      plain.contains("a3 [ rule ]"),
      "first file fully rendered: {plain:?}"
    );
    assert!(plain.contains(NOTICE_3), "{plain:?}");
    assert!(plain.contains(" 6 errors"), "totals stay honest: {plain:?}");
  }

  /// Exactly N messages with cap N: no notice, byte-identical to uncapped.
  #[test]
  fn cap_boundary_exact_total_no_notice() {
    let mut uncapped_results = two_files();
    let uncapped = format_report(&mut uncapped_results, &FormatterData::default(), &opts());
    let mut exact_results = two_files();
    let exact = format_report(&mut exact_results, &FormatterData::default(), &capped(6));
    assert_eq!(uncapped, exact, "cap == total must not change the output");
    assert!(!uncapped.contains("not shown"));
  }

  /// One message over the cap: the notice appears (singular), the last
  /// message in render order is hidden, totals unchanged.
  #[test]
  fn cap_boundary_one_over_total_notices_singular() {
    let mut results = two_files();
    let out = format_report(&mut results, &FormatterData::default(), &capped(5));
    let plain = strip_ansi(&out);
    assert!(plain.contains(NOTICE_1), "{plain:?}");
    assert!(
      !plain.contains("b3 [ rule ]"),
      "last message hidden: {plain:?}"
    );
    assert!(plain.contains("b2 [ rule ]"), "earlier messages kept");
    assert!(plain.contains(" 6 errors"), "totals stay honest: {plain:?}");
  }

  /// `max_messages: None` must reproduce today's output byte-for-byte
  /// (no notice, everything rendered) — as must any cap >= the total.
  #[test]
  fn max_messages_none_identical_and_notice_free() {
    let mut none = s1_results();
    let a = format_report(&mut none, &FormatterData::default(), &opts());
    assert!(!a.contains("not shown"), "no notice without a cap");
    for name in ["a", "b", "c", "d", "e", "f", "g"] {
      assert!(a.contains(name), "all messages rendered: {name}");
    }

    let mut huge_results = s1_results();
    let huge = FormatOptions {
      max_messages: Some(usize::MAX),
      ..opts()
    };
    let mut exact_results = s1_results();
    assert_eq!(
      a,
      format_report(&mut huge_results, &FormatterData::default(), &huge),
      "huge cap == None"
    );
    assert_eq!(
      a,
      format_report(&mut exact_results, &FormatterData::default(), &capped(7)),
      "cap == total == None"
    );
  }

  /// Cap above the total: no notice, identical output.
  #[test]
  fn max_messages_above_total_identical_to_uncapped() {
    let mut none = two_files();
    let a = format_report(&mut none, &FormatterData::default(), &opts());
    let mut above = two_files();
    let b = format_report(&mut above, &FormatterData::default(), &capped(100));
    assert_eq!(a, b);
    assert!(!b.contains("not shown"), "no notice when nothing is hidden");
  }

  /// ASCII mode uses the plain `...` prefix and stays pure ASCII; unicode
  /// mode uses the report's ellipsis glyph.
  #[test]
  fn cap_notice_ascii_prefix_vs_unicode_prefix() {
    let mut ascii_results = two_files();
    let o = FormatOptions {
      ascii: true,
      max_messages: Some(4),
      ..opts()
    };
    let out = format_report(&mut ascii_results, &FormatterData::default(), &o);
    assert!(out.is_ascii(), "ascii mode stays pure ascii: {out:?}");
    assert!(
      out.contains(
        "... 2 more messages not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)"
      ),
      "{out:?}"
    );

    let mut uni_results = two_files();
    let out = format_report(&mut uni_results, &FormatterData::default(), &capped(4));
    assert!(out.contains('…'), "unicode ellipsis prefix: {out:?}");
    assert!(out.contains(NOTICE_2), "{out:?}");
    assert!(
      !out.contains("... 2 more messages"),
      "no dotted prefix in unicode mode"
    );
  }
}
