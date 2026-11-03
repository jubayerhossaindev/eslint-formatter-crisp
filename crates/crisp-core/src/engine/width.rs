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

/// Skip an OSC escape sequence terminated by BEL (0x07) or ST (ESC `\`).
/// `chars` is positioned right after the leading ESC.
pub(crate) fn skip_osc_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    for oc in chars.by_ref() {
        if oc == '\x07' {
            break;
        }
        if oc == '\x1b' {
            chars.next(); // consume '\\'
            break;
        }
    }
}

/// Skip a CSI escape sequence terminated by a final byte in 0x40..=0x7E.
/// `chars` is positioned right after the leading ESC `[`; consumes the `[`.
pub(crate) fn skip_csi_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    chars.next(); // consume '['
    for oc in chars.by_ref() {
        if ('\x40'..='\x7E').contains(&oc) {
            break;
        }
    }
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
                skip_osc_sequence(&mut chars);
                continue;
            }
            // CSI: ESC [ ... final byte in 0x40..=0x7E
            if chars.peek() == Some(&'[') {
                skip_csi_sequence(&mut chars);
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

    /// display_width OSC escape with ST (ESC \\): skip path.
    #[test]
    fn display_width_osc_st() {
        assert_eq!(display_width("a\x1b]0;title\x1b\\b"), 2);
    }

    /// display_width OSC escape with BEL (0x07): skip path (line 27).
    #[test]
    fn display_width_osc_bel() {
        assert_eq!(display_width("a\x1b]0;title\x07b"), 2);
    }

    /// display_width bare ESC (not [ or ]): ESC skipped, X and b counted.
    #[test]
    fn display_width_bare_esc() {
        assert_eq!(display_width("a\x1bXb"), 3);
    }

    /// display_width CSI where b IS a final byte (0x40..=0x7E): CSI skipped,
    /// only 'a' counted.
    #[test]
    fn display_width_csi_no_final() {
        assert_eq!(display_width("a\x1b[b"), 1);
    }

    /// display_width_chars tab case.
    #[test]
    fn display_width_chars_tab() {
        assert_eq!(display_width_chars(&['a', '\t', 'b']), 4);
    }

    /// display_width with newline and carriage return: exercises
    /// `c == '\n' || c == '\r'` skip branch (line 60).
    #[test]
    fn display_width_skip_newlines() {
        assert_eq!(display_width("a\nb\r\rc"), 3);
    }
}
