//! Terminal styling: ANSI SGR emission with three color levels, plus the
//! two summary-block gradients used by the reference output (`fruit` and
//! `cristal` — color stops verified in Phase 1, report §6.4 / finding F3-F4).
//!
//! Level semantics (chalk-compatible detection happens in the JS adapter):
//! - 0: no escape codes at all (NO_COLOR / piped output / TERM=dumb)
//! - 1: 16-color; gradients are downsampled to the nearest ANSI-16 color per
//!   character (byte-for-byte match of the codes observed in experiment s1)
//! - 2: 256-color gradients
//! - 3: 24-bit RGB gradients

pub const FRUIT_A: (u8, u8, u8) = (0xff, 0x4e, 0x50); // #ff4e50  (red)
pub const FRUIT_B: (u8, u8, u8) = (0xf9, 0xd4, 0x23); // #f9d423  (yellow)
pub const CRISTAL_A: (u8, u8, u8) = (0xbd, 0xff, 0xf3); // #bdfff3 (cyan)
pub const CRISTAL_B: (u8, u8, u8) = (0x4a, 0xc2, 0x9a); // #4ac29a (green)

/// Experimental color overrides (`CRISP_THEME` / `MO_THEME` env, v0.3.0+).
/// Every field is optional; unset fields keep the default palette. `mono`
/// disables the syntax-highlighter token colors entirely.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThemeOverride {
  pub fruit_a: Option<(u8, u8, u8)>,
  pub fruit_b: Option<(u8, u8, u8)>,
  pub cristal_a: Option<(u8, u8, u8)>,
  pub cristal_b: Option<(u8, u8, u8)>,
  pub mono: bool,
}

/// One-dark-pro approximations used by the mini highlighter at full color.
pub const C_KEYWORD: (u8, u8, u8) = (0xc6, 0x78, 0xdd);
pub const C_STRING: (u8, u8, u8) = (0x98, 0xc3, 0x79);
pub const C_NUMBER: (u8, u8, u8) = (0xd1, 0x9a, 0x66);
pub const C_COMMENT: (u8, u8, u8) = (0x5c, 0x63, 0x70);
pub const C_DEFAULT: (u8, u8, u8) = (0xab, 0xb2, 0xbf);
/// v0.5.3 UI/UX: operator color (one-dark-pro sky/cyan).
pub const C_OPERATOR: (u8, u8, u8) = (0x56, 0xb6, 0xc2);
/// v0.5.3 UI/UX: punctuation color (one-dark-pro muted gray-blue).
pub const C_PUNCTUATION: (u8, u8, u8) = (0x7f, 0x84, 0x8e);
/// v0.5.3 UI/UX: method-call color (one-dark-pro yellow).
pub const C_METHOD: (u8, u8, u8) = (0xe5, 0xc0, 0x7b);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Style {
  pub level: u8,
  pub theme: ThemeOverride,
}

fn wrap(level: u8, code: &str, text: &str) -> String {
  if level == 0 || text.is_empty() {
    return text.to_string();
  }
  format!("\x1b[{}m{}\x1b[22m", code, text)
}

fn fg(level: u8, code: &str, text: &str) -> String {
  if level == 0 || text.is_empty() {
    return text.to_string();
  }
  format!("\x1b[{}m{}\x1b[39m", code, text)
}

impl Style {
  pub fn new(level: u8) -> Self {
    Style {
      level: level.min(3),
      theme: ThemeOverride::default(),
    }
  }

  /// Style with experimental env-driven color overrides.
  pub fn with_theme(level: u8, theme: &ThemeOverride) -> Self {
    Style {
      level: level.min(3),
      theme: *theme,
    }
  }

