//! Criterion micro-benchmarks for `engine::format_report`.
//!
//! Three scenarios span the realistic load range observed in production:
//! - `small_5_msgs`     - 1 file, 5 messages (typical single-file `eslint .`)
//! - `medium_1000_msgs`  - 100 files, 10 messages each = 1 000 messages
//!   (typical project-wide lint)
//! - `large_50000_msgs` - 1 000 files, 50 messages each = 50 000 messages
//!   (large monorepo; exercises the CRISP_MAX_MESSAGES = 5 000 default cap
//!   active in the engine).
//!
//! Run: `cargo bench` (auto-discovers this file via the `[[bench]]` entry in
//! Cargo.toml). HTML reports land under `target/criterion/`.
//!
//! The benchmark re-uses the `LintResult` / `LintMessage` / `FormatterData`
//! / `FormatOptions` types straight from the engine module - the N-API
//! `JsLintResult` -> `LintResult` conversion happens in `src/lib.rs::format`
//! and is intentionally out of scope here (we are measuring the engine
//! pipeline: sort, relativize, code-frame, summary).
//!
//! ## Why `#[path]` instead of `use eslint_formatter_crisp::engine::*`?
//!
//! The crate ships as `cdylib + rlib`. The `cdylib` is the `.node` artifact
//! loaded by Node; the `rlib` is what unit tests / benches / integration
//! tests link against. The `#[napi]` macros in `src/lib.rs` generate
//! `#[ctor]` registration functions (gated on `not(test)` in napi-derive's
//! codegen) that pull in `napi_get_named_property` / `napi_create_function`
//! / etc. - extern symbols the host (Node) provides at runtime. When the
//! bench binary links the lib rlib, those ctor'd wrappers drag the napi
//! externs into the link graph and the bench fails to link (no Node host).
//! The lib unittest binary sidesteps this because `cargo test` compiles
//! `src/lib.rs` with `--test`, which sets `cfg(test)` and turns the
//! `#[ctor]` emissions off - the wrappers then dead-code-eliminate.
//!
//! Benches run with `harness = false` (criterion owns `main`) and therefore
//! cannot re-use that `--test` shortcut. The cleanest workaround that does
//! NOT require restructuring `lib.rs` (which is owned by another agent) or
//! adding a workspace feature (Cargo unifies features across targets, so
//! `dyn-symbols` would change the shipped `.node` artifact) is to inline
//! the engine module via `#[path]`: the bench binary then compiles the
//! engine sources directly and does NOT link the lib rlib at all, so the
//! napi `#[ctor]` wrappers are never pulled into the link graph. The
//! engine source has zero napi dependencies (it only uses `unicode-width`),
//! so this is safe.
//!
//! The `#[allow(...)]` annotations below keep clippy quiet about the dead
//! `#[cfg(test)] mod tests` helpers that the engine source files bring
//! along (those tests are compiled because `--cfg test` is set on the bench
//! invocation, but their `#[test]` functions are never registered with the
//! test harness since `harness = false` - criterion owns `main`).

// Inline the engine module so the bench binary does not depend on
// `libeslint_formatter_crisp.rlib` (which would pull in the napi host
// symbols via `#[napi]` ctor registrations - see module docs above).
#![allow(dead_code, unused_imports)]
#![allow(clippy::all)]
#[path = "../src/engine/mod.rs"]
mod engine;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use engine::{
  format_report,
  formatter::FormatOptions,
  model::{FormatterData, LintMessage, LintResult},
};

/// Realistic JavaScript source for one file — short enough to keep the
/// benchmark setup cheap, long enough that the locator's source-excerpt
/// path (highlight + indicator rows) runs to completion on every iter.
const SAMPLE_SRC: &str = "\
const unusedVar = 42;
const greeting = 'hello ' + name;
console.log(greeting);

function add(a, b) {
  return a + b;
}
";

/// One "realistic" lint message: rule id, human text with a quoted
/// identifier (exercises `bold_message`), 1-based line/column inside the
/// sample source, error severity, half are auto-fixable.
fn make_msg(idx: usize) -> LintMessage {
  let rule_suffix = idx % 4;
  LintMessage {
    message: format!("'x{idx}' is assigned a value but never used."),
    severity: Some(if idx.is_multiple_of(3) { 1 } else { 2 }),
    line: ((idx % 5) + 1) as u32,
    column: ((idx % 7) + 1) as u32,
    rule_id: Some(format!("no-unused-vars-{rule_suffix}")),
    fatal: false,
    fixable: idx.is_multiple_of(2),
    suggestion: None,
  }
}

/// Build `n_files` results, each with `msgs_per_file` messages. The
/// `error_count` / `warning_count` fields are pre-populated honestly —
/// `format_report` itself recomputes them from the message list (audit
/// finding P6), but we feed them in the expected shape so the sort step
/// (`errorCount DESC, warningCount DESC, path ASC`) sees realistic data
/// rather than the all-zero fallback.
fn make_results(n_files: usize, msgs_per_file: usize) -> Vec<LintResult> {
  (0..n_files)
    .map(|f| {
      let messages: Vec<LintMessage> = (0..msgs_per_file).map(|i| make_msg(f * 31 + i)).collect();
      let errors = messages.iter().filter(|m| m.is_error()).count() as u32;
      let warnings = messages.len() as u32 - errors;
      LintResult {
        file_path: format!("/work/src/module_{f}/file_{f}.js"),
        source: Some(SAMPLE_SRC.to_string()),
        output: None,
        error_count: errors,
        warning_count: warnings,
        messages,
      }
    })
    .collect()
}

/// Build the default `FormatOptions` for small/medium (no cap; the engine
/// runs at full color level 1 because that is the most common runtime
/// shape and exercises the gradient + ANSI-emission code paths).
fn opts(max_messages: Option<usize>) -> FormatOptions {
  FormatOptions {
    color_level: 1,
    base_dir: "/work".into(),
    links: false,
    ascii: false,
    max_width: None,
    suggestions: false,
    theme: Default::default(),
    max_messages,
  }
}

fn bench_format(c: &mut Criterion) {
  let mut group = c.benchmark_group("format_report");

  // Small: 1 file × 5 messages — single-file lint. Sub-millisecond
  // territory; criterion's `iter` keeps the noise floor readable.
  group.bench_function("small_5_msgs", |b| {
    b.iter(|| {
      let mut results = make_results(1, 5);
      format_report(
        black_box(&mut results),
        &FormatterData::default(),
        &opts(None),
      )
    })
  });

  // Medium: 100 files × 10 messages = 1 000 messages. Typical project-wide
  // `eslint .` after a feature branch lands.
  group.bench_function("medium_1000_msgs", |b| {
    b.iter(|| {
      let mut results = make_results(100, 10);
      format_report(
        black_box(&mut results),
        &FormatterData::default(),
        &opts(None),
      )
    })
  });

  // Large: 1 000 files × 50 messages = 50 000 messages — the audit's
  // pathological case. With `max_messages = Some(5000)` the engine renders
  // only the first 5 000 messages (filling file 1 fully, then spilling)
  // and appends ONE truncation notice, while still scanning ALL messages
  // for the honest summary totals. This is the DoS-guard hot path the
  // audit roadmap ⑥ added the cap for.
  group.bench_function("large_50000_msgs", |b| {
    b.iter(|| {
      let mut results = make_results(1000, 50);
      format_report(
        black_box(&mut results),
        &FormatterData::default(),
        &opts(Some(5000)),
      )
    })
  });

  group.finish();
}

criterion_group!(benches, bench_format);
criterion_main!(benches);
