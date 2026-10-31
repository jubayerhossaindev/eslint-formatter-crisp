//! Rule documentation URL resolution.
//!
//! Priority (mirrors the reference `getRuleUrl` at utils.js:29-39, verified
//! Phase 1 finding F-table): ESLint's `data.rulesMeta[ruleId].docs.url` first;
//! then a built-in fallback for core ESLint rules. The reference falls back to
//! the `eslint-rule-docs` package (a large plugin registry); crisp instead
//! links core rules only and omits the link for unknown plugin rules. This is
//! a documented intentional difference (report §M0 improvements list).

use crate::engine::model::FormatterData;

fn core_rule_url(rule_id: &str) -> Option<String> {
  if rule_id.is_empty() || rule_id.contains('/') {
    return None; // plugin rules have no stable core URL
  }
  Some(format!("https://eslint.org/docs/latest/rules/{}", rule_id))
}

pub fn get_rule_url(rule_id: Option<&str>, data: &FormatterData) -> Option<String> {
  let rule_id = rule_id?;
  if let Some(meta) = data.rules_meta.get(rule_id)
    && let Some(docs) = &meta.docs
    && let Some(url) = &docs.url
    && !url.is_empty()
  {
    return Some(url.clone());
  }
  core_rule_url(rule_id)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::engine::model::{RuleDocs, RuleMeta};
  use std::collections::HashMap;

  fn data_with(rule: &str, url: &str) -> FormatterData {
    let mut m = HashMap::new();
    m.insert(
      rule.to_string(),
      RuleMeta {
        docs: Some(RuleDocs {
          url: Some(url.to_string()),
          deprecated: None,
        }),
      },
    );
    FormatterData { rules_meta: m }
  }

  #[test]
  fn rules_meta_wins() {
    let d = data_with("no-console", "https://example.com/no-console");
    assert_eq!(
      get_rule_url(Some("no-console"), &d).as_deref(),
      Some("https://example.com/no-console")
    );
  }

  #[test]
  fn core_fallback() {
    assert_eq!(
      get_rule_url(Some("no-unused-vars"), &FormatterData::default()).as_deref(),
      Some("https://eslint.org/docs/latest/rules/no-unused-vars")
    );
  }

  #[test]
  fn plugin_rule_without_meta_has_no_link() {
    assert_eq!(
      get_rule_url(Some("import-x/no-duplicates"), &FormatterData::default()),
      None
    );
  }

  #[test]
  fn missing_rule_id() {
    assert_eq!(get_rule_url(None, &FormatterData::default()), None);
  }

  #[test]
  fn empty_meta_url_falls_through_to_core() {
    let d = data_with("x", "");
    // "x" has no slash -> core fallback applies after the empty meta URL.
    assert_eq!(
      get_rule_url(Some("x"), &d).as_deref(),
      Some("https://eslint.org/docs/latest/rules/x")
    );
    // plugin rules get nothing
    let d2 = data_with("p/rule", "");
    assert_eq!(get_rule_url(Some("p/rule"), &d2), None);
  }
}
