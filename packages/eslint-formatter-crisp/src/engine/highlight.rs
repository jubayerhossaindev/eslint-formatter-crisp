//! Minimal, dependency-free syntax highlighter for source excerpts.
//!
//! This is an INTENTIONAL, documented approximation of the reference's shiki
//! rendering (Phase 1 report §6.2): it is a per-line tokenizer that colors
//! keywords, strings, comments, and numbers with one-dark-pro-inspired
//! palette values. It does not reproduce shiki's TextMate tokenization
//! exactly and has no multi-line state. Trade-off: zero grammars to ship,
//! zero cold-start cost, deterministic output.

use crate::engine::style::{Style, TokenKind};

const KEYWORDS: &[&str] = &[
  "abstract",
  "any",
  "as",
  "async",
  "await",
  "boolean",
  "break",
  "case",
  "catch",
  "class",
  "const",
  "continue",
  "debugger",
  "declare",
  "default",
  "delete",
  "do",
  "else",
  "enum",
  "export",
  "extends",
  "false",
  "finally",
  "for",
  "from",
  "function",
  "get",
  "if",
  "implements",
  "import",
  "in",
  "instanceof",
  "interface",
  "keyof",
  "let",
  "namespace",
  "never",
  "new",
  "null",
  "number",
  "object",
  "of",
  "private",
  "protected",
  "public",
  "readonly",
  "return",
  "satisfies",
  "set",
  "static",
  "string",
  "super",
  "switch",
  "symbol",
  "this",
  "throw",
  "true",
  "try",
  "type",
  "typeof",
  "undefined",
  "unknown",
  "var",
  "void",
  "while",
  "with",
  "yield",
];

fn is_ident_start(c: char) -> bool {
  c.is_alphabetic() || c == '_' || c == '$'
}
fn is_ident_part(c: char) -> bool {
  c.is_alphanumeric() || c == '_' || c == '$'
}

/// Tokenize one line and return the styled string. Never panics on any input.
/// Unterminated strings/comments are colored to end-of-line (safe fallback).
pub fn highlight_line(line: &str, style: &Style) -> String {
  if style.level == 0 || line.is_empty() {
    return line.to_string();
  }
  let chars: Vec<char> = line.chars().collect();
  let mut out = String::with_capacity(line.len() + 64);
  let mut i = 0usize;
  let n = chars.len();

  while i < n {
    let c = chars[i];

    // Comments
    if c == '/' && i + 1 < n && chars[i + 1] == '/' {
      let rest: String = chars[i..].iter().collect();
      out.push_str(&style.token(TokenKind::Comment, &rest));
      break;
    }
    if c == '/' && i + 1 < n && chars[i + 1] == '*' {
      // single-line block comment only (no cross-line state)
      let mut j = i + 2;
      while j + 1 < n && !(chars[j] == '*' && chars[j + 1] == '/') {
        j += 1;
      }
      let end = if j + 1 < n { (j + 2).min(n) } else { n };
      let text: String = chars[i..end].iter().collect();
      out.push_str(&style.token(TokenKind::Comment, &text));
      i = end;
      continue;
    }

    // Strings (single, double, template) — unterminated-safe
    if c == '\'' || c == '"' || c == '`' {
      let quote = c;
      let mut j = i + 1;
      while j < n {
        if chars[j] == '\\' {
          j += 2;
          continue;
        }
        if chars[j] == quote {
          j += 1;
          break;
        }
        j += 1;
      }
      let text: String = chars[i..j.min(n)].iter().collect();
      out.push_str(&style.token(TokenKind::String, &text));
      i = j.min(n);
      continue;
    }

    // Numbers (int/float, incl. leading digit after word boundary)
    if c.is_ascii_digit() {
      let mut j = i;
      while j < n && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
        j += 1;
      }
      let text: String = chars[i..j].iter().collect();
      out.push_str(&style.token(TokenKind::Number, &text));
      i = j;
      continue;
    }

    // Identifiers / keywords
    if is_ident_start(c) {
      let mut j = i;
      while j < n && is_ident_part(chars[j]) {
        j += 1;
      }
      let text: String = chars[i..j].iter().collect();
      // v0.5.3 UI/UX: method-call detection — if the previous non-space char
      // was a dot (`.`), this identifier is a method name (e.g. `.log`,
      // `.forEach`), render it with the Method token color.
      let prev_non_space = (0..i)
        .rev()
        .find(|&k| !chars[k].is_whitespace())
        .map(|k| chars[k]);
      let kind = if KEYWORDS.contains(&text.as_str()) {
        TokenKind::Keyword
      } else if prev_non_space == Some('.') {
        TokenKind::Method
      } else {
        TokenKind::Default
      };
      out.push_str(&style.token(kind, &text));
      i = j;
      continue;
    }

    // v0.5.3 UI/UX: operators (=, +, ===, =>, <, >, !, etc.) — multi-char
    // runs of operator chars get the Operator color.
    if is_operator_char(c) {
      let mut j = i;
      while j < n && is_operator_char(chars[j]) {
        j += 1;
      }
      let text: String = chars[i..j].iter().collect();
      out.push_str(&style.token(TokenKind::Operator, &text));
      i = j;
      continue;
    }

    // v0.5.3 UI/UX: punctuation ((, ), {, }, ;, ,, :, etc.) — distinct
    // muted color so structure pops visually without competing with
    // identifiers.
    if is_punctuation_char(c) {
      let mut j = i;
      while j < n && is_punctuation_char(chars[j]) {
        j += 1;
      }
      let text: String = chars[i..j].iter().collect();
      out.push_str(&style.token(TokenKind::Punctuation, &text));
      i = j;
      continue;
    }

    // Everything else (whitespace, misc) plain default.
    out.push_str(&style.token(TokenKind::Default, &c.to_string()));
    i += 1;
  }

  out
}

