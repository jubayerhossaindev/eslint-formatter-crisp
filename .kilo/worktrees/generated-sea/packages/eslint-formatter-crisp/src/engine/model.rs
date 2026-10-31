//! Engine-level data model. Framework-agnostic: the N-API layer converts
//! JavaScript objects into these structs; the engine never touches Node types.

/// A single ESLint lint message, normalized from ESLint's LintMessage.
#[derive(Debug, Clone, PartialEq)]
pub struct LintMessage {
  /// Raw message text from ESLint (may contain quoted identifiers).
  pub message: String,
  /// ESLint severity: 1 = warning, 2 = error. `None` is treated as warning.
  pub severity: Option<i32>,
  /// 1-based line number. `None` / 0 means "no location" (parsing error).
  pub line: u32,
  /// 1-based column number. `None` / 0 means "no location".
  pub column: u32,
  /// Rule identifier, e.g. `no-unused-vars` or `import-x/no-duplicates`.
  pub rule_id: Option<String>,
  /// True when the message is a fatal parsing error.
  pub fatal: bool,
  /// True when the message carries a `fix` (auto-fixable).
  pub fixable: bool,
  /// First fix suggestion's description (ESLint `suggestions[0].desc`),
  /// extracted by the JS adapter only when the experimental suggestions
  /// display is enabled. `None` for plain runs.
  pub suggestion: Option<String>,
}

/// Per-file ESLint result, normalized from ESLint's LintResult.
#[derive(Debug, Clone, PartialEq)]
pub struct LintResult {
  /// Absolute path of the linted file, as reported by ESLint.
  pub file_path: String,
  /// Original source text (absent when ESLint suppresses it).
  pub source: Option<String>,
  /// Fixed source text produced by `--fix-dry-run` / `--fix` runs.
  /// The engine prefers `output` over `source` for code frames,
  /// matching the reference behavior verified in Phase 1 (experiment s8).
  pub output: Option<String>,
  pub error_count: u32,
  pub warning_count: u32,
  pub messages: Vec<LintMessage>,
}

/// `docs.url` node inside ESLint's `rulesMeta`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuleDocs {
  pub url: Option<String>,
  /// True when ESLint's rule metadata marks the rule deprecated
  /// (experimental display, gated behind CRISP_SUGGESTIONS=1).
  pub deprecated: Option<bool>,
}

/// Per-rule metadata passed by ESLint as the second formatter argument.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuleMeta {
  pub docs: Option<RuleDocs>,
}

/// The `data` argument ESLint passes to formatters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatterData {
  /// ruleId -> metadata (documentation URLs).
  pub rules_meta: std::collections::HashMap<String, RuleMeta>,
}

impl LintMessage {
  /// Effective severity: fatal or severity==2 => error, otherwise warning.
  /// Mirrors the reference logic at formatter.js:95-97 (Phase 1 report §5.1).
  pub fn is_error(&self) -> bool {
    self.fatal || self.severity.unwrap_or(1) == 2
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn severity_error_variants() {
    let mut m = LintMessage {
      message: "x".into(),
      severity: Some(2),
      line: 1,
      column: 1,
      rule_id: None,
      fatal: false,
      fixable: false,
      suggestion: None,
    };
    assert!(m.is_error());
    m.severity = Some(1);
    assert!(!m.is_error());
    m.fatal = true;
    assert!(m.is_error(), "fatal must force error severity");
    m.severity = None;
    m.fatal = false;
    assert!(!m.is_error(), "missing severity defaults to warning");
  }
}