  pub fn dim(&self, text: &str) -> String {
    wrap(self.level, "2", text)
  }
  pub fn bold(&self, text: &str) -> String {
    wrap(self.level, "1", text)
  }
  pub fn dim_bold(&self, text: &str) -> String {
    if self.level == 0 {
      return text.to_string();
    }
    format!("\x1b[2m\x1b[1m{}\x1b[22m\x1b[22m", text)
  }
  pub fn red(&self, text: &str) -> String {
    fg(self.level, "31", text)
  }
  pub fn yellow(&self, text: &str) -> String {
    fg(self.level, "33", text)
  }
  /// v0.5.3 UI/UX: cyan for rule-id (subtle, distinguishable from dim).
  pub fn cyan(&self, text: &str) -> String {
    fg(self.level, "36", text)
  }
  /// v0.5.3 UI/UX: bright green for auto-fixable marker (success signal).
  pub fn green_bright(&self, text: &str) -> String {
    fg(self.level, "92", text)
  }
  /// v0.5.3 UI/UX: bright white for prominent file-path basename.
  pub fn white_bright(&self, text: &str) -> String {
    fg(self.level, "97", text)
  }
  pub fn yellow_dim(&self, text: &str) -> String {
    if self.level == 0 {
      return text.to_string();
    }
    format!("\x1b[33m\x1b[2m{}\x1b[22m\x1b[39m", text)
  }
  pub fn red_bright(&self, text: &str) -> String {
    fg(self.level, "91", text)
  }
  pub fn yellow_bright(&self, text: &str) -> String {
    fg(self.level, "93", text)
  }

  /// Fixed-color token paint for the mini highlighter. At level 1 the
  /// palette uses fixed ANSI-16 codes chosen for readability (keywords
  /// magenta, strings green, numbers amber, comments gray); levels 2/3
  /// quantize the one-dark-pro RGB values. With the experimental `mono`
  /// theme the tokens stay uncolored.
  pub fn token(&self, kind: TokenKind, text: &str) -> String {
    if self.level == 0 || text.is_empty() || self.theme.mono {
      return text.to_string();
    }
    let fixed16 = |kind: TokenKind| -> u32 {
      match kind {
        TokenKind::Keyword => 35,
        TokenKind::String => 32,
        TokenKind::Number => 33,
        TokenKind::Comment => 90,
        TokenKind::Operator => 36,    // cyan
        TokenKind::Punctuation => 37, // white/gray
        TokenKind::Method => 33,      // yellow
        TokenKind::Default => 37,
      }
    };
    match self.level {
      3 => {
        let (r, g, b) = match kind {
          TokenKind::Keyword => C_KEYWORD,
          TokenKind::String => C_STRING,
          TokenKind::Number => C_NUMBER,
          TokenKind::Comment => C_COMMENT,
          TokenKind::Operator => C_OPERATOR,
          TokenKind::Punctuation => C_PUNCTUATION,
          TokenKind::Method => C_METHOD,
          TokenKind::Default => C_DEFAULT,
        };
        format!("\x1b[38;2;{};{};{}m{}\x1b[39m", r, g, b, text)
      }
      2 => {
        let (r, g, b) = match kind {
          TokenKind::Keyword => C_KEYWORD,
          TokenKind::String => C_STRING,
          TokenKind::Number => C_NUMBER,
          TokenKind::Comment => C_COMMENT,
          TokenKind::Operator => C_OPERATOR,
          TokenKind::Punctuation => C_PUNCTUATION,
          TokenKind::Method => C_METHOD,
          TokenKind::Default => C_DEFAULT,
        };
        format!("\x1b[38;5;{}m{}\x1b[39m", rgb_to_256(r, g, b), text)
      }
      _ => format!("\x1b[{}m{}\x1b[39m", fixed16(kind), text),
    }
  }

