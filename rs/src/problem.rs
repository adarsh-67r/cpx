// Shared types: problems, submissions, contests, rating changes, and samples.

use serde::{Deserialize, Serialize};

/// One judge problem. Rating 0 means unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Problem {
    pub platform: String,
    pub id: String,
    pub name: String,
    pub url: String,
    pub rating: i64,
    pub tags: Vec<String>,
    pub category: String,
}

/// One judged attempt. The verdict is the judge's raw string, such as "OK".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Submission {
    pub platform: String,
    pub id: String,
    pub problem_id: String,
    pub verdict: String,
    pub language: String,
    pub submitted_at: i64,
}

/// One rated contest result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RatingChange {
    pub platform: String,
    pub contest_id: String,
    pub contest_name: String,
    pub old_rating: i64,
    pub new_rating: i64,
    pub at: i64,
}

/// A judge contest that has not finished yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Contest {
    pub platform: String,
    pub id: String,
    pub name: String,
    pub url: String,
    pub start: i64,
    pub duration: i64,
}

/// One sample test. The JSON keys match the Go version: Input and Output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Sample {
    #[serde(rename = "Input")]
    pub input: String,
    #[serde(rename = "Output")]
    pub output: String,
}
