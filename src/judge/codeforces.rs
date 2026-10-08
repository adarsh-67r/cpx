// Codeforces: the problemset and user data come from the public API; samples
// come from the problem page.

use crate::judge::{get, pre_text};
use crate::problem::{Contest, Problem, RatingChange, Sample, Submission};
use anyhow::{anyhow, Result};
use scraper::{Html, Selector};
use serde::Deserialize;

const API: &str = "https://codeforces.com/api";

#[derive(Deserialize)]
struct ApiEnvelope<T> {
    status: String,
    #[serde(default)]
    comment: String,
    result: Option<T>,
}

fn unwrap_api<T: for<'de> Deserialize<'de>>(body: &str) -> Result<T> {
    let env: ApiEnvelope<T> = serde_json::from_str(body)?;
    if env.status != "OK" {
        return Err(anyhow!("codeforces api: {}", env.comment));
    }
    env.result.ok_or_else(|| anyhow!("codeforces api: empty result"))
}

#[derive(Deserialize)]
struct ApiProblemset {
    problems: Vec<ApiProblem>,
}

#[derive(Deserialize)]
struct ApiProblem {
    #[serde(rename = "contestId")]
    contest_id: i64,
    index: String,
    name: String,
    #[serde(default)]
    rating: i64,
    #[serde(default)]
    tags: Vec<String>,
}

pub fn fetch_problems() -> Result<Vec<Problem>> {
    parse_problems(&get(&format!("{API}/problemset.problems"))?)
}

pub fn parse_problems(body: &str) -> Result<Vec<Problem>> {
    let set: ApiProblemset = unwrap_api(body)?;
    Ok(set
        .problems
        .into_iter()
        .map(|p| Problem {
            platform: "codeforces".into(),
            id: format!("{}{}", p.contest_id, p.index),
            name: p.name,
            url: format!("https://codeforces.com/problemset/problem/{}/{}", p.contest_id, p.index),
            rating: p.rating,
            tags: p.tags,
            category: String::new(),
        })
        .collect())
}

#[derive(Deserialize)]
struct ApiSubmission {
    id: i64,
    verdict: Option<String>,
    #[serde(rename = "programmingLanguage", default)]
    language: String,
    #[serde(rename = "creationTimeSeconds")]
    created: i64,
    problem: ApiSubmissionProblem,
}

#[derive(Deserialize)]
struct ApiSubmissionProblem {
    #[serde(rename = "contestId", default)]
    contest_id: i64,
    index: String,
}

pub fn fetch_submissions(handle: &str) -> Result<Vec<Submission>> {
    let url = format!("{API}/user.status?handle={}&from=1&count=100000", encode(handle));
    parse_submissions(&get(&url)?)
}

pub fn parse_submissions(body: &str) -> Result<Vec<Submission>> {
    let subs: Vec<ApiSubmission> = unwrap_api(body)?;
    Ok(subs
        .into_iter()
        .map(|s| Submission {
            platform: "codeforces".into(),
            id: s.id.to_string(),
            problem_id: format!("{}{}", s.problem.contest_id, s.problem.index),
            verdict: s.verdict.unwrap_or_default(),
            language: s.language,
            submitted_at: s.created,
        })
        .collect())
}

#[derive(Deserialize)]
struct ApiRating {
    #[serde(rename = "contestId")]
    contest_id: i64,
    #[serde(rename = "contestName")]
    contest_name: String,
    #[serde(rename = "oldRating")]
    old_rating: i64,
    #[serde(rename = "newRating")]
    new_rating: i64,
    #[serde(rename = "ratingUpdateTimeSeconds")]
    at: i64,
}

pub fn fetch_ratings(handle: &str) -> Result<Vec<RatingChange>> {
    let url = format!("{API}/user.rating?handle={}", encode(handle));
    parse_ratings(&get(&url)?)
}

pub fn parse_ratings(body: &str) -> Result<Vec<RatingChange>> {
    let rows: Vec<ApiRating> = unwrap_api(body)?;
    Ok(rows
        .into_iter()
        .map(|r| RatingChange {
            platform: "codeforces".into(),
            contest_id: r.contest_id.to_string(),
            contest_name: r.contest_name,
            old_rating: r.old_rating,
            new_rating: r.new_rating,
            at: r.at,
        })
        .collect())
}

