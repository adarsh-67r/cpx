// Catppuccin palette and the styles every view renders with. "catppuccin" is
// the default (Mocha, the dark flavor).

use ratatui::style::{Color, Modifier, Style};

pub const THEMES: [&str; 3] = ["catppuccin", "auto", "light"];

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
    }
}