  /// Linear RGB interpolation across `text` between two color stops.
  /// Emits one SGR sequence per character (matching the reference's
  /// gradient-string rendering observed in Phase 1), then a single reset.
  pub fn gradient(&self, text: &str, from: (u8, u8, u8), to: (u8, u8, u8)) -> String {
    if self.level == 0 || text.is_empty() {
      return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len().max(1);
    let mut out = String::with_capacity(text.len() * 12);
    for (i, c) in chars.iter().enumerate() {
      let t = if n <= 1 {
        0.0
      } else {
        i as f32 / (n - 1) as f32
      };
      let r = from.0 as f32 + (to.0 as f32 - from.0 as f32) * t;
      let g = from.1 as f32 + (to.1 as f32 - from.1 as f32) * t;
      let b = from.2 as f32 + (to.2 as f32 - from.2 as f32) * t;
      let (r, g, b) = (r.round() as u8, g.round() as u8, b.round() as u8);
      match self.level {
        3 => out.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b)),
        2 => out.push_str(&format!("\x1b[38;5;{}m", rgb_to_256(r, g, b))),
        _ => out.push_str(&format!("\x1b[{}m", nearest_16_code(r, g, b))),
      }
      out.push(*c);
    }
    out.push_str("\x1b[39m");
    out
  }

  pub fn fruit(&self, text: &str) -> String {
    self.gradient(
      text,
      self.theme.fruit_a.unwrap_or(FRUIT_A),
      self.theme.fruit_b.unwrap_or(FRUIT_B),
    )
  }
  pub fn cristal(&self, text: &str) -> String {
    self.gradient(
      text,
      self.theme.cristal_a.unwrap_or(CRISTAL_A),
      self.theme.cristal_b.unwrap_or(CRISTAL_B),
    )
  }

  /// OSC 8 hyperlink with plaintext fallback.
  pub fn link(&self, text: &str, url: Option<&str>, links_enabled: bool) -> String {
    match (links_enabled, url) {
      (true, Some(u)) if self.level > 0 && !u.is_empty() => {
        format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", u, text)
      }
      _ => text.to_string(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
  Keyword,
  String,
  Number,
  Comment,
  /// v0.5.3 UI/UX: operators (=, +, ===, =>) — one-dark-pro cyan/sky.
  Operator,
  /// v0.5.3 UI/UX: punctuation ((, ), {, }, ;, .) — one-dark-pro muted.
  Punctuation,
  /// v0.5.3 UI/UX: method-call identifier after a dot (e.g. `.log`).
  Method,
  Default,
}

/// Distance to the nearest of the 16 base ANSI colors; returns the SGR
/// foreground code (30-37 for normal, 90-97 for bright).
pub fn nearest_16_code(r: u8, g: u8, b: u8) -> u32 {
  const BASE: [(u32, u32, u32); 16] = [
    (0, 0, 0),
    (128, 0, 0),
    (0, 128, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (0, 128, 128),
    (192, 192, 192),
    (128, 128, 128),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
  ];
  let (rr, gg, bb) = (r as u32, g as u32, b as u32);
  let mut best = 0usize;
  let mut best_d = u64::MAX;
  for (i, (cr, cg, cb)) in BASE.iter().enumerate() {
    let dr = rr as i64 - *cr as i64;
    let dg = gg as i64 - *cg as i64;
    let db = bb as i64 - *cb as i64;
    let d = (dr * dr + dg * dg + db * db) as u64;
    if d < best_d {
      best_d = d;
      best = i;
    }
  }
  if best < 8 {
    30 + best as u32
  } else {
    90 + (best - 8) as u32
  }
}

/// Map RGB to the xterm 256-color palette (6x6x6 cube + grayscale ramp).
pub fn rgb_to_256(r: u8, g: u8, b: u8) -> u32 {
  let gray_distance = |v: u8| {
    // distance to grayscale ramp levels 8*i - 8 (i in 1..=24)
    let levels: Vec<u32> = (1..=24).map(|i| 8 * i as u32 - 8).collect();
    levels
      .iter()
      .map(|l| {
        let d = v as i32 - *l as i32;
        (d * d) as u64
      })
      .min()
      .unwrap_or(u64::MAX)
  };
  let g_min = gray_distance(r).max(gray_distance(g)).max(gray_distance(b));
  let cube = |c: u8| -> u32 {
    // map 0..255 to 0..5 cube index
    let v = c as u32;
    if v < 48 {
      0
    } else if v < 115 {
      1
    } else {
      // `v >= 115` here, so the subtraction cannot underflow and the
      // division already yields a u32.
      (v - 35) / 40
    }
  };
  let (ri, gi, bi) = (cube(r), cube(g), cube(b));
  let cube_avg_d = {
    let cr = [0, 95, 135, 175, 215, 255][ri as usize] as i64;
    let cg = [0, 95, 135, 175, 215, 255][gi as usize] as i64;
    let cb = [0, 95, 135, 175, 215, 255][bi as usize] as i64;
    let dr = r as i64 - cr;
    let dg = g as i64 - cg;
    let db = b as i64 - cb;
    ((dr * dr + dg * dg + db * db) as u64) * 10 // slightly favor cube
  };
  if gray_distance(r) == u64::MAX || gray_distance(g) == u64::MAX || gray_distance(b) == u64::MAX {
    return 16;
  }
  if g_min < cube_avg_d {
    // grayscale: 232 + i where level = 8 + i*10, i in 0..=23
    let avg = (r as u32 + g as u32 + b as u32) / 3;
    let i = ((avg.saturating_sub(8)) / 10).min(23);
    232 + i
  } else {
    16 + 36 * ri + 6 * gi + bi
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn level0_is_plain() {
    let s = Style::new(0);
    assert_eq!(s.dim("x"), "x");
    assert_eq!(s.red("✖"), "✖");
    assert_eq!(s.gradient("───", FRUIT_A, FRUIT_B), "───");
    assert_eq!(s.fruit("──"), "──");
    assert_eq!(s.cristal("──"), "──");
    assert_eq!(s.token(TokenKind::Keyword, "const"), "const");
  }

  #[test]
  fn level1_basic_codes() {
    let s = Style::new(1);
    assert_eq!(s.dim("a"), "\x1b[2ma\x1b[22m");
    assert_eq!(s.red("✖"), "\x1b[31m✖\x1b[39m");
    assert_eq!(s.red_bright(" 5 errors"), "\x1b[91m 5 errors\x1b[39m");
    assert_eq!(s.green_bright("x"), "\x1b[92mx\x1b[39m");
  }

  #[test]
  fn gradient_stops_match_reference_codes() {
    // Verified against experiment s1 (Phase 1 report §6.2):
    // fruit endpoints render as 91 (bright-red) and 93 (bright-yellow).
    let s = Style::new(1);
    let out = s.fruit("─");
    assert!(out.contains("\x1b[91m"), "fruit start must be 91: {out:?}");
    let out2 = s.fruit(&"─".repeat(20));
    assert!(out2.contains("\x1b[93m"), "fruit end must be 93: {out2:?}");
    // cristal endpoints render as 97 (bright-white) then mid 36/96 range.
    let out3 = s.cristal(&"─".repeat(20));
    assert!(
      out3.contains("\x1b[97m"),
      "cristal start must be 97: {out3:?}"
    );
  }

  #[test]
  fn gradient_level3_is_rgb() {
    let s = Style::new(3);
    let out = s.fruit("──");
    assert!(out.contains("\x1b[38;2;255;78;80m"));
    assert!(out.contains("\x1b[38;2;249;212;35m"));
  }

  #[test]
  fn link_fallback() {
    let s = Style::new(1);
    assert_eq!(s.link("x", None, true), "x");
    assert_eq!(s.link("x", Some("file:///a.js"), false), "x");
    assert_eq!(
      s.link("x", Some("file:///a.js"), true),
      "\x1b]8;;file:///a.js\x1b\\x\x1b]8;;\x1b\\"
    );
    // links with colors disabled stay plain
    assert_eq!(Style::new(0).link("x", Some("file:///a.js"), true), "x");
  }

  #[test]
  fn rgb256_corners() {
    assert_eq!(rgb_to_256(0, 0, 0), 16);
    assert_eq!(rgb_to_256(255, 255, 255), 231);
    assert_eq!(rgb_to_256(0, 255, 0), 46);
  }
}

#[cfg(test)]
mod theme_tests {
  use super::*;

  #[test]
  fn fruit_uses_override_stops() {
    let default = Style::new(3).fruit("--");
    let theme = ThemeOverride {
      fruit_a: Some((0x00, 0x11, 0x22)),
      ..Default::default()
    };
    let themed = Style::with_theme(3, &theme).fruit("--");
    assert_ne!(default, themed, "override changes the gradient");
    assert!(
      themed.contains("\x1b[38;2;0;17;34m"),
      "first stop is the custom color: {themed:?}"
    );
    // partial override keeps the untouched stop
    assert!(
      themed.contains("\x1b[38;2;249;212;35m"),
      "fruit_b still default"
    );
  }

  #[test]
  fn mono_disables_token_colors() {
    let plain = Style::new(1).token(TokenKind::Keyword, "const");
    let mono = Style::with_theme(
      1,
      &ThemeOverride {
        mono: true,
        ..Default::default()
      },
    )
    .token(TokenKind::Keyword, "const");
    assert!(plain.starts_with("\x1b["), "default paints tokens");
    assert_eq!(mono, "const", "mono leaves tokens uncolored");
  }

  #[test]
  fn default_theme_matches_historical_bytes() {
    let themed = Style::with_theme(1, &ThemeOverride::default());
    assert_eq!(themed.fruit("x"), Style::new(1).fruit("x"));
    assert_eq!(themed.cristal("y"), Style::new(1).cristal("y"));
  }
}
