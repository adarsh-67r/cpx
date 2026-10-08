// AtCoder: samples come from the task page. The whole catalog comes from
// kenkoooo.com's community resources, since AtCoder's own site lists tasks
// one contest at a time. Difficulty values there are estimates.

use crate::judge::{get, pre_text};
use crate::problem::{Problem, Sample};
use anyhow::{anyhow, Result};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const PROBLEMS_URL: &str = "https://kenkoooo.com/atcoder/resources/problems.json";
pub const MODELS_URL: &str = "https://kenkoooo.com/atcoder/resources/problem-models.json";

/// Samples are "Sample Input N" / "Sample Output N", or 入力例 N / 出力例 N.
pub fn parse_samples(page: &str) -> Vec<Sample> {
    let doc = Html::parse_document(page);
    let h3 = Selector::parse("h3").unwrap();
    let pre = Selector::parse("pre").unwrap();

    let mut inputs: BTreeMap<u32, String> = BTreeMap::new();
    let mut outputs: BTreeMap<u32, String> = BTreeMap::new();
    for h in doc.select(&h3) {
        let label: String = h.text().collect::<String>().trim().to_string();
        let Some(n) = label.split_whitespace().last().and_then(|x| x.parse::<u32>().ok()) else {
            continue;
        };
        let Some(parent) = h.parent().and_then(ElementRef::wrap) else {
            continue;
        };
        let Some(p) = parent.select(&pre).next() else { continue };
        let text = pre_text(p);
        if label.starts_with("Sample Input") || label.starts_with("入力例") {
            inputs.insert(n, text);
        } else if label.starts_with("Sample Output") || label.starts_with("出力例") {
            outputs.insert(n, text);
        }
    }
    inputs
        .into_iter()
        .filter_map(|(n, input)| outputs.get(&n).map(|output| Sample { input, output: output.clone() }))
        .collect()
}

pub fn fetch_samples(url: &str) -> Result<Vec<Sample>> {
    Ok(parse_samples(&get(url)?))
}

#[derive(Deserialize)]
struct ApiProblem {
    id: String,
    contest_id: String,
    name: String,
}

#[derive(Deserialize)]
struct ApiModel {
    #[serde(default)]
    difficulty: i64,
}

pub fn fetch_catalog() -> Result<Vec<Problem>> {
    parse_catalog(&get(PROBLEMS_URL)?, &get(MODELS_URL)?)
}

/// Joins the problem list with the difficulty estimates by id. A difficulty
/// of 0 or less means unknown.
pub fn parse_catalog(problems_json: &str, models_json: &str) -> Result<Vec<Problem>> {
    let list: Vec<ApiProblem> = serde_json::from_str(problems_json).map_err(|e| anyhow!("atcoder problems: {e}"))?;
    let models: BTreeMap<String, ApiModel> =
        serde_json::from_str(models_json).map_err(|e| anyhow!("atcoder models: {e}"))?;
    Ok(list
        .into_iter()
        .map(|p| {
            let rating = models.get(&p.id).map(|m| m.difficulty).filter(|d| *d > 0).unwrap_or(0);
            Problem {
                platform: "atcoder".into(),
                url: format!("https://atcoder.jp/contests/{}/tasks/{}", p.contest_id, p.id),
                id: p.id,
                name: p.name,
                rating,
                tags: Vec::new(),
                category: p.contest_id,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::fixture;

    #[test]
    fn samples_from_saved_page() {
        let s = parse_samples(&fixture("atcoder_abc001_1.html"));
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], Sample { input: "15\n10".into(), output: "5".into() });
    }

    #[test]
    fn catalog_joins_difficulty() {
        let problems = r#"[
            {"id":"abc001_1","contest_id":"abc001","problem_index":"A","name":"Vacant","title":"A"},
            {"id":"abc001_2","contest_id":"abc001","problem_index":"B","name":"Visibility","title":"B"}
        ]"#;
        let models = r#"{"abc001_1":{"difficulty":-200},"abc001_2":{"difficulty":650}}"#;
        let ps = parse_catalog(problems, models).unwrap();
        assert_eq!(ps[0].rating, 0);
        assert_eq!(ps[1].rating, 650);
        assert_eq!(ps[1].url, "https://atcoder.jp/contests/abc001/tasks/abc001_2");
        assert_eq!(ps[1].category, "abc001");
    }

    #[test]
    fn catalog_bad_json_is_an_error() {
        assert!(parse_catalog("not json", "{}").is_err());
    }
}