#[derive(Deserialize)]
struct ApiContest {
    id: i64,
    name: String,
    phase: String,
    #[serde(rename = "startTimeSeconds", default)]
    start: i64,
    #[serde(rename = "durationSeconds", default)]
    duration: i64,
}

pub fn fetch_contests() -> Result<Vec<Contest>> {
    parse_contests(&get(&format!("{API}/contest.list?gym=false"))?)
}

/// Keeps contests that have not finished yet.
pub fn parse_contests(body: &str) -> Result<Vec<Contest>> {
    let list: Vec<ApiContest> = unwrap_api(body)?;
    Ok(list
        .into_iter()
        .filter(|c| c.phase != "FINISHED")
        .map(|c| Contest {
            platform: "codeforces".into(),
            id: c.id.to_string(),
            name: c.name,
            url: format!("https://codeforces.com/contest/{}", c.id),
            start: c.start,
            duration: c.duration,
        })
        .collect())
}

/// Samples on a problem page: each .sample-test pairs its .input and .output blocks.
pub fn parse_samples(page: &str) -> Vec<Sample> {
    let doc = Html::parse_document(page);
    let sample_sel = Selector::parse(".sample-test").unwrap();
    let input_sel = Selector::parse(".input pre").unwrap();
    let output_sel = Selector::parse(".output pre").unwrap();

    let mut out = Vec::new();
    for st in doc.select(&sample_sel) {
        let ins: Vec<_> = st.select(&input_sel).collect();
        let outs: Vec<_> = st.select(&output_sel).collect();
        for (i, o) in ins.into_iter().zip(outs) {
            out.push(Sample {
                input: pre_text(i),
                output: pre_text(o),
            });
        }
    }
    out
}

pub fn fetch_samples(url: &str) -> Result<Vec<Sample>> {
    Ok(parse_samples(&get(url)?))
}

/// Percent-encodes the characters that matter in a query value.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::fixture;

    #[test]
    fn samples_from_saved_page() {
        let s = parse_samples(&fixture("cf_4A.html"));
        assert_eq!(s, vec![Sample { input: "8".into(), output: "YES".into() }]);
    }

    #[test]
    fn problems_from_api() {
        let body = r#"{"status":"OK","result":{"problems":[
            {"contestId":4,"index":"A","name":"Watermelon","rating":800,"tags":["math"]},
            {"contestId":1095,"index":"F","name":"Make It Connected","tags":[]}
        ],"problemStatistics":[]}}"#;
        let ps = parse_problems(body).unwrap();
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "4A");
        assert_eq!(ps[0].rating, 800);
        assert_eq!(ps[1].rating, 0);
        assert_eq!(ps[0].url, "https://codeforces.com/problemset/problem/4/A");
    }

    #[test]
    fn api_failure_is_an_error() {
        assert!(parse_problems(r#"{"status":"FAILED","comment":"busy"}"#).is_err());
        assert!(parse_submissions(r#"{"status":"FAILED","comment":"handle: not found"}"#).is_err());
    }

    #[test]
    fn submissions_and_ratings() {
        let subs = parse_submissions(
            r#"{"status":"OK","result":[{"id":350,"contestId":4,"verdict":"OK","programmingLanguage":"GNU C++17","creationTimeSeconds":1700000000,"problem":{"contestId":4,"index":"A"}}]}"#,
        )
        .unwrap();
        assert_eq!(subs[0].problem_id, "4A");
        assert_eq!(subs[0].submitted_at, 1700000000);

        let rs = parse_ratings(
            r#"{"status":"OK","result":[{"contestId":1700,"contestName":"Round 1","oldRating":1400,"newRating":1500,"ratingUpdateTimeSeconds":1690000000}]}"#,
        )
        .unwrap();
        assert_eq!(rs[0].contest_id, "1700");
        assert_eq!(rs[0].new_rating, 1500);
    }

    #[test]
    fn contests_drop_finished() {
        let cs = parse_contests(
            r#"{"status":"OK","result":[
                {"id":2050,"name":"A","phase":"BEFORE","startTimeSeconds":1800000000,"durationSeconds":7200},
                {"id":2040,"name":"Old","phase":"FINISHED","startTimeSeconds":1700000000,"durationSeconds":7200}
            ]}"#,
        )
        .unwrap();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].url, "https://codeforces.com/contest/2050");
    }

    #[test]
    fn handles_are_encoded() {
        assert_eq!(encode("adarsh67"), "adarsh67");
        assert_eq!(encode("a b"), "a%20b");
    }
}
