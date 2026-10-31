//! eslint-formatter-crisp — Rust core + N-API binding.
//!
//! `engine` is framework-agnostic (unit-tested with plain cargo test); the
//! `#[napi]` exports below are a thin typed conversion layer. JS errors are
//! thrown by napi-rs when the input shape does not match (e.g. a non-string
//! `message`), producing clear `TypeError`s at the boundary.

pub mod engine;

pub use engine::format_report;
pub use engine::model::{FormatterData, LintMessage, LintResult};

use engine::model::{RuleDocs, RuleMeta};
use engine::style::ThemeOverride;
use engine::{DEFAULT_MAX_MESSAGES, FormatOptions};
use napi_derive::napi;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// JS-facing object schemas (camelCase in JS, snake_case in Rust)
// ---------------------------------------------------------------------------

#[napi(object)]
pub struct JsLintMessage {
  pub message: String,
  /// ESLint severity: 1 = warning, 2 = error.
  pub severity: Option<i32>,
  /// 1-based; 0/absent = "no location" (parsing errors).
  pub line: Option<u32>,
  pub column: Option<u32>,
  pub rule_id: Option<String>,
  pub fatal: Option<bool>,
  /// Presence of ESLint's `fix` property; the adapter maps `!!m.fix`.
  pub fix: Option<bool>,
  /// First fix suggestion description (only set when the experimental
  /// suggestions display is enabled in the adapter).
  pub suggestion: Option<String>,
}

#[napi(object)]
pub struct JsLintResult {
  pub file_path: String,
  pub source: Option<String>,
  /// Post-fix source from `--fix-dry-run` runs (preferred over `source`).
  pub output: Option<String>,
  pub error_count: Option<u32>,
  pub warning_count: Option<u32>,
  pub messages: Vec<JsLintMessage>,
}

#[napi(object)]
pub struct JsRuleDocs {
  pub url: Option<String>,
  /// Present only when the rule metadata marks the rule deprecated
  /// (ESLint >= 8.41 rulesMeta.docs.deprecated). Consumed by the
  /// experimental deprecation marker (CRISP_SUGGESTIONS=1, v0.3.0+).
  pub deprecated: Option<bool>,
}

#[napi(object)]
pub struct JsRuleMeta {
  pub docs: Option<JsRuleDocs>,
}

#[napi(object)]
pub struct JsFormatterData {
  pub rules_meta: Option<HashMap<String, JsRuleMeta>>,
}

#[napi(object)]
pub struct JsFormatOptions {
  /// 0 = plain, 1 = 16-color, 2 = 256-color, 3 = 24-bit. Detected in JS.
  pub color_level: Option<i32>,
  /// ESLint cwd; used to relativize file paths. Defaults to ".".
  pub base_dir: Option<String>,
  /// OSC 8 hyperlinks. Only enable when the stream is a capable TTY.
  pub links: Option<bool>,
  /// Pure-ASCII decoration mode. The JS adapter sets this from the
  /// `CRISP_ASCII=1` environment variable.
  pub ascii: Option<bool>,
  /// Terminal width in columns. The JS adapter derives it from the
  /// `COLUMNS` environment variable or the TTY size; `CRISP_FULL_LINES=1`
  /// leaves it unset (unlimited-width output).
  pub max_width: Option<u32>,
  /// Whole-report message cap (audit roadmap ⑥). The JS adapter feeds the
  /// `CRISP_MAX_MESSAGES` environment variable here (parsing stays in JS —
  /// the engine never reads the environment): absent -> engine default cap
  /// (`DEFAULT_MAX_MESSAGES`, 5000), `0` -> unlimited, `n` -> cap at `n`.
  pub max_messages: Option<u32>,
  /// Experimental suggestions/deprecation display (`CRISP_SUGGESTIONS=1`).
  pub suggestions: Option<bool>,
  /// Experimental color overrides (`CRISP_THEME` / `MO_THEME`).
  pub theme: Option<JsThemeOverride>,
}

