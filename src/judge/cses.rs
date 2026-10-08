// CSES: samples and the problem list come from the pages. Solved and
// attempted state comes from the score icons, which only show when logged in.

use crate::judge::{get, get_with, pre_text, tidy};
use crate::problem::{Problem, Sample};
use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

pub const LIST_URL: &str = "https://cses.fi/problemset/list/";

/// The JSON keys match the Go version (field names, not snake_case).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    #[serde(rename = "Solved")]
    pub solved: Vec<String>,
    #[serde(rename = "Attempted")]
    pub attempted: Vec<String>,
}

/// The sample pairs that follow the "Example" heading.
pub fn parse_samples(page: &str) -> Vec<Sample> {
    let doc = Html::parse_document(page);
    let example = Selector::parse("#example").unwrap();
    let Some(ex) = doc.select(&example).next() else {
        return Vec::new();
    };
    let pres: Vec<ElementRef> = ex
        .next_siblings()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "pre")
        .collect();
    pres.chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| Sample {
            input: pre_text(c[0]),
            output: pre_text(c[1]),
        })
        .collect()
}

pub fn fetch_samples(url: &str) -> Result<Vec<Sample>> {
    Ok(parse_samples(&get(url)?))
}

/// The task list, with each task's category, plus solved and attempted state.
pub fn fetch_list(session: &str) -> Result<(Vec<Problem>, Progress)> {
    let body = get_with(LIST_URL, Some(session).filter(|s| !s.is_empty()))?;
    Ok(parse_list(&body))
}

pub fn parse_list(page: &str) -> (Vec<Problem>, Progress) {
    let doc = Html::parse_document(page);
    let sel = Selector::parse("h2, li.task").unwrap();
    let link = Selector::parse("a").unwrap();
    let score = Selector::parse(".task-score").unwrap();

    let mut problems = Vec::new();
    let mut progress = Progress::default();
    let mut category = String::new();
    for el in doc.select(&sel) {
        if el.value().name() == "h2" {
            category = tidy(&el.text().collect::<String>());
            continue;
        }
        let Some(a) = el.select(&link).next() else { continue };
        let href = a.value().attr("href").unwrap_or("");
        let id = href.rsplit('/').next().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        problems.push(Problem {
            platform: "cses".into(),
            id: id.clone(),
            name: tidy(&a.text().collect::<String>()),
            url: format!("https://cses.fi/problemset/task/{id}"),
            rating: 0,
            tags: Vec::new(),
            category: category.clone(),
        });
        if let Some(s) = el.select(&score).next() {
            let classes = s.value().attr("class").unwrap_or("");
            if classes.split_whitespace().any(|c| c == "full") {
                progress.solved.push(id.clone());
            } else if classes.split_whitespace().any(|c| c == "zero") {
                progress.attempted.push(id);
            }
        }
    }
    (problems, progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::fixture;

    #[test]
    fn samples_from_saved_page() {
        let s = parse_samples(&fixture("cses_1068.html"));
        assert_eq!(s, vec![Sample { input: "3".into(), output: "3 10 5 16 8 4 2 1".into() }]);
    }

    #[test]
    fn list_from_saved_page() {
        let (probs, progress) = parse_list(&fixture("cses_list.html"));
        assert!(probs.len() >= 100, "got {}", probs.len());
        let weird = probs.iter().find(|p| p.id == "1068").expect("1068 present");
        assert_eq!(weird.name, "Weird Algorithm");
        assert_eq!(weird.category, "Introductory Problems");
        assert_eq!(weird.url, "https://cses.fi/problemset/task/1068");
        assert!(progress.solved.is_empty(), "logged-out page has no progress");
    }

    #[test]
    fn progress_from_logged_in_scores() {
        let page = r#"<div class="content"><h2>Introductory Problems</h2><ul class="task-list">
            <li class="task"><a href="/problemset/task/1068">Weird Algorithm</a><span class="task-score icon full"></span>
            <li class="task"><a href="/problemset/task/1083">Missing Number</a><span class="task-score icon zero"></span>
            <li class="task"><a href="/problemset/task/1069">Repetitions</a><span class="task-score icon "></span>
            </ul></div>"#;
        let (probs, progress) = parse_list(page);
        assert_eq!(probs.len(), 3);
        assert_eq!(progress.solved, vec!["1068".to_string()]);
        assert_eq!(progress.attempted, vec!["1083".to_string()]);
    }
}
