//! eslint-formatter-crisp — pure Rust formatting engine.
//!
//! `engine` is framework-agnostic (unit-tested with plain `cargo test`).
//! The N-API binding lives in the `crisp-native` crate.

pub mod engine;

pub use engine::model::{FormatterData, LintMessage, LintResult, RuleDocs, RuleMeta};
pub use engine::{DEFAULT_MAX_MESSAGES, FormatOptions, format_report};
