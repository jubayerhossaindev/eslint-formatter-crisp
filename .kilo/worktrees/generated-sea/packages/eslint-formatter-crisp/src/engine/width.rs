//! Display-width helpers built on `unicode-width`.
//!
//! Improvements over the reference implementation (documented in the Phase 1
//! report, §8.2): ALL tab characters are expanded to two spaces (the reference
//! only expanded leading tabs, causing mid-line tab drift), and column math
//! uses display width (CJK-aware) rather than raw character counts.

/// Expand every tab to two spaces.
pub fn tabs_to_spaces(s: &str) -> String {
  s.replace('\t', "  ")
}

/// Visible display width of a string, ignoring ANSI escape sequences and
/// counting tabs as two columns (after the caller has normally already
/// expanded them). Control characters contribute zero width.
pub fn display_width(s: &str) -> usize {
  let mut total = 0usize;
  let mut chars = s.chars().peekable();
  while let Some(c) = chars.next() {
    if c == '\x1b' {
      // Skip a CSI/OSC escape sequence if present. We control where
      // escapes are emitted, so this is a defensive fallback only.
      if chars.peek() == Some(&']') {
        // OSC 8: skip until BEL (0x07) or ST (ESC \)
        for oc in chars.by_ref() {
          if oc == '\x07' {
            break;
          }
          if oc == '\x1b' {
            chars.next(); // consume '\\'
            break;
          }
        }
        continue;
      }
      // CSI: ESC [ ... final byte in 0x40..=0x7E
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
    if c == '\n' || c == '\r' {
      continue;
    }
    if c == '\t' {
      total += 2;
      continue;
    }
    total += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
  }
  total
}

/// Visible display width over an already-collected char slice.
pub fn display_width_chars(chars: &[char]) -> usize {
  chars
    .iter()
    .map(|c| {
      if *c == '\t' {
        2
      } else {
        unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0)
      }
    })
    .sum()
}

/// Clamp a 1-based column to a usable slice bound for a line of `len` chars.
pub fn col_prefix_bound(col: u32, char_len: usize) -> usize {
  if col == 0 {
    return 0;
  }
  let c = (col - 1) as usize;
  c.min(char_len)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tabs_all_expanded() {
    assert_eq!(tabs_to_spaces("\tfoo\tbar"), "  foo  bar");
    assert_eq!(tabs_to_spaces("a\tb"), "a  b");
    assert_eq!(tabs_to_spaces("no tabs"), "no tabs");
  }

  #[test]
  fn widths() {
    assert_eq!(display_width("hello"), 5);
    assert_eq!(display_width("你好"), 4); // CJK wide chars
    assert_eq!(display_width("\u{1F525}"), 2); // fire emoji is wide
    assert_eq!(display_width("a\tb"), 4); // tab counts as two columns
    assert_eq!(display_width("\x1b[2m─\x1b[22m"), 1); // ANSI skipped
  }

  #[test]
  fn col_bound_clamps() {
    assert_eq!(col_prefix_bound(1, 5), 0);
    assert_eq!(col_prefix_bound(6, 5), 5, "out-of-range column clamps");
    assert_eq!(col_prefix_bound(3, 5), 2);
    assert_eq!(col_prefix_bound(0, 5), 0);
  }
}
