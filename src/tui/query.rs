// The Problems filter: one line of space-separated terms, all of which must
// match. `@cf` / `@cses` / `@atcoder` picks a judge, `1200-1600`, `1600+`,
// and `-1400` bound the rating, `#dp` requires a tag (`#dp,greedy` any of
// them), and other words match the id, name, or tags.

use crate::problem::Problem;

#[derive(Debug, Default, PartialEq)]
pub struct Query {
    pub platform: Option<&'static str>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    /// Each entry must match; within an entry, any tag will do.
    pub tags: Vec<Vec<String>>,
    pub words: Vec<String>,
}

pub const PLATFORMS: [(&str, &str); 3] = [("@cf", "codeforces"), ("@cses", "cses"), ("@atcoder", "atcoder")];

fn platform(token: &str) -> Option<&'static str> {
    match token {
        "@cf" | "@codeforces" => Some("codeforces"),
        "@cses" => Some("cses"),
        "@ac" | "@atcoder" => Some("atcoder"),
        _ => None,
    }
}

pub fn parse(line: &str) -> Query {
    let mut q = Query::default();
    for token in line.to_lowercase().split_whitespace() {
        if let Some(p) = platform(token) {
            q.platform = Some(p);
        } else if let Some(tags) = token.strip_prefix('#') {
            let any: Vec<String> = tags.split(',').filter(|t| !t.is_empty()).map(|t| t.replace('_', " ")).collect();
            if !any.is_empty() {
                q.tags.push(any);
            }
        } else if let Some(n) = token.strip_suffix('+').and_then(|n| n.parse().ok()) {
            q.min = Some(n);
        } else if let Some(n) = token.strip_prefix('-').and_then(|n| n.parse().ok()) {
            q.max = Some(n);
        } else if let Some((lo, hi)) = token.split_once('-').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))) {
            q.min = Some(lo);
            q.max = Some(hi);
        } else {
            q.words.push(token.to_string());
        }
    }
    q
}

impl Query {
    pub fn matches(&self, p: &Problem) -> bool {
        if self.platform.is_some_and(|pl| pl != p.platform) {
            return false;
        }
        // Unrated problems (CSES, most AtCoder) drop out once a range is set.
        if (self.min.is_some() || self.max.is_some()) && p.rating == 0 {
            return false;
        }
        if self.min.is_some_and(|m| p.rating < m) || self.max.is_some_and(|m| p.rating > m) {
            return false;
        }
        let tags: Vec<String> = p.tags.iter().map(|t| t.to_lowercase()).collect();
        if !self.tags.iter().all(|any| any.iter().any(|t| tags.iter().any(|pt| pt.contains(t.as_str())))) {
            return false;
        }
        self.words.iter().all(|w| {
            p.id.to_lowercase().contains(w.as_str()) || p.name.to_lowercase().contains(w.as_str()) || tags.iter().any(|t| t.contains(w.as_str()))
        })
    }
}

/// The filter line with its platform term replaced by the next judge
/// (all → Codeforces → CSES → AtCoder → all).
pub fn cycle_platform(line: &str) -> String {
    let mut current = None;
    let mut rest: Vec<&str> = Vec::new();
    for token in line.split_whitespace() {
        match platform(&token.to_lowercase()) {
            Some(p) => current = Some(p),
            None => rest.push(token),
        }
    }
    let next = match current {
        None => Some(PLATFORMS[0].0),
        Some(p) => PLATFORMS.iter().position(|(_, name)| *name == p).and_then(|i| PLATFORMS.get(i + 1)).map(|(t, _)| *t),
    };
    if let Some(t) = next {
        rest.insert(0, t);
    }
    rest.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prob(platform: &str, id: &str, rating: i64, tags: &[&str]) -> Problem {
        Problem {
            platform: platform.into(),
            id: id.into(),
            name: format!("Problem {id}"),
            url: String::new(),
            rating,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            category: String::new(),
        }
    }

    #[test]
    fn parses_every_term_kind() {
        let q = parse("@CF 1200-1600 #dp,greedy #math knap");
        assert_eq!(q.platform, Some("codeforces"));
        assert_eq!((q.min, q.max), (Some(1200), Some(1600)));
        assert_eq!(q.tags, vec![vec!["dp".to_string(), "greedy".to_string()], vec!["math".to_string()]]);
        assert_eq!(q.words, vec!["knap".to_string()]);
        assert_eq!((parse("1600+").min, parse("-1400").max), (Some(1600), Some(1400)));
        assert_eq!(parse("#number_theory").tags, vec![vec!["number theory".to_string()]]);
    }

    #[test]
    fn matches_all_terms() {
        let a = prob("codeforces", "1A", 1300, &["dp", "math"]);
        let b = prob("codeforces", "2B", 1900, &["greedy"]);
        let c = prob("cses", "1068", 0, &[]);
        let hits = |line: &str| [&a, &b, &c].iter().filter(|p| parse(line).matches(p)).map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(hits(""), vec!["1A", "2B", "1068"]);
        assert_eq!(hits("@cses"), vec!["1068"]);
        assert_eq!(hits("1200-1600"), vec!["1A"]);
        assert_eq!(hits("1500+"), vec!["2B"]);
        assert_eq!(hits("#dp,greedy"), vec!["1A", "2B"]);
        assert_eq!(hits("#dp #greedy"), Vec::<String>::new());
        assert_eq!(hits("1a"), vec!["1A"]);
    }

    #[test]
    fn platform_cycles_and_keeps_other_terms() {
        assert_eq!(cycle_platform("#dp"), "@cf #dp");
        assert_eq!(cycle_platform("@cf #dp"), "@cses #dp");
        assert_eq!(cycle_platform("@cses"), "@atcoder");
        assert_eq!(cycle_platform("@atcoder #dp"), "#dp");
    }
}