/// v0.5.3 UI/UX: operator characters (assignment, arithmetic, comparison,
/// logical, fat-arrow). Grouped so multi-char operators like `===`, `=>`,
/// `!==` render as a single styled run.
fn is_operator_char(c: char) -> bool {
  matches!(
    c,
    '=' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '!' | '?' | '&' | '|' | '^' | '~' | ':'
  )
  // NOTE: `/` is also the comment opener, but the comment check runs first
  // so we only reach here when it's NOT a comment. `/` as an operator
  // (division) gets the operator color.
}

/// v0.5.3 UI/UX: punctuation characters (brackets, braces, parens, comma,
/// semicolon, dot). The dot is included here too — when an identifier
/// follows it, the identifier becomes a Method token (see above), so the
/// dot itself reads as punctuation linking them.
fn is_punctuation_char(c: char) -> bool {
  matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ';' | ',' | '.')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn never_panics_on_weird_input() {
    let s = Style::new(1);
    for line in [
      "",
      "   ",
      "'",
      "\"",
      "`",
      "/*",
      "*/",
      "//",
      "\\",
      "a'b\"c`d",
      "'unterminated",
      "/* unterminated",
      "1..2..3",
      "café",
      "你好 world",
    ] {
      let _ = highlight_line(line, &s);
    }
  }

  #[test]
  fn keywords_and_strings_colored_level1() {
    let s = Style::new(1);
    let out = highlight_line("const x = \"hi\";", &s);
    // Level-1 palette: keywords magenta (35), strings green (32).
    assert!(
      out.contains("\x1b[35mconst"),
      "keyword code: {:?}",
      out.escape_debug().to_string()
    );
    assert!(
      out.contains("\x1b[32m\"hi\""),
      "string code: {:?}",
      out.escape_debug().to_string()
    );
  }

  #[test]
  fn comment_to_eol() {
    let s = Style::new(1);
    let out = highlight_line("foo(); // trailing", &s);
    assert!(out.contains("trailing"));
    let plain = Style::new(0);
    assert_eq!(highlight_line("const a;", &plain), "const a;");
  }
}
