//! Property-based tests for `engine::format_report` (Task 4 — proptest).
//!
//! Four randomized properties pin down the engine's behavioral contracts:
//!
//! 1. `never_panics_on_arbitrary_input`
//!    For any `Vec<LintResult>` with arbitrary file paths, messages, rule
//!    ids, line/column 0..=1000, severity 1 or 2 — `format_report` must
//!    return without panicking. Sanity bound on output size catches any
//!    pathological blow-up.
//!
//! 2. `no_raw_control_chars`
//!    Given pre-sanitized input (the contract the JS adapter actually
//!    feeds the engine — see "Sanitization boundary" note below), the
//!    engine output must not contain raw C0/C1 control bytes except `\n`
//!    and `\t`. This is the injection-prevention invariant: a hostile
//!    upstream cannot smuggle terminal escape sequences through.
//!
//! 3. `totals_match_message_list`
//!    For any input where every message has severity 1 or 2, the engine's
//!    summary block must report ` N errors` / ` M warnings` (with the
//!    correct singular/plural form) where N and M are the actual counts
//!    derived from the message list. See "Counts contract" note below.
//!
//! 4. `cap_truncation_is_honest`
//!    When `max_messages = Some(N)` and the input has more than N
//!    messages across non-clean files, the output must contain the
//!    truncation notice (`not shown`) AND the summary must still report
//!    the full uncapped error/warning counts (audit roadmap ⑥: "the
//!    report never lies about totals").
//!
//! ## Why `#[path]` instead of `use eslint_formatter_crisp::engine::*`?
//!
//! Same reason as `benches/format.rs`: the lib rlib pulls in the napi
//! host symbols via `#[napi]` ctor registrations, which an integration
//! test binary cannot resolve (no Node host). Inlining the engine module
//! via `#[path]` sidesteps the link error without restructuring `lib.rs`
//! (owned by another agent) or adding a workspace feature (would change
//! the shipped `.node` artifact).
//!
//! ## Sanitization boundary
//!
//! The Rust engine does NOT sanitize terminal control chars itself — the
//! JS adapter (`index.cjs::sanitizeControl`) replaces every C0/C1 byte
//! (except `\n` and `\t`) with a Unicode control-picture glyph (`␀`..`␟`,
//! `␡`, or `\u{FFFD}`) before the data crosses the N-API boundary into
//! Rust. The engine's contract is therefore "my caller has already
//! sanitized the input". The `no_raw_control_chars` strategy mirrors
//! that: it generates arbitrary unicode strings and then runs them
//! through a Rust port of `sanitizeControl` before constructing the
//! `LintResult`s. The property under test is then "the engine does not
//! REINTRODUCE control chars the input did not have" — which is the
//! actual security-relevant invariant for the engine half of the
//! pipeline. (For the adapter half, the existing fuzz-regression suite
//! in `test/fuzz-regression.test.mjs` covers hostile JS input.)
//!
//! ## Counts contract
//!
//! `format_report` itself trusts the `error_count` / `warning_count`
//! fields on each `LintResult` — it does NOT recompute them from the
//! message list. The N-API layer (`src/lib.rs::format`) does the
//! recompute (audit finding P6, "Count normalization") BEFORE calling
//! `format_report`. The `totals_match_message_list` strategy mirrors
//! that contract: it pre-computes the per-file counts from the message
//! list, then asserts the engine's summary reports exactly those counts.
//! If a future caller bypasses the N-API layer and feeds bogus counts
//! directly to `format_report`, the engine would faithfully echo the
//! bogus counts — a known architectural limitation, NOT a regression.

#![allow(dead_code, unused_imports)]
#![allow(clippy::all)]

// Inline the engine module — see "Why `#[path]`?" note above.
#[path = "../src/engine/mod.rs"]
mod engine;

