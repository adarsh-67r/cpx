// Judge access: fetching pages and API responses, and parsing them. Parsers
// take text so they can be tested against saved pages.

pub mod atcoder;
pub mod codeforces;
pub mod cses;
pub mod dispatch;
pub mod statement;
pub mod tex;

use anyhow::{anyhow, Result};
use scraper::ElementRef;
use std::time::Duration;

pub const USER_AGENT: &str = "Mozilla/5.0 (compatible; cpx/0.1)";

/// GET a URL and return the body. Non-200 responses are errors.
pub fn get(url: &str) -> Result<String> {
    get_with(url, None)
}

/// GET with an optional PHPSESSID cookie (used for CSES progress).
pub fn get_with(url: &str, php_sessid: Option<&str>) -> Result<String> {
    let mut req = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept-Language", "en")
        .timeout(Duration::from_secs(60));
    if let Some(s) = php_sessid.filter(|s| !s.is_empty()) {
        req = req.set("Cookie", &format!("PHPSESSID={s}"));
    }
    let resp = req.call().map_err(|e| anyhow!("GET {url}: {e}"))?;
    Ok(resp.into_string()?)
}

/// Text of an element, with <br> turned into a newline. Used for <pre> blocks,
/// where line breaks matter.
pub fn pre_text(el: ElementRef<'_>) -> String {
    let mut out = String::new();
    push_text(el, &mut out);
    out.replace("\r\n", "\n").trim_matches('\n').to_string()
}

fn push_text(el: ElementRef<'_>, out: &mut String) {
    for child in el.children() {
        match child.value() {
            scraper::Node::Text(t) => out.push_str(t),
            scraper::Node::Element(e) if e.name() == "br" => out.push('\n'),
            scraper::Node::Element(_) => {
                if let Some(inner) = ElementRef::wrap(child) {
                    push_text(inner, out);
                }
            }
            _ => {}
        }
    }
}

/// Whitespace runs become single spaces; the ends are trimmed.
pub fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
pub fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name))
        .unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}
