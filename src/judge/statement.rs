// Statements as typed blocks, the way cpos keeps them. The parsers say what
// each piece is (title, metadata, heading, paragraph, formula, item, code),
// and the terminal view decides the spacing.

use crate::judge::{get, pre_text, tidy, tex::convert_math};
use crate::problem::Problem;
use anyhow::{anyhow, Result};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

pub const TITLE: &str = "title";
pub const META: &str = "meta";
pub const HEADING: &str = "heading";
pub const PARAGRAPH: &str = "paragraph";
pub const MATH: &str = "math";
pub const ITEM: &str = "item";
pub const CODE: &str = "code";

/// One piece of a statement. The JSON keys match the Go version's cache files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub kind: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ordered: bool,
}

/// Fetches the statement for p, parsed for its platform.
pub fn fetch(p: &Problem) -> Result<Vec<Block>> {
    let page = get(&p.url)?;
    match p.platform.as_str() {
        "codeforces" => Ok(parse_codeforces(&page)),
        "cses" => Ok(parse_cses(&page)),
        "atcoder" => Ok(parse_atcoder(&page)),
        other => Err(anyhow!("no statement parser for platform {other:?}")),
    }
}

pub fn parse_codeforces(page: &str) -> Vec<Block> {
    let doc = Html::parse_document(page);
    let root = doc.select(&sel(".problem-statement")).next();
    match root {
        Some(r) => walk_root(r),
        None => Vec::new(),
    }
}

pub fn parse_cses(page: &str) -> Vec<Block> {
    let doc = Html::parse_document(page);
    let Some(content) = doc.select(&sel(".content")).next() else {
        return Vec::new();
    };
    let mut b = Builder::default();
    for child in content.children() {
        // Everything from the Example heading on is covered by the samples.
        if let Some(el) = ElementRef::wrap(child) {
            if el.value().attr("id") == Some("example") {
                break;
            }
        }
        walk(child, &mut b);
    }
    b.flush(PARAGRAPH);
    let mut blocks = finish(b.blocks);
    if let Some(h1) = doc.select(&sel("h1")).next() {
        let title = tidy(&h1.text().collect::<String>());
        if !title.is_empty() {
            blocks.insert(0, Block { kind: TITLE.into(), text: title, ordered: false });
        }
    }
    blocks
}

pub fn parse_atcoder(page: &str) -> Vec<Block> {
    let doc = Html::parse_document(page);
    let Some(root) = doc.select(&sel("#task-statement")).next() else {
        return Vec::new();
    };
    match root.select(&sel(".lang-en")).next() {
        Some(en) => walk_root(en),
        None => walk_root(root),
    }
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("valid selector")
}

fn walk_root(root: ElementRef<'_>) -> Vec<Block> {
    let mut b = Builder::default();
    walk(*root, &mut b);
    b.flush(PARAGRAPH);
    finish(b.blocks)
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    cur: String,
}

impl Builder {
    fn flush(&mut self, kind: &str) {
        let text = tidy(&self.cur);
        self.cur.clear();
        if !text.is_empty() {
            self.blocks.push(Block { kind: kind.into(), text, ordered: false });
        }
    }
}

const BLOCK_TAGS: &[&str] = &[
    "p", "div", "ul", "ol", "table", "tr", "section", "header", "dt", "dd", "center", "figure",
];

const PROPERTY_CLASSES: &[&str] = &["time-limit", "memory-limit", "input-file", "output-file"];

fn walk(node: ego_tree::NodeRef<'_, scraper::Node>, b: &mut Builder) {
    match node.value() {
        scraper::Node::Text(t) => b.cur.push_str(t),
        scraper::Node::Element(_) => {
            if let Some(el) = ElementRef::wrap(node) {
                element(el, b);
            }
        }
        _ => {}
    }
}

fn walk_children(el: ElementRef<'_>, b: &mut Builder) {
    for child in el.children() {
        walk(child, b);
    }
}

fn element(el: ElementRef<'_>, b: &mut Builder) {
    let name = el.value().name();
    let class = el.value().attr("class").unwrap_or("");
    let classes: Vec<&str> = class.split_whitespace().collect();

    if matches!(name, "style" | "head" | "noscript") {
        return;
    }
    if classes.iter().any(|c| c.contains("MathJax") || c.contains("mjx") || *c == "sr-only" || *c == "sample-tests") {
        return;
    }

    if name == "script" {
        let t = el.value().attr("type").unwrap_or("");
        if t.starts_with("math/tex") {
            let tex: String = el.text().collect::<String>().trim().to_string();
            emit_math(b, &tex, t.contains("mode=display"));
        }
        return;
    }

    // CSES wraps math in a KaTeX span whose annotation holds the TeX source.
    if classes.iter().any(|c| c.contains("math")) {
        if let Some(tex) = annotation_of(el) {
            emit_math(b, &tex, classes.contains(&"math-display"));
            return;
        }
    }

    match name {
        "br" => b.cur.push(' '),
        "pre" => {
            b.flush(PARAGRAPH);
            let code = pre_text(el);
            if !code.trim().is_empty() {
                b.blocks.push(Block { kind: CODE.into(), text: code, ordered: false });
            }
        }
        "li" => {
            b.flush(PARAGRAPH);
            walk_children(el, b);
            let ordered = el.parent().and_then(ElementRef::wrap).is_some_and(|p| p.value().name() == "ol");
            let text = tidy(&b.cur);
            b.cur.clear();
            if !text.is_empty() {
                b.blocks.push(Block { kind: ITEM.into(), text, ordered });
            }
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            b.flush(PARAGRAPH);
            walk_children(el, b);
            b.flush(HEADING);
        }
        _ if classes.contains(&"section-title") => {
            b.flush(PARAGRAPH);
            walk_children(el, b);
            b.flush(HEADING);
        }
        _ if classes.contains(&"title") && parent_has_class(el, "header") => {
            b.flush(PARAGRAPH);
            walk_children(el, b);
            b.flush(TITLE);
        }
        _ if classes.first().is_some_and(|c| PROPERTY_CLASSES.contains(c)) => {
            b.flush(PARAGRAPH);
            emit_property(el, b);
        }
        _ if BLOCK_TAGS.contains(&name) => {
            b.flush(PARAGRAPH);
            walk_children(el, b);
            b.flush(PARAGRAPH);
        }
        _ => walk_children(el, b),
    }
}

