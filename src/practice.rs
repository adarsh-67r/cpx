// Picks problems to work on next. Skill per tag is the 75th percentile of the
// ratings of accepted problems with that tag. Each problem is scored by mode.

use crate::problem::Problem;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Weakness,
    Push,
    Refresh,
    Upsolve,
    Explore,
    Plan,
}

pub const MODES: [Mode; 7] = [Mode::Auto, Mode::Weakness, Mode::Push, Mode::Refresh, Mode::Upsolve, Mode::Explore, Mode::Plan];

/// Rating milestones Plan steps through (Codeforces ranks).
pub const LADDER: [i64; 9] = [1200, 1400, 1600, 1900, 2100, 2300, 2400, 2600, 3000];

const REFRESH_AFTER: i64 = 60 * 24 * 3600;
const MAX_WEAK_GAP: i64 = 500;

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Weakness => "weakness",
            Mode::Push => "push",
            Mode::Refresh => "refresh",
            Mode::Upsolve => "upsolve",
            Mode::Explore => "explore",
            Mode::Plan => "plan",
        }
    }

    /// The next mode in the cycle the UI uses.
    pub fn next(self) -> Mode {
        let i = MODES.iter().position(|m| *m == self).unwrap_or(0);
        MODES[(i + 1) % MODES.len()]
    }
}

/// The user's state. rating 0 means unknown.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub problems: Vec<Problem>,
    /// Problem key (platform + id) to accepted.
    pub accepted: HashSet<String>,
    /// Problem key to the latest submission time (unix seconds), any verdict.
    pub attempted: HashMap<String, i64>,
    pub rating: i64,
    pub now: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub problem: Problem,
    pub score: f64,
    pub reasons: Vec<String>,
}

pub fn key(p: &Problem) -> String {
    format!("{}{}", p.platform, p.id)
}

/// Skill per tag: the 75th percentile of ratings of accepted problems with that tag.
pub fn tag_skill(input: &Input) -> HashMap<String, i64> {
    let mut ratings: HashMap<&str, Vec<i64>> = HashMap::new();
    for p in &input.problems {
        if !input.accepted.contains(&key(p)) || p.rating == 0 {
            continue;
        }
        for t in &p.tags {
            ratings.entry(t.as_str()).or_default().push(p.rating);
        }
    }
    ratings
        .into_iter()
        .map(|(tag, mut rs)| {
            rs.sort();
            let idx = ((rs.len() - 1) as f64 * 0.75).floor() as usize;
            (tag.to_string(), rs[idx])
        })
        .collect()
}

/// The latest submission time for each tag, over all problems carrying it.
fn tag_touch(input: &Input) -> HashMap<String, i64> {
    let mut touch: HashMap<String, i64> = HashMap::new();
    for p in &input.problems {
        let Some(&at) = input.attempted.get(&key(p)) else { continue };
        for t in &p.tags {
            let e = touch.entry(t.clone()).or_insert(0);
            if at > *e {
                *e = at;
            }
        }
    }
    touch
}

struct Ctx<'a> {
    input: &'a Input,
    skill: HashMap<String, i64>,
    touch: HashMap<String, i64>,
    target: i64,
}

impl Ctx<'_> {
    /// The tag's skill, or the user's rating when the tag has no solves yet.
    fn skill_of(&self, tag: &str) -> i64 {
        self.skill.get(tag).copied().unwrap_or(self.input.rating)
    }
}

/// Up to `count` unsolved problems for the mode, best first.
pub fn recommend(input: &Input, mode: Mode, count: usize) -> Vec<Pick> {
    let target = if input.rating == 0 { 1200 } else { input.rating + 50 };
    let ctx = Ctx { input, skill: tag_skill(input), touch: tag_touch(input), target };

    let mut picks: Vec<Pick> = input
        .problems
        .iter()
        .filter(|p| !input.accepted.contains(&key(p)) && p.rating != 0)
        .map(|p| match mode {
            Mode::Weakness => score_weakness(p, &ctx),
            Mode::Push => score_push(p, &ctx),
            Mode::Refresh => score_refresh(p, &ctx),
            Mode::Upsolve => score_upsolve(p, &ctx),
            Mode::Explore => score_explore(p, &ctx),
            Mode::Plan => score_plan(p, &ctx),
            Mode::Auto => score_auto(p, &ctx),
        })
        .filter(|pk| pk.score > 0.0)
        .collect();

    // Stable sort: equal scores keep the catalog order.
    picks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    picks.truncate(count);
    picks
}

fn pick(p: &Problem, score: f64, reasons: Vec<String>) -> Pick {
    Pick { problem: p.clone(), score, reasons }
}

fn empty(p: &Problem) -> Pick {
    pick(p, 0.0, Vec::new())
}