#[napi(object)]
pub struct JsThemeOverride {
  /// `#rrggbb` hex strings; invalid values are dropped in the parser.
  pub fruit_a: Option<String>,
  pub fruit_b: Option<String>,
  pub cristal_a: Option<String>,
  pub cristal_b: Option<String>,
  /// Disable the syntax-highlighter token colors entirely.
  pub mono: Option<bool>,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// JS `max_messages` -> engine cap. Absent: apply the Rust-side default
/// (`DEFAULT_MAX_MESSAGES`) so pathological runs stay bounded even when the
/// adapter passes no option at all. `0` explicitly means unlimited -> `None`.
fn resolve_max_messages(v: Option<u32>) -> Option<usize> {
  match v {
    None => Some(DEFAULT_MAX_MESSAGES),
    Some(0) => None,
    Some(n) => Some(n as usize),
  }
}

/// Parse `#rrggbb` / `rrggbb` into RGB; invalid input -> None.
fn parse_hex_color(s: Option<&String>) -> Option<(u8, u8, u8)> {
  let s = s?;
  let hex = s.strip_prefix('#').unwrap_or(s);
  if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
    return None;
  }
  let v = u32::from_str_radix(hex, 16).ok()?;
  Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn to_engine_theme(t: Option<JsThemeOverride>) -> ThemeOverride {
  let Some(t) = t else {
    return ThemeOverride::default();
  };
  ThemeOverride {
    fruit_a: parse_hex_color(t.fruit_a.as_ref()),
    fruit_b: parse_hex_color(t.fruit_b.as_ref()),
    cristal_a: parse_hex_color(t.cristal_a.as_ref()),
    cristal_b: parse_hex_color(t.cristal_b.as_ref()),
    mono: t.mono.unwrap_or(false),
  }
}

fn to_engine_message(m: JsLintMessage) -> LintMessage {
  LintMessage {
    message: m.message,
    severity: m.severity,
    line: m.line.unwrap_or(0),
    column: m.column.unwrap_or(0),
    rule_id: m.rule_id,
    fatal: m.fatal.unwrap_or(false),
    fixable: m.fix.unwrap_or(false),
    suggestion: m.suggestion,
  }
}

fn to_engine_result(r: JsLintResult) -> LintResult {
  LintResult {
    file_path: r.file_path,
    source: r.source,
    output: r.output,
    error_count: r.error_count.unwrap_or(0),
    warning_count: r.warning_count.unwrap_or(0),
    messages: r.messages.into_iter().map(to_engine_message).collect(),
  }
}

fn to_engine_data(d: Option<JsFormatterData>) -> FormatterData {
  let mut rules_meta = HashMap::new();
  if let Some(d) = d
    && let Some(map) = d.rules_meta
  {
    for (k, v) in map {
      rules_meta.insert(
        k,
        RuleMeta {
          docs: v.docs.map(|docs| RuleDocs {
            url: docs.url,
            deprecated: docs.deprecated,
          }),
        },
      );
    }
  }
  FormatterData { rules_meta }
}

// ---------------------------------------------------------------------------
// Exported formatter entry point
// ---------------------------------------------------------------------------

/// Format ESLint results into a terminal report string.
///
/// This is the single native entry point; the JS adapter is responsible for
/// color-level/link detection and for clear error messages around loading.
#[napi]
pub fn format(
  results: Vec<JsLintResult>,
  data: Option<JsFormatterData>,
  options: Option<JsFormatOptions>,
) -> napi::Result<String> {
  let opts = options.unwrap_or(JsFormatOptions {
    color_level: None,
    base_dir: None,
    links: None,
    ascii: None,
    max_width: None,
    max_messages: None,
    suggestions: None,
    theme: None,
  });
  let engine_opts = FormatOptions {
    color_level: opts.color_level.unwrap_or(1).clamp(0, 3) as u8,
    base_dir: opts.base_dir.unwrap_or_else(|| ".".to_string()),
    links: opts.links.unwrap_or(false),
    ascii: opts.ascii.unwrap_or(false),
    max_width: opts
      .max_width
      .and_then(|w| if w >= 20 { Some(w as usize) } else { None }),
    max_messages: resolve_max_messages(opts.max_messages),
    suggestions: opts.suggestions.unwrap_or(false),
    theme: to_engine_theme(opts.theme),
  };

  // Count normalization (audit finding P6): the per-file error/warning
  // counts are ALWAYS recomputed from the message list, never trusted from
  // the input. For real ESLint data this is an identity operation (ESLint's
  // counts already match its messages), but it makes the summary and the
  // rendered message list impossible to desync — a hand-rolled caller
  // sending errorCount=1 with 2 error messages now renders 2 errors in the
  // header AND the summary. Negative/huge/absent inputs simply resolve to
  // whatever the messages support (0 for an empty list).
  let mut engine_results: Vec<LintResult> = results.into_iter().map(to_engine_result).collect();
  for r in engine_results.iter_mut() {
    let errors = r.messages.iter().filter(|m| m.is_error()).count() as u32;
    r.error_count = errors;
    r.warning_count = r.messages.len() as u32 - errors;
  }

  let data = to_engine_data(data);
  Ok(format_report(&mut engine_results, &data, &engine_opts))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resolve_max_messages_mapping() {
    assert_eq!(DEFAULT_MAX_MESSAGES, 5000);
    // Absent option: the Rust default cap applies (audit roadmap ⑥).
    assert_eq!(resolve_max_messages(None), Some(5000));
    // Explicit 0 = unlimited.
    assert_eq!(resolve_max_messages(Some(0)), None);
    // Explicit values pass through.
    assert_eq!(resolve_max_messages(Some(1)), Some(1));
    assert_eq!(resolve_max_messages(Some(12_345)), Some(12_345));
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

  /// `count` error messages in one file, no source (one fallback row each).
  fn js_results(count: usize) -> Vec<JsLintResult> {
    vec![JsLintResult {
      file_path: "/work/big.js".into(),
      source: None,
      output: None,
      error_count: None,
      warning_count: None,
      messages: (0..count)
        .map(|i| JsLintMessage {
          message: format!("msg{i}"),
          severity: Some(2),
          line: Some(1),
          column: Some(1),
          rule_id: Some("r".into()),
          fatal: None,
          fix: None,
          suggestion: None,
        })
        .collect(),
    }]
  }

  fn js_opts(max_messages: Option<u32>) -> Option<JsFormatOptions> {
    Some(JsFormatOptions {
      color_level: Some(0),
      base_dir: None,
      links: None,
      ascii: None,
      max_width: None,
      max_messages,
      suggestions: None,
      theme: None,
    })
  }

  /// The Rust default cap of 5000 applies even when the adapter passes no
  /// options object at all; the summary stays honest (5001 problems).
  #[test]
  fn default_cap_5000_applies_without_options() {
    let results = js_results(DEFAULT_MAX_MESSAGES + 1);
    let out = format(results, None, None).unwrap();
    let plain = strip_ansi(&out);
    assert!(
      plain.contains(
        "… 1 more message not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)"
      ),
      "{plain:?}"
    );
    assert!(plain.contains(" 5001 errors"), "honest totals: {plain:?}");
    assert!(plain.contains("✖ msg0 [ r ]"), "first message rendered");
    assert!(plain.contains("✖ msg4999 [ r ]"), "5000th message rendered");
    assert!(
      !plain.contains("✖ msg5000 [ r ]"),
      "message beyond the cap is hidden"
    );
  }

  /// `max_messages: 0` maps to unlimited: no notice, everything rendered.
  #[test]
  fn zero_max_messages_disables_cap() {
    let results = js_results(DEFAULT_MAX_MESSAGES + 1);
    let out = format(results, None, js_opts(Some(0))).unwrap();
    assert!(!out.contains("not shown"), "0 = unlimited: {out:?}");
    assert!(out.contains(" 5001 errors"), "{out:?}");
    assert!(out.contains("✖ msg5000 [ r ]"), "last message rendered");
  }

  /// An explicit cap passes through to the engine unchanged.
  #[test]
  fn explicit_max_messages_passthrough() {
    let results = js_results(6);
    let out = format(results, None, js_opts(Some(3))).unwrap();
    assert!(
      out.contains(
        "… 3 more messages not shown (raised cap: set CRISP_MAX_MESSAGES=0 for unlimited)"
      ),
      "{out:?}"
    );
    assert!(out.contains(" 6 errors"), "honest totals: {out:?}");
    assert!(out.contains("✖ msg2 [ r ]"), "third message rendered");
    assert!(!out.contains("✖ msg3 [ r ]"), "fourth message hidden");
  }
}