/// Display math becomes its own block. Inline math joins the running paragraph
/// between $ signs, and convert_math turns it into readable text later.
fn emit_math(b: &mut Builder, tex: &str, display: bool) {
    if display {
        b.flush(PARAGRAPH);
        if !tex.is_empty() {
            b.blocks.push(Block { kind: MATH.into(), text: tex.into(), ordered: false });
        }
    } else {
        b.cur.push_str(&format!("${tex}$"));
    }
}

/// "time limit per test: 1 second", from the property title and its value.
fn emit_property(el: ElementRef<'_>, b: &mut Builder) {
    let title_sel = sel(".property-title");
    let Some(title) = el.select(&title_sel).next() else { return };
    let prop = tidy(&title.text().collect::<String>());
    let full = tidy(&el.text().collect::<String>());
    let value = full.strip_prefix(&prop).unwrap_or(&full).trim().to_string();
    if !prop.is_empty() && !value.is_empty() {
        b.blocks.push(Block { kind: META.into(), text: format!("{prop}: {value}"), ordered: false });
    }
}

fn annotation_of(el: ElementRef<'_>) -> Option<String> {
    let ann = sel("annotation");
    el.select(&ann).next().map(|a| a.text().collect::<String>().trim().to_string())
}

fn parent_has_class(el: ElementRef<'_>, class: &str) -> bool {
    el.parent()
        .and_then(ElementRef::wrap)
        .is_some_and(|p| p.value().attr("class").unwrap_or("").split_whitespace().any(|c| c == class))
}

/// Drops empty blocks, turns TeX into readable text, and removes repeats.
fn finish(input: Vec<Block>) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for mut bl in input {
        if bl.text.trim().is_empty() {
            continue;
        }
        if bl.kind == MATH {
            bl.text = convert_math(&format!("$${}$$", bl.text));
        } else if bl.kind != CODE {
            bl.text = convert_math(&bl.text);
        }
        if out.last() == Some(&bl) {
            continue;
        }
        out.push(bl);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::fixture;

    fn find<'a>(bs: &'a [Block], kind: &str, contains: &str) -> Option<&'a Block> {
        bs.iter().find(|b| b.kind == kind && b.text.contains(contains))
    }

    #[test]
    fn codeforces_blocks() {
        let bs = parse_codeforces(&fixture("cf_4A.html"));
        assert_eq!(find(&bs, TITLE, "Watermelon").unwrap().text, "A. Watermelon");
        assert!(find(&bs, META, "time limit per test: 1 second").is_some());
        assert!(find(&bs, META, "memory limit per test: 64 megabytes").is_some());
        assert!(find(&bs, HEADING, "Input").is_some());
        assert!(find(&bs, PARAGRAPH, "Pete and his friend Billy").is_some());
        assert!(bs.iter().all(|b| !b.text.contains('<')), "no markup in blocks");
    }

    #[test]
    fn codeforces_header_is_first_and_ordered() {
        let bs = parse_codeforces(&fixture("cf_4A.html"));
        let kinds: Vec<&str> = bs.iter().take(5).map(|b| b.kind.as_str()).collect();
        assert_eq!(kinds, vec![TITLE, META, META, META, META]);
    }

    #[test]
    fn cses_blocks_have_title_and_no_example() {
        let bs = parse_cses(&fixture("cses_1068.html"));
        assert_eq!(bs[0].kind, TITLE);
        assert_eq!(bs[0].text, "Weird Algorithm");
        assert!(find(&bs, PARAGRAPH, "the sequence for").is_some());
        assert!(find(&bs, PARAGRAPH, "Example").is_none());
    }

    #[test]
    fn atcoder_blocks_are_present() {
        let bs = parse_atcoder(&fixture("atcoder_abc001_1.html"));
        assert!(!bs.is_empty());
        assert!(bs.iter().all(|b| !b.text.contains('<')));
    }

    #[test]
    fn math_is_math_block_or_inline_text() {
        let page = r#"<div class="problem-statement"><p>Given <script type="math/tex">n</script> items.</p><script type="math/tex; mode=display">\sum a_i</script><p>Done.</p></div>"#;
        let bs = parse_codeforces(page);
        assert!(find(&bs, PARAGRAPH, "Given n items.").is_some(), "{bs:?}");
        assert_eq!(find(&bs, MATH, "∑ aᵢ").unwrap().kind, MATH);
    }

    #[test]
    fn code_keeps_line_breaks_and_empty_blocks_go() {
        let bs = parse_codeforces(r#"<div class="problem-statement"><pre>a b<br>c d</pre><div></div><p>   </p></div>"#);
        assert_eq!(bs.len(), 1);
        assert_eq!(bs[0].kind, CODE);
        assert_eq!(bs[0].text, "a b\nc d");
    }

    #[test]
    fn block_json_matches_go_keys() {
        let b = Block { kind: "item".into(), text: "x".into(), ordered: true };
        let j = serde_json::to_string(&b).unwrap();
        assert_eq!(j, r#"{"kind":"item","text":"x","ordered":true}"#);
    }
}
