//! Minimal pluralization for the three labels the formatter uses.
//! Equivalent to the `plur` crate for these inputs (append "s").

pub fn plural(word: &str, count: u32) -> String {
  if count == 1 {
    word.to_string()
  } else {
    format!("{}s", word)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_plur_for_formatter_labels() {
    assert_eq!(plural("error", 0), "errors");
    assert_eq!(plural("error", 1), "error");
    assert_eq!(plural("error", 5), "errors");
    assert_eq!(plural("warning", 1), "warning");
    assert_eq!(plural("warning", 2), "warnings");
    assert_eq!(plural("auto-fixable", 1), "auto-fixable");
    assert_eq!(plural("auto-fixable", 4), "auto-fixables");
  }
}
