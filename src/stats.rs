// Numbers for the Dashboard and Analytics tabs: streaks, daily activity,
// per-tag records, and the sparkline and heat shades.

use crate::problem::{Problem, Submission};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::collections::HashMap;

/// The UTC calendar day of a unix time.
pub fn day(unix: i64) -> NaiveDate {
    DateTime::from_timestamp(unix, 0).unwrap_or_default().date_naive()
}

/// Submissions per UTC day.
pub fn activity(subs: &[Submission]) -> HashMap<NaiveDate, usize> {
    let mut out = HashMap::new();
    for s in subs {
        *out.entry(day(s.submitted_at)).or_insert(0) += 1;
    }
    out
}

/// Consecutive days with activity, ending today. With no activity yet today,
/// the streak counts back from yesterday.
pub fn streak(activity: &HashMap<NaiveDate, usize>, now: DateTime<Utc>) -> usize {
    let mut d = now.date_naive();
    if activity.get(&d).copied().unwrap_or(0) == 0 {
        d -= Duration::days(1);
    }
    let mut n = 0;
    while activity.get(&d).copied().unwrap_or(0) > 0 {
        n += 1;
        d -= Duration::days(1);
    }
    n
}

/// Accepted submissions at or after `since` (unix seconds).
pub fn accepted_since(subs: &[Submission], since: i64) -> usize {
    subs.iter().filter(|s| s.verdict == "OK" && s.submitted_at >= since).count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicStat {
    pub tag: String,
    pub accepted: usize,
    pub attempted: usize,
    pub avg_rating: i64,
}

/// Per-tag records. A problem counts as attempted when any submission exists
/// for it, and as accepted when one was OK. Most accepted first.
pub fn topics(problems: &[Problem], subs: &[Submission]) -> Vec<TopicStat> {
    let mut attempted: HashMap<&str, bool> = HashMap::new();
    let mut accepted: HashMap<&str, bool> = HashMap::new();
    for s in subs {
        attempted.insert(s.problem_id.as_str(), true);
        if s.verdict == "OK" {
            accepted.insert(s.problem_id.as_str(), true);
        }
    }

    #[derive(Default)]
    struct Agg {
        accepted: usize,
        attempted: usize,
        rating_sum: i64,
        rating_n: i64,
    }
    let mut by_tag: HashMap<String, Agg> = HashMap::new();
    for p in problems {
        if !attempted.contains_key(p.id.as_str()) {
            continue;
        }
        for tag in &p.tags {
            let a = by_tag.entry(tag.clone()).or_default();
            a.attempted += 1;
            if accepted.contains_key(p.id.as_str()) {
                a.accepted += 1;
                if p.rating > 0 {
                    a.rating_sum += p.rating;
                    a.rating_n += 1;
                }
            }
        }
    }

    let mut out: Vec<TopicStat> = by_tag
        .into_iter()
        .map(|(tag, a)| TopicStat {
            tag,
            accepted: a.accepted,
            attempted: a.attempted,
            avg_rating: if a.rating_n > 0 { a.rating_sum / a.rating_n } else { 0 },
        })
        .collect();
    out.sort_by(|a, b| b.accepted.cmp(&a.accepted).then_with(|| a.tag.cmp(&b.tag)));
    out
}

/// Draws values as block characters, scaled to the largest value.
pub fn sparkline(values: &[i64]) -> String {
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let (Some(&lo), Some(&hi)) = (values.iter().min(), values.iter().max()) else {
        return String::new();
    };
    values
        .iter()
        .map(|&v| {
            let idx = if hi > lo { ((v - lo) * 7 / (hi - lo)) as usize } else { 0 };
            BLOCKS[idx.min(7)]
        })
        .collect()
}

/// A block chart `height` rows tall of the last `width` values, top row
/// first. Each column fills up to its value in eighths of a row; the lowest
/// value still shows a sliver so every contest is visible.
pub fn chart(values: &[i64], height: usize, width: usize) -> Vec<String> {
    const PARTS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];
    let values = &values[values.len().saturating_sub(width)..];
    let (Some(&lo), Some(&hi)) = (values.iter().min(), values.iter().max()) else {
        return Vec::new();
    };
    let total = (height * 8) as i64;
    let filled: Vec<i64> = values.iter().map(|&v| if hi > lo { 1 + (v - lo) * (total - 1) / (hi - lo) } else { total / 2 }).collect();
    (0..height)
        .map(|row| {
            let floor = ((height - 1 - row) * 8) as i64;
            filled
                .iter()
                .map(|&f| match f - floor {
                    n if n >= 8 => '█',
                    n if n <= 0 => ' ',
                    n => PARTS[n as usize],
                })
                .collect()
        })
        .collect()
}