/// Useful when the problem sits above the user's skill in one tag, by at most MAX_WEAK_GAP.
fn score_weakness(p: &Problem, c: &Ctx) -> Pick {
    let mut best = 0.0;
    let mut reasons = Vec::new();
    for t in &p.tags {
        let s = c.skill_of(t);
        let gap = p.rating - s;
        if gap <= 0 || gap > MAX_WEAK_GAP {
            continue;
        }
        if gap as f64 > best {
            best = gap as f64;
            reasons = vec![format!("{t}: your skill ~{s}, problem {}", p.rating)];
        }
    }
    pick(p, best, reasons)
}

/// Close to the target rating scores higher. Weaker tags add a small bonus.
fn score_auto(p: &Problem, c: &Ctx) -> Pick {
    let closeness = 1000.0 - (p.rating - c.target).abs() as f64;
    if closeness <= 0.0 {
        return empty(p);
    }
    let mut bonus = 0.0;
    let mut reasons = vec![format!("rating {} near target {}", p.rating, c.target)];
    for t in &p.tags {
        match c.skill.get(t) {
            Some(&s) if s >= c.target => {}
            Some(&s) => {
                bonus += 50.0;
                reasons.push(format!("{t} is a weaker tag (skill ~{s})"));
            }
            None => {
                bonus += 50.0;
                reasons.push(format!("{t}: no solves yet"));
            }
        }
    }
    pick(p, closeness + bonus, reasons)
}

/// Just above the user's rating, in a tag where they are already strong.
fn score_push(p: &Problem, c: &Ctx) -> Pick {
    let r = c.input.rating;
    if r == 0 || p.rating <= r || p.rating > r + 300 {
        return empty(p);
    }
    for t in &p.tags {
        if let Some(&s) = c.skill.get(t) {
            if s >= r {
                return pick(
                    p,
                    (300 - (p.rating - r)) as f64,
                    vec![
                        format!("{} is above your rating {r}", p.rating),
                        format!("{t} is strong for you (skill ~{s})"),
                    ],
                );
            }
        }
    }
    empty(p)
}

/// A tag solved before but not touched for REFRESH_AFTER, at a level already handled.
fn score_refresh(p: &Problem, c: &Ctx) -> Pick {
    for t in &p.tags {
        let Some(&s) = c.skill.get(t) else { continue };
        let last = c.touch.get(t).copied().unwrap_or(0);
        if c.input.now - last < REFRESH_AFTER {
            continue;
        }
        if p.rating < s - 200 || p.rating > s + 100 {
            continue;
        }
        return pick(p, 100.0, vec![format!("{t}: not touched in 60+ days (skill ~{s})")]);
    }
    empty(p)
}

/// Attempted but never accepted. Newer attempts first.
fn score_upsolve(p: &Problem, c: &Ctx) -> Pick {
    match c.input.attempted.get(&key(p)) {
        Some(&at) => pick(p, 1.0 + at as f64 / 1e10, vec!["attempted, never accepted".into()]),
        None => empty(p),
    }
}

/// A tag with no accepted problems yet, at or near the user's level.
fn score_explore(p: &Problem, c: &Ctx) -> Pick {
    if c.input.rating > 0 && p.rating > c.input.rating + 100 {
        return empty(p);
    }
    for t in &p.tags {
        if !c.skill.contains_key(t) {
            return pick(p, 100.0 - p.rating as f64 / 100.0, vec![format!("{t}: never solved yet")]);
        }
    }
    empty(p)
}

/// The rung just under the next milestone above the user.
fn score_plan(p: &Problem, c: &Ctx) -> Pick {
    let r = c.input.rating;
    let rung = LADDER.iter().copied().find(|&m| m > r).unwrap_or(LADDER[LADDER.len() - 1]);
    if p.rating > rung || p.rating <= rung - 300 {
        return empty(p);
    }
    pick(p, p.rating as f64, vec![format!("rung toward {rung} (you are {r})")])
}

