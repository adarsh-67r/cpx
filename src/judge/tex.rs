// Turns TeX between $...$ and $$...$$ into plain text that reads well in a
// terminal: common symbols become Unicode, and simple powers and subscripts
// become superscript and subscript digits. Unknown forms stay in caret form.

use regex::Regex;
use std::sync::OnceLock;

/// Converts every $...$ and $$...$$ in s.
pub fn convert_math(s: &str) -> String {
    let (display, inline) = regexes();
    let s = display.replace_all(s, |c: &regex::Captures| tex_to_text(&c[1]));
    inline
        .replace_all(&s, |c: &regex::Captures| tex_to_text(&c[1]))
        .into_owned()
}

struct Patterns {
    display: Regex,
    inline: Regex,
    frac: Regex,
    sqrt: Regex,
    sup: Regex,
    sub: Regex,
    space: Regex,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        display: Regex::new(r"(?s)\$\$(.*?)\$\$").unwrap(),
        inline: Regex::new(r"\$([^$\n]*?)\$").unwrap(),
        frac: Regex::new(r"\\frac\{([^{}]*)\}\{([^{}]*)\}").unwrap(),
        sqrt: Regex::new(r"\\sqrt\{([^{}]*)\}").unwrap(),
        sup: Regex::new(r"\^\{([^{}]*)\}|\^(.)").unwrap(),
        sub: Regex::new(r"_\{([^{}]*)\}|_(.)").unwrap(),
        space: Regex::new(r"[ \t]+").unwrap(),
    })
}

fn regexes() -> (&'static Regex, &'static Regex) {
    let p = patterns();
    (&p.display, &p.inline)
}

/// TeX commands and their Unicode forms. Longer commands come first, so \leq
/// is not read as \le followed by q.
const SYMBOLS: &[(&str, &str)] = &[
    (r"\leq", "≤"), (r"\le", "≤"), (r"\geq", "≥"), (r"\ge", "≥"),
    (r"\neq", "≠"), (r"\ne", "≠"), (r"\approx", "≈"),
    (r"\cdots", "⋯"), (r"\ldots", "…"), (r"\dots", "…"),
    (r"\cdot", "·"), (r"\times", "×"), (r"\div", "÷"), (r"\pm", "±"),
    (r"\infty", "∞"), (r"\sum", "∑"), (r"\prod", "∏"),
    (r"\to", "→"), (r"\rightarrow", "→"), (r"\leftarrow", "←"),
    (r"\in", "∈"), (r"\notin", "∉"),
    (r"\subseteq", "⊆"), (r"\subset", "⊂"), (r"\cup", "∪"), (r"\cap", "∩"),
    (r"\forall", "∀"), (r"\exists", "∃"),
    (r"\lfloor", "⌊"), (r"\rfloor", "⌋"), (r"\lceil", "⌈"), (r"\rceil", "⌉"),
    (r"\pmod", "mod "), (r"\bmod", "mod "),
    (r"\mid", "|"), (r"\,", " "), (r"\;", " "), (r"\!", ""), (r"\quad", "  "),
    (r"\{", "{"), (r"\}", "}"), (r"\_", "_"), (r"\%", "%"),
    (r"\log", "log"), (r"\min", "min"), (r"\max", "max"), (r"\gcd", "gcd"),
];

const SUPERSCRIPT: &[(char, char)] = &[
    ('0', '⁰'), ('1', '¹'), ('2', '²'), ('3', '³'), ('4', '⁴'), ('5', '⁵'), ('6', '⁶'), ('7', '⁷'),
    ('8', '⁸'), ('9', '⁹'), ('+', '⁺'), ('-', '⁻'), ('n', 'ⁿ'), ('i', 'ⁱ'),
];

const SUBSCRIPT: &[(char, char)] = &[
    ('0', '₀'), ('1', '₁'), ('2', '₂'), ('3', '₃'), ('4', '₄'), ('5', '₅'), ('6', '₆'), ('7', '₇'),
    ('8', '₈'), ('9', '₉'), ('+', '₊'), ('-', '₋'), ('i', 'ᵢ'), ('j', 'ⱼ'), ('n', 'ₙ'),
];

/// Converts one TeX expression to plain text.
pub fn tex_to_text(tex: &str) -> String {
    let p = patterns();
    let mut s = tex.trim().to_string();
    s = p.frac.replace_all(&s, "($1)/($2)").into_owned();
    s = p.sqrt.replace_all(&s, "√($1)").into_owned();
    for (cmd, rep) in SYMBOLS {
        s = s.replace(cmd, rep);
    }
    s = p
        .sup
        .replace_all(&s, |c: &regex::Captures| {
            let inner = c.get(1).or(c.get(2)).map(|m| m.as_str()).unwrap_or("");
            to_script(inner, SUPERSCRIPT, '^')
        })
        .into_owned();
    s = p
        .sub
        .replace_all(&s, |c: &regex::Captures| {
            let inner = c.get(1).or(c.get(2)).map(|m| m.as_str()).unwrap_or("");
            to_script(inner, SUBSCRIPT, '_')
        })
        .into_owned();
    s = s.replace(['{', '}'], "");
    p.space.replace_all(&s, " ").into_owned()
}

/// Converts every character to its script form. If a character has none, the
/// caret or underscore form is kept, so no meaning is lost.
fn to_script(s: &str, table: &[(char, char)], marker: char) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        match table.iter().find(|(from, _)| *from == ch) {
            Some((_, to)) => out.push(*to),
            None => return format!("{marker}({s})"),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_and_scripts() {
        assert_eq!(convert_math("$n$"), "n");
        assert_eq!(convert_math("$1 \\le n \\le 10^5$"), "1 ≤ n ≤ 10⁵");
        assert_eq!(convert_math("$x_{10}$"), "x₁₀");
        assert_eq!(convert_math("$\\frac{a}{b}$"), "(a)/(b)");
        assert_eq!(convert_math("$\\sqrt{n}$"), "√(n)");
        assert_eq!(convert_math("$2^{n+1}$"), "2ⁿ⁺¹");
        assert_eq!(convert_math("plain text stays"), "plain text stays");
    }

    #[test]
    fn display_block_and_unsupported_scripts() {
        assert_eq!(convert_math("before\n$$\\sum a_i$$\nafter"), "before\n∑ aᵢ\nafter");
        assert_eq!(convert_math("$x^{k}$"), "x^(k)");
    }
}