/// The Codeforces rank title for a rating.
pub fn rank(rating: i64) -> &'static str {
    match rating {
        r if r >= 3000 => "Legendary Grandmaster",
        r if r >= 2600 => "International Grandmaster",
        r if r >= 2400 => "Grandmaster",
        r if r >= 2300 => "International Master",
        r if r >= 2100 => "Master",
        r if r >= 1900 => "Candidate Master",
        r if r >= 1600 => "Expert",
        r if r >= 1400 => "Specialist",
        r if r >= 1200 => "Pupil",
        _ => "Newbie",
    }
}

/// Shade level 0..=4 for a daily count. Zero stays empty.
pub fn heat_level(count: usize) -> usize {
    match count {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4..=6 => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_scales_columns_to_the_range() {
        let c = chart(&[100, 200, 300], 2, 10);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0], "  █"); // only the highest reaches the top row
        assert_eq!(c[1].chars().next(), Some('▁')); // the lowest keeps a sliver
        assert_eq!(c[1].chars().nth(2), Some('█'));
        assert_eq!(chart(&[1, 2, 3, 4, 5], 1, 2)[0].chars().count(), 2, "keeps the last width values");
        assert!(chart(&[], 3, 10).is_empty());
    }

    #[test]
    fn rank_titles() {
        assert_eq!(rank(393), "Newbie");
        assert_eq!(rank(1200), "Pupil");
        assert_eq!(rank(2399), "International Master");
    }
    use chrono::TimeZone;

    fn sub(verdict: &str, at: i64, problem: &str) -> Submission {
        Submission { platform: "codeforces".into(), id: at.to_string(), problem_id: problem.into(), verdict: verdict.into(), language: String::new(), submitted_at: at }
    }

    fn at(day: &str) -> i64 {
        NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap().and_hms_opt(1, 0, 0).unwrap().and_utc().timestamp()
    }

    #[test]
    fn streak_counts_back_from_today() {
        let now = Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap();
        let mut act = HashMap::new();
        for d in ["2026-10-05", "2026-10-06", "2026-10-07"] {
            act.insert(NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap(), 1);
        }
        assert_eq!(streak(&act, now), 3);
        act.remove(&NaiveDate::from_ymd_opt(2026, 10, 7).unwrap());
        assert_eq!(streak(&act, now), 2, "no activity yet today counts from yesterday");
        assert_eq!(streak(&HashMap::new(), now), 0);
    }

    #[test]
    fn accepted_since_counts_only_ok() {
        let subs = vec![sub("OK", at("2026-10-01"), "1A"), sub("OK", at("2026-10-07"), "1B"), sub("WRONG_ANSWER", at("2026-10-07"), "1C")];
        assert_eq!(accepted_since(&subs, at("2026-10-05")), 1);
    }

    #[test]
    fn topics_count_and_average() {
        let probs = vec![
            Problem { id: "1A".into(), rating: 800, tags: vec!["math".into()], ..Default::default() },
            Problem { id: "1B".into(), rating: 1200, tags: vec!["math".into(), "dp".into()], ..Default::default() },
            Problem { id: "1C".into(), rating: 1600, tags: vec!["dp".into()], ..Default::default() },
            Problem { id: "1D".into(), rating: 900, tags: vec!["math".into()], ..Default::default() },
        ];
        let subs = vec![sub("OK", 1, "1A"), sub("OK", 2, "1B"), sub("WRONG_ANSWER", 3, "1C")];
        let t = topics(&probs, &subs);
        let math = t.iter().find(|s| s.tag == "math").unwrap();
        assert_eq!((math.accepted, math.attempted, math.avg_rating), (2, 2, 1000));
        let dp = t.iter().find(|s| s.tag == "dp").unwrap();
        assert_eq!((dp.accepted, dp.attempted, dp.avg_rating), (1, 2, 1200));
        assert_eq!(t[0].tag, "math");
    }

    #[test]
    fn sparkline_scales() {
        assert_eq!(sparkline(&[1, 2, 3, 4, 5, 6, 7, 8]), "▁▂▃▄▅▆▇█");
        assert_eq!(sparkline(&[5, 5, 5]), "▁▁▁");
        assert_eq!(sparkline(&[]), "");
    }

    #[test]
    fn heat_levels() {
        assert_eq!([heat_level(0), heat_level(1), heat_level(3), heat_level(6), heat_level(7)], [0, 1, 2, 3, 4]);
    }
}