/// Groups picks by tag for the Practice tab's detail, most picks first.
pub fn tags_of(picks: &[Pick]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for pk in picks {
        for t in &pk.problem.tags {
            *out.entry(t.clone()).or_insert(0) += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cf(id: &str, rating: i64, tags: &[&str]) -> Problem {
        Problem {
            platform: "codeforces".into(),
            id: id.into(),
            rating,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        }
    }

    fn ids(picks: &[Pick]) -> Vec<&str> {
        picks.iter().map(|p| p.problem.id.as_str()).collect()
    }

    #[test]
    fn skill_is_75th_percentile_of_accepted() {
        let input = Input {
            problems: vec![cf("1A", 800, &["dp"]), cf("1B", 1000, &["dp"]), cf("1C", 1200, &["dp"]), cf("1D", 1600, &["dp"]), cf("1E", 2000, &["graphs"])],
            accepted: ["codeforces1A", "codeforces1B", "codeforces1C", "codeforces1D"].iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let skill = tag_skill(&input);
        assert_eq!(skill["dp"], 1200);
        assert!(!skill.contains_key("graphs"));
    }

    #[test]
    fn recommend_skips_solved_and_unrated() {
        let input = Input {
            problems: vec![cf("1A", 1200, &["dp"]), cf("1B", 1200, &["dp"]), cf("1C", 0, &["dp"])],
            accepted: ["codeforces1B".to_string()].into_iter().collect(),
            rating: 1200,
            ..Default::default()
        };
        assert_eq!(ids(&recommend(&input, Mode::Auto, 10)), vec!["1A"]);
    }

    #[test]
    fn weakness_prefers_above_skill_in_a_tag() {
        let input = Input {
            problems: vec![cf("1A", 1000, &["math"]), cf("1B", 1000, &["dp"]), cf("2A", 1200, &["dp"]), cf("2B", 1000, &["math"])],
            accepted: ["codeforces1A", "codeforces1B"].iter().map(|s| s.to_string()).collect(),
            rating: 1000,
            ..Default::default()
        };
        let picks = recommend(&input, Mode::Weakness, 5);
        assert_eq!(ids(&picks), vec!["2A"]);
        assert!(!picks[0].reasons.is_empty());
    }

    #[test]
    fn weakness_works_for_low_ratings() {
        // The case that found a bug in the Go version: a 393 rating with 800-rated tags.
        let input = Input {
            problems: vec![cf("1003B", 1300, &["constructive algorithms"]), cf("1A", 800, &["constructive algorithms"])],
            accepted: ["codeforces1A"].iter().map(|s| s.to_string()).collect(),
            rating: 393,
            ..Default::default()
        };
        assert!(!recommend(&input, Mode::Weakness, 5).is_empty());
    }

    #[test]
    fn auto_prefers_target_rating() {
        let input = Input {
            problems: vec![cf("3A", 1000, &["greedy"]), cf("3B", 1300, &["greedy"])],
            rating: 1200,
            ..Default::default()
        };
        assert_eq!(ids(&recommend(&input, Mode::Auto, 2))[0], "3B");
    }

    #[test]
    fn push_picks_above_rating_in_strong_tag() {
        let input = Input {
            problems: vec![cf("1A", 1000, &["dp"]), cf("1B", 1100, &["dp"]), cf("1C", 1200, &["dp"]), cf("1D", 1300, &["dp"]), cf("2A", 1250, &["dp"]), cf("2B", 1250, &["geometry"]), cf("2C", 1500, &["dp"])],
            accepted: ["codeforces1A", "codeforces1B", "codeforces1C", "codeforces1D"].iter().map(|s| s.to_string()).collect(),
            rating: 1000,
            ..Default::default()
        };
        assert_eq!(ids(&recommend(&input, Mode::Push, 10)), vec!["2A"]);
    }

    #[test]
    fn refresh_revisits_stale_tags() {
        let now = 10_000_000_000;
        let input = Input {
            problems: vec![cf("1A", 1000, &["dp"]), cf("1B", 1000, &["graphs"]), cf("2A", 1000, &["dp"]), cf("2B", 1000, &["graphs"])],
            accepted: ["codeforces1A", "codeforces1B"].iter().map(|s| s.to_string()).collect(),
            attempted: [("codeforces1A".to_string(), now - 100 * 24 * 3600), ("codeforces1B".to_string(), now - 3 * 24 * 3600)].into_iter().collect(),
            rating: 1000,
            now,
        };
        assert_eq!(ids(&recommend(&input, Mode::Refresh, 10)), vec!["2A"]);
    }

    #[test]
    fn upsolve_newest_first() {
        let input = Input {
            problems: vec![cf("1A", 1000, &["dp"]), cf("1B", 1000, &["dp"]), cf("1C", 1000, &["dp"])],
            accepted: ["codeforces1C".to_string()].into_iter().collect(),
            attempted: [("codeforces1A".to_string(), 100), ("codeforces1B".to_string(), 200), ("codeforces1C".to_string(), 50)].into_iter().collect(),
            rating: 1000,
            ..Default::default()
        };
        assert_eq!(ids(&recommend(&input, Mode::Upsolve, 10)), vec!["1B", "1A"]);
    }

    #[test]
    fn explore_introduces_new_tag() {
        let input = Input {
            problems: vec![cf("1A", 1000, &["dp"]), cf("2A", 1000, &["graphs"]), cf("2B", 1000, &["dp"])],
            accepted: ["codeforces1A".to_string()].into_iter().collect(),
            rating: 1000,
            ..Default::default()
        };
        assert_eq!(ids(&recommend(&input, Mode::Explore, 10)), vec!["2A"]);
    }

    #[test]
    fn plan_targets_the_next_rung() {
        let input = Input {
            problems: vec![cf("1A", 1100, &["x"]), cf("1B", 1150, &["x"]), cf("1C", 1300, &["x"]), cf("1D", 1400, &["x"])],
            rating: 1250,
            ..Default::default()
        };
        let picks = recommend(&input, Mode::Plan, 10);
        assert_eq!(picks[0].problem.rating, 1400);
        assert!(picks.iter().all(|p| p.problem.rating > 1100 && p.problem.rating <= 1400));
    }

    #[test]
    fn modes_cycle() {
        assert_eq!(Mode::Plan.next(), Mode::Auto);
        assert_eq!(Mode::Auto.next(), Mode::Weakness);
    }
}