use engine::{
  format_report,
  formatter::FormatOptions,
  model::{FormatterData, LintMessage, LintResult},
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategy helpers
// ---------------------------------------------------------------------------

/// Range of severity values the engine treats as well-formed: 1 (warning)
/// and 2 (error). `is_error()` returns true iff severity==2 OR fatal=true;
/// everything else is a warning (so severity 1 maps to warning, and any
/// other value also maps to warning — see `LintMessage::is_error`).
fn arb_severity() -> impl Strategy<Value = Option<i32>> {
  prop_oneof![Just(Some(1)), Just(Some(2)), Just(None)]
}

/// 1-based line/column. `0` is the engine's sentinel for "no location"
/// (the parsing-error short-circuit path), so it is included in the
/// strategy to exercise that branch too. The audit's `toU32` clamp
/// rejects negatives/NaN at the JS boundary; here we test the engine's
/// Rust-side domain directly (already `u32`).
fn arb_line_col() -> impl Strategy<Value = u32> {
  0u32..=1000
}

/// Arbitrary file path. `any::<String>()` generates arbitrary unicode
/// strings (proptest biases toward short / printable ASCII, but includes
/// non-ASCII and the occasional control byte). After `sanitize_control`
/// below, the path mirrors what the JS adapter feeds the engine. Slashes
/// and Windows drive letters show up often enough to exercise the
/// `relative_path` / `split_path` helpers; empty string is also covered
/// (the engine handles it — `split_path` returns `"./"`).
fn arb_file_path() -> impl Strategy<Value = String> {
  any::<String>()
}

/// A single `LintMessage` with arbitrary rule id, message text (after
/// sanitization — see "Sanitization boundary" note above), line/column,
/// and severity. `fixable` / `fatal` are kept simple booleans (proptest
/// picks both).
fn arb_message() -> impl Strategy<Value = LintMessage> {
  (
    any::<String>(),
    arb_severity(),
    arb_line_col(),
    arb_line_col(),
    prop::option::of(any::<String>()),
    any::<bool>(),
    any::<bool>(),
    prop::option::of(any::<String>()),
  )
    .prop_map(
      |(message, severity, line, column, rule_id, fatal, fixable, suggestion)| LintMessage {
        message: sanitize_control(&message),
        severity,
        line,
        column,
        rule_id: rule_id.as_ref().map(|s| sanitize_control(s)),
        fatal,
        fixable,
        suggestion: suggestion.as_ref().map(|s| sanitize_control(s)),
      },
    )
}

/// One file's worth of messages: 0..`msgs_per_file` messages.
fn arb_messages(msgs_per_file: std::ops::Range<usize>) -> impl Strategy<Value = Vec<LintMessage>> {
  prop::collection::vec(arb_message(), msgs_per_file)
}

/// A `LintResult` for one file. The `error_count` / `warning_count`
/// fields are pre-computed from the generated message list (mirroring
/// what `src/lib.rs::format` does for real callers — see "Counts
/// contract" note above). `source` and `output` are NOT generated: the
/// engine treats source text as trusted (it is the file ESLint just
/// parsed), so source-text sanitization is the JS adapter's
/// responsibility and is not exercised here.
fn arb_result(msgs_per_file: std::ops::Range<usize>) -> impl Strategy<Value = LintResult> {
  (arb_file_path(), arb_messages(msgs_per_file)).prop_map(|(file_path, messages)| {
    let errors = messages.iter().filter(|m| m.is_error()).count() as u32;
    let warnings = messages.len() as u32 - errors;
    LintResult {
      file_path: sanitize_control(&file_path),
      source: None,
      output: None,
      error_count: errors,
      warning_count: warnings,
      messages,
    }
  })
}

/// Generate `n_files` `LintResult`s, each with 0..`msgs_per_file`
/// messages.
fn arb_results(
  n_files: std::ops::Range<usize>,
  msgs_per_file: std::ops::Range<usize>,
) -> impl Strategy<Value = Vec<LintResult>> {
  prop::collection::vec(arb_result(msgs_per_file), n_files)
}

// ---------------------------------------------------------------------------
// Rust port of `index.cjs::sanitizeControl`
// ---------------------------------------------------------------------------

/// Replace every C0/C1 control byte (except `\n` and `\t`) with a
/// Unicode control-picture glyph, mirroring `sanitizeControl` in
/// `index.cjs`. This is what real callers feed the engine, so the
/// `no_raw_control_chars` strategy runs arbitrary strings through this
/// before constructing `LintResult`s.
fn sanitize_control(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for c in input.chars() {
    let code = c as u32;
    // C0 control range EXCEPT \t (0x09) and \n (0x0A).
    let is_c0_other = code <= 0x1F && code != 0x09 && code != 0x0A;
    // C1 control range (0x7F..=0x9F): DEL + 0x80..0x9F.
    let is_c1 = (0x7F..=0x9F).contains(&code);
    if is_c0_other {
      // 0x00..0x1F (excluding \t, \n) -> ␀..␟ (U+2400 + code)
      out.push(char::from_u32(0x2400 + code).unwrap_or('\u{FFFD}'));
    } else if code == 0x7F {
      // DEL -> ␡ (U+2421)
      out.push('\u{2421}');
    } else if is_c1 {
      // C1 range has no Unicode control-picture block — use REPLACEMENT
      // CHARACTER (parity with `index.cjs::sanitizeControl`).
      out.push('\u{FFFD}');
    } else {
      out.push(c);
    }
  }
  out
}

// ---------------------------------------------------------------------------
// Options helpers
// ---------------------------------------------------------------------------

fn opts(color_level: u8, max_messages: Option<usize>) -> FormatOptions {
  FormatOptions {
    color_level,
    base_dir: "/work".into(),
    links: false,
    ascii: false,
    max_width: None,
    suggestions: false,
    theme: Default::default(),
    max_messages,
  }
}

// ---------------------------------------------------------------------------
// Property 1 — never panics on arbitrary input
// ---------------------------------------------------------------------------

proptest! {
  /// `format_report` must return for any input. The engine has explicit
  /// "never panics" coverage on its sub-modules (`highlight_line` /
  /// `locate` / `bold_message`); this test pins the property at the
  /// top-level entry point and exercises the sort step (path ordering
  /// ties) and the code-frame path (locating errors in absent source).
  #[test]
  fn never_panics_on_arbitrary_input(
    results in arb_results(0..10, 0..50)
  ) {
    let mut results = results;
    let out = format_report(
      &mut results,
      &FormatterData::default(),
      &opts(0, None),
    );
    // Sanity bound: 50 files × 50 messages × ~50 bytes each ≈ 125 KB
    // of output for the largest generated case. A 10 MB ceiling
    // catches any pathological blow-up without false-positiving on
    // legitimate formatting.
    prop_assert!(
      out.len() < 10_000_000,
      "output grew to {} bytes (sanity bound 10 MB)",
      out.len()
    );
  }
}

// ---------------------------------------------------------------------------
// Property 2 — no raw control chars
// ---------------------------------------------------------------------------

proptest! {
  /// Output must not contain raw C0/C1 control bytes (except `\n` and
  /// `\t`). The strategy mirrors the JS adapter's `sanitizeControl` pass
  /// on input fields, so this test verifies the engine's invariant: it
  /// does not REINTRODUCE control chars the input did not have. Run at
  /// color_level 0 so the engine's own SGR/OSC escapes (which contain
  /// `\x1b`) don't trivially fail the check.
  #[test]
  fn no_raw_control_chars(
    results in arb_results(0..5, 0..20)
  ) {
    let mut results = results;
    let out = format_report(
      &mut results,
      &FormatterData::default(),
      &opts(0, None),
    );
    // Check Unicode CODE POINTS, not bytes: the engine emits multi-byte
    // UTF-8 glyphs (`─` is U+2500 = E2 94 80) whose continuation bytes
    // happen to land in 0x80..0x9F — but those are not control chars,
    // they're interior bytes of a single non-control code point. The
    // JS adapter's `/[\u0000-\u0008\u000B-\u001F\u007F-\u009F]/` regex
    // matches the same code-point set we check here.
    for (char_idx, c) in out.chars().enumerate() {
      let code = c as u32;
      let is_c0_other =
        code <= 0x1F && code != 0x09 && code != 0x0A;
      let is_c1 = (0x7F..=0x9F).contains(&code);
      if is_c0_other || is_c1 {
        // Locate the code point in the byte stream for the diagnostic.
        let byte_off = out.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(0);
        return Err(TestCaseError::fail(format!(
          "raw control char U+{:04X} at char idx {} / byte offset {} (output prefix: {:?})",
          code,
          char_idx,
          byte_off,
          &out[..byte_off.min(120)],
        )));
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Property 3 — totals match the message list
// ---------------------------------------------------------------------------

proptest! {
  /// For any input whose messages all have severity 1 (warning) or 2
  /// (error), the summary block must report exactly `N errors` / `M
  /// warnings` (correctly pluralized) where N and M are the actual
  /// counts derived from the message list. The strategy pre-computes
  /// the per-file counts (mirroring `src/lib.rs::format`'s P6 recompute)
  /// before calling `format_report`, so the engine sees correct counts
  /// — and the assertion verifies the engine echoes them faithfully.
  #[test]
  fn totals_match_message_list(
    results in arb_results(1..5, 1..20)
  ) {
    let mut results = results;
    // Tally actual counts over ALL non-empty files (mirrors
    // `format_report`'s `total_errors` / `total_warnings` accumulator).
    let mut expected_errors: u32 = 0;
    let mut expected_warnings: u32 = 0;
    for r in &results {
      for m in &r.messages {
        if m.is_error() {
          expected_errors += 1;
        } else {
          expected_warnings += 1;
        }
      }
    }
    // If every file is empty (no messages), `format_report` returns "".
    if expected_errors + expected_warnings == 0 {
      return Ok(());
    }
    let out = format_report(
      &mut results,
      &FormatterData::default(),
      &opts(0, None),
    );
    let expected_err_label = if expected_errors == 1 { "error" } else { "errors" };
    let expected_warn_label = if expected_warnings == 1 { "warning" } else { "warnings" };
    let expected_err_line = format!(" {} {}", expected_errors, expected_err_label);
    let expected_warn_line = format!(" {} {}", expected_warnings, expected_warn_label);
    // v0.5.2: zero-count rows are skipped in the summary. Only assert the
    // presence of NON-ZERO counts — a zero count is now correctly absent.
    if expected_errors > 0 {
      prop_assert!(
        out.contains(&expected_err_line),
        "summary must report non-zero {}; got output: {:?}",
        expected_err_line,
        out,
      );
    }
    if expected_warnings > 0 {
      prop_assert!(
        out.contains(&expected_warn_line),
        "summary must report non-zero {}; got output: {:?}",
        expected_warn_line,
        out,
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Property 4 — cap truncation is honest
// ---------------------------------------------------------------------------

proptest! {
  /// When `max_messages = Some(N)` and the input has more than N
  /// messages across non-clean files, the engine must (a) emit the
  /// `not shown` truncation notice and (b) STILL report the full
  /// uncapped error/warning counts in the summary. This is the
  /// audit-roadmap-⑥ contract: the cap bounds render cost without
  /// lying about totals.
  #[test]
  fn cap_truncation_is_honest(
    n_files in 1usize..5,
    msgs_per_file in 10usize..30,
    cap_n in 1usize..50,
  ) {
    // Strategy: pick dimensions so that total messages > cap, ensuring
    // the cap actually triggers. With `n_files >= 1` and
    // `msgs_per_file >= 10`, the smallest total is 10; with `cap_n <
    // 50`, the cap will trigger in most random draws. The check below
    // skips the assertion when the cap doesn't trigger (cap >= total).
    let mut results = arb_results_with_dims(n_files, msgs_per_file);
    let total_msgs: usize = results.iter().map(|r| r.messages.len()).sum();
    if total_msgs <= cap_n {
      // Cap wouldn't trigger — nothing to assert here. proptest will
      // re-draw on the next iteration with different dimensions.
      return Ok(());
    }
    let expected_errors: u32 = results
      .iter()
      .flat_map(|r| r.messages.iter())
      .filter(|m| m.is_error())
      .count() as u32;
    let expected_warnings: u32 = results
      .iter()
      .flat_map(|r| r.messages.iter())
      .filter(|m| !m.is_error())
      .count() as u32;
    let out = format_report(
      &mut results,
      &FormatterData::default(),
      &opts(0, Some(cap_n)),
    );
    prop_assert!(
      out.contains("not shown"),
      "cap dropped {} of {} messages but no truncation notice appeared; output: {:?}",
      total_msgs - cap_n,
      total_msgs,
      out,
    );
    let err_label = if expected_errors == 1 { "error" } else { "errors" };
    let warn_label = if expected_warnings == 1 { "warning" } else { "warnings" };
    if expected_errors > 0 {
      let expected_err_line = format!(" {} {}", expected_errors, err_label);
      prop_assert!(
        out.contains(&expected_err_line),
        "summary must report full uncapped error count {}; got: {:?}",
        expected_err_line,
        out,
      );
    }
    if expected_warnings > 0 {
      let expected_warn_line = format!(" {} {}", expected_warnings, warn_label);
      prop_assert!(
        out.contains(&expected_warn_line),
        "summary must report full uncapped warning count {}; got: {:?}",
        expected_warn_line,
        out,
      );
    }
  }
}

/// Build `n_files` `LintResult`s with EXACTLY `msgs_per_file` messages
/// each (no random message-count variation — for the cap test we need
/// deterministic total so we can check the cap actually triggers).
fn arb_results_with_dims(n_files: usize, msgs_per_file: usize) -> Vec<LintResult> {
  (0..n_files)
    .map(|f| {
      let messages: Vec<LintMessage> = (0..msgs_per_file)
        .map(|i| LintMessage {
          message: sanitize_control(&format!("err {} in file {}", i, f)),
          severity: Some(if i.is_multiple_of(2) { 2 } else { 1 }),
          line: ((i % 5) + 1) as u32,
          column: ((i % 7) + 1) as u32,
          rule_id: Some(format!("rule-{}", i % 3)),
          fatal: false,
          fixable: false,
          suggestion: None,
        })
        .collect();
      let errors = messages.iter().filter(|m| m.is_error()).count() as u32;
      let warnings = messages.len() as u32 - errors;
      LintResult {
        file_path: format!("/work/src/file_{}.js", f),
        source: None,
        output: None,
        error_count: errors,
        warning_count: warnings,
        messages,
      }
    })
    .collect()
}
