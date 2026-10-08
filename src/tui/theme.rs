// Palettes and the styles every view renders with. All four Catppuccin
// flavors, plus a grayscale "mono". "catppuccin" (Mocha) is the default.

use ratatui::style::{Color, Modifier, Style};

pub const THEMES: [&str; 6] = ["catppuccin", "auto", "light", "macchiato", "frappe", "mono"];

/// The theme after name in THEMES, wrapping; T in the app cycles with it.
pub fn next(name: &str) -> &'static str {
    let i = THEMES.iter().position(|n| *n == name).unwrap_or(0);
    THEMES[(i + 1) % THEMES.len()]
}

#[derive(Clone, Copy)]
struct Palette {
    base: Color,
    text: Color,
    subtext: Color,
    muted: Color,
    surface: Color,
    mauve: Color,
    green: Color,
    red: Color,
    yellow: Color,
}

const MOCHA: Palette = Palette {
    base: Color::Rgb(0x1e, 0x1e, 0x2e),
    text: Color::Rgb(0xcd, 0xd6, 0xf4),
    subtext: Color::Rgb(0xa6, 0xad, 0xc8),
    muted: Color::Rgb(0x6c, 0x70, 0x86),
    surface: Color::Rgb(0x31, 0x32, 0x44),
    mauve: Color::Rgb(0xcb, 0xa6, 0xf7),
    green: Color::Rgb(0xa6, 0xe3, 0xa1),
    red: Color::Rgb(0xf3, 0x8b, 0xa8),
    yellow: Color::Rgb(0xf9, 0xe2, 0xaf),
};

const LATTE: Palette = Palette {
    base: Color::Rgb(0xef, 0xf1, 0xf5),
    text: Color::Rgb(0x4c, 0x4f, 0x69),
    subtext: Color::Rgb(0x6c, 0x6f, 0x85),
    muted: Color::Rgb(0x9c, 0xa0, 0xb0),
    surface: Color::Rgb(0xcc, 0xd0, 0xda),
    mauve: Color::Rgb(0x88, 0x39, 0xef),
    green: Color::Rgb(0x40, 0xa0, 0x2b),
    red: Color::Rgb(0xd2, 0x0f, 0x39),
    yellow: Color::Rgb(0xdf, 0x8e, 0x1d),
};

const fn rgb(c: u32) -> Color {
    Color::Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

const MACCHIATO: Palette = Palette {
    base: rgb(0x24273a),
    text: rgb(0xcad3f5),
    subtext: rgb(0xa5adcb),
    muted: rgb(0x6e738d),
    surface: rgb(0x363a4f),
    mauve: rgb(0xc6a0f6),
    green: rgb(0xa6da95),
    red: rgb(0xed8796),
    yellow: rgb(0xeed49f),
};

const FRAPPE: Palette = Palette {
    base: rgb(0x303446),
    text: rgb(0xc6d0f5),
    subtext: rgb(0xa5adce),
    muted: rgb(0x737994),
    surface: rgb(0x414559),
    mauve: rgb(0xca9ee6),
    green: rgb(0xa6d189),
    red: rgb(0xe78284),
    yellow: rgb(0xe5c890),
};

const MONO: Palette = Palette {
    base: rgb(0x121212),
    text: rgb(0xdcdcdc),
    subtext: rgb(0xaaaaaa),
    muted: rgb(0x7a7a7a),
    surface: rgb(0x2a2a2a),
    mauve: rgb(0xffffff),
    green: rgb(0xc8c8c8),
    red: rgb(0xffffff),
    yellow: rgb(0xe6e6e6),
};

/// Rank colors for a problem rating, from the Catppuccin accents in the
/// Codeforces order (gray, green, teal, blue, mauve, peach, red); 0 is unrated.
pub fn rating_color(rating: i64) -> Color {
    rgb(match rating {
        0 => 0x6c7086,
        r if r < 1200 => 0x9399b2,
        r if r < 1400 => 0xa6e3a1,
        r if r < 1600 => 0x94e2d5,
        r if r < 1900 => 0x89b4fa,
        r if r < 2100 => 0xcba6f7,
        r if r < 2400 => 0xfab387,
        _ => 0xf38ba8,
    })
}

/// Every style a view needs, plus the frame background.
pub struct Theme {
    pub bg: Color,
    pub title: Style,
    pub muted: Style,
    pub text: Style,
    pub sub: Style,
    pub bold: Style,
    pub select: Style,
    pub pass: Style,
    pub fail: Style,
    pub active: Style,
    pub pane_border: Style,
    #[allow(dead_code)]
    pub key: Style,
    /// False on mono, which keeps ratings gray.
    pub rating_colors: bool,
}

/// Builds the styles for the named theme. "auto" and an unknown name fall
/// back to the dark (Mocha) palette.
///
/// Note: "auto" does not detect the terminal's real background (that
/// needs an OSC query); it just uses the dark palette. Add detection if
/// someone actually asks for light-on-auto.
pub fn apply(name: &str) -> Theme {
    let p = match name {
        "light" => LATTE,
        "macchiato" => MACCHIATO,
        "frappe" => FRAPPE,
        "mono" => MONO,
        _ => MOCHA,
    };
    let bg = p.base;
    Theme {
        bg,
        title: Style::default().fg(p.mauve).bg(bg).add_modifier(Modifier::BOLD),
        muted: Style::default().fg(p.muted).bg(bg),
        text: Style::default().fg(p.text).bg(bg),
        sub: Style::default().fg(p.subtext).bg(bg),
        bold: Style::default().fg(p.text).bg(bg).add_modifier(Modifier::BOLD),
        select: Style::default().fg(p.text).bg(p.surface).add_modifier(Modifier::BOLD),
        pass: Style::default().fg(p.green).bg(bg),
        fail: Style::default().fg(p.red).bg(bg),
        active: Style::default().fg(p.yellow).bg(bg).add_modifier(Modifier::BOLD),
        pane_border: Style::default().fg(p.surface).bg(bg),
        key: Style::default().fg(p.mauve).bg(bg).add_modifier(Modifier::BOLD),
        rating_colors: name != "mono",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_cycles_through_every_theme() {
        let mut name = THEMES[0];
        for _ in 0..THEMES.len() {
            name = next(name);
        }
        assert_eq!(name, THEMES[0]);
        assert_eq!(next("unknown"), THEMES[1]);
    }
}
