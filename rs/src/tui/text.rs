// Small text helpers shared by every view: one logical, single-style line
// that gets word-wrapped to a pane's width right before it is drawn.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

#[derive(Clone)]
pub struct LLine {
    pub text: String,
    pub style: Style,
}

impl LLine {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        LLine { text: text.into(), style }
    }
}

/// Greedy word-wrap. Existing newlines are hard breaks; a single word longer
/// than width is broken at the character boundary. A line that already fits
/// is kept verbatim, so column padding and indents survive.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.chars().count() <= width {
            out.push(paragraph.to_string());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let mut remaining = word.to_string();
            loop {
                let sep = usize::from(!line.is_empty());
                let room = width.saturating_sub(line.chars().count() + sep);
                if remaining.chars().count() <= room {
                    if !line.is_empty() {
                        line.push(' ');
                    }
                    line.push_str(&remaining);
                    break;
                }
                if line.is_empty() {
                    let chars: Vec<char> = remaining.chars().collect();
                    let split = width.min(chars.len());
                    out.push(chars[..split].iter().collect::<String>());
                    remaining = chars[split..].iter().collect();
                    if remaining.is_empty() {
                        break;
                    }
                    continue;
                }
                out.push(std::mem::take(&mut line));
            }
        }
        out.push(line);
    }
    out
}

/// Wraps every logical line and keeps its style, so a pane can slice the
/// result by a scroll offset.
pub fn wrap_llines(lines: &[LLine], width: usize) -> Vec<LLine> {
    let mut out = Vec::new();
    for l in lines {
        for physical in wrap(&l.text, width) {
            out.push(LLine::new(physical, l.style));
        }
    }
    out
}

/// Shortens s to n chars, adding an ellipsis when it cuts.
pub fn truncate(s: &str, n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        return s.to_string();
    }
    if n == 1 {
        return "…".to_string();
    }
    let mut out: String = chars[..n - 1].iter().collect();
    out.push('…');
    out
}

/// Centers s in a field of width columns, measured in chars. Content wider
/// than width is left as-is.
pub fn center(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let total = width - len;
    let left = total / 2;
    let right = total - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

pub fn clamp(v: i64, lo: i64, hi: i64) -> i64 {
    if hi < lo {
        return lo;
    }
    v.clamp(lo, hi)
}

pub fn llines_to_ratatui(lines: &[LLine]) -> Vec<Line<'static>> {
    lines.iter().map(|l| Line::from(Span::styled(l.text.clone(), l.style))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_fitting_lines_and_breaks_long_ones() {
        assert_eq!(wrap("  a     b", 20), vec!["  a     b"]);
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("abcdefgh", 3), vec!["abc", "def", "gh"]);
        assert_eq!(wrap("x\n\ny", 5), vec!["x", "", "y"]);
    }
}
