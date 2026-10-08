// Goal plans: "I want to reach rating N". The topics that matter are read from
// the catalog itself (how often each tag shows up among problems near the
// goal), each is compared with the user's skill in it, and a short ladder of
// unsolved problems climbs from the current rating to the goal, leaning on the
// topics with the biggest gap.

use crate::practice::{key, tag_skill, Input, LADDER};
use crate::problem::Problem;
use crate::stats::rank;
use std::collections::HashMap;

pub const TARGET_MIN: i64 = 900;
pub const TARGET_MAX: i64 = 3500;
/// Problems per rung, and the most rungs a plan has.
const PER_RUNG: usize = 6;
const MAX_RUNGS: usize = 4;
/// A tag must cover this share of the goal band to count as a goal topic.
const MIN_SHARE: f64 = 0.03;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ready,
    Close,
    Weak,
    New,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Ready => "ready",
            Status::Close => "close",
            Status::Weak => "weak",
            Status::New => "new",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Topic {
    pub tag: String,
    /// Share of goal-band problems with this tag, 0..1.
    pub share: f64,
    /// The user's skill in the tag (see practice::tag_skill); 0 when none solved.
    pub skill: i64,
    pub status: Status,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub problem: Problem,
    /// The weakest goal topic this problem trains.
    pub tag: String,
    pub rung: usize,
    pub rungs: usize,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub target: i64,
    pub target_rank: &'static str,
    /// Official rating; 0 when unrated.
    pub rating: i64,
    pub readiness_pct: u32,
    /// Goal topics that are not ready, biggest gap first.
    pub focus: Vec<Topic>,
    pub steps: Vec<Step>,
}

/// The next rank milestone above rating: the default goal.
pub fn next_milestone(rating: i64) -> i64 {
    LADDER.iter().copied().find(|&m| m > rating).unwrap_or(TARGET_MAX)
}

/// Steps the goal to the next (dir > 0) or previous milestone, or by 100 past the ends.
pub fn cycle_milestone(current: i64, dir: i64) -> i64 {
    let hit = if dir > 0 { LADDER.iter().copied().find(|&m| m > current) } else { LADDER.iter().rev().copied().find(|&m| m < current) };
    hit.unwrap_or(current + 100 * dir.signum()).clamp(TARGET_MIN, TARGET_MAX)
}

pub fn analyze(input: &Input, target: i64) -> Plan {
    let target = target.clamp(TARGET_MIN, TARGET_MAX);
    let skills = tag_skill(input);

    // How often each tag appears among Codeforces problems just under the goal.
    let mut freq: HashMap<&str, usize> = HashMap::new();
    let mut band = 0usize;
    for p in input.problems.iter().filter(|p| p.platform == "codeforces" && p.rating > target - 300 && p.rating <= target) {
        band += 1;
        for t in &p.tags {
            *freq.entry(t.as_str()).or_insert(0) += 1;
        }
    }

    let mut topics: Vec<Topic> = freq
        .into_iter()
        .map(|(tag, n)| (tag, n as f64 / band.max(1) as f64))
        .filter(|&(_, share)| share >= MIN_SHARE)
        .map(|(tag, share)| {
            let skill = skills.get(tag).copied().unwrap_or(0);
            let status = if skill == 0 {
                Status::New
            } else if skill >= target - 100 {
                Status::Ready
            } else if skill >= target - 300 {
                Status::Close
            } else {
                Status::Weak
            };
            Topic { tag: tag.to_string(), share, skill, status }
        })
        .collect();

    // A topic's gap: how far the skill falls short of the goal, weighted by how common it is.
    let gap = |t: &Topic| t.share * (1.0 - (t.skill as f64 / target as f64).min(1.0));
    let total: f64 = topics.iter().map(|t| t.share).sum();
    let met: f64 = topics.iter().map(|t| t.share - gap(t)).sum();
    let readiness_pct = if total > 0.0 { (met / total * 100.0).round() as u32 } else { 0 };

    topics.sort_by(|a, b| gap(b).total_cmp(&gap(a)).then_with(|| a.tag.cmp(&b.tag)));
    let weight: HashMap<String, f64> = topics.iter().map(|t| (t.tag.clone(), gap(t))).collect();
    let focus: Vec<Topic> = topics.into_iter().filter(|t| t.status != Status::Ready).take(8).collect();

    Plan {
        target,
        target_rank: rank(target),
        rating: input.rating,
        readiness_pct,
        focus,
        steps: ladder(input, &weight, target),
    }
}

/// Rungs 100 apart from just above the current rating up to the goal (at most
/// MAX_RUNGS, the ones nearest the goal), each with unsolved problems that
/// train the biggest gaps, no more than two per topic.
fn ladder(input: &Input, weight: &HashMap<String, f64>, target: i64) -> Vec<Step> {
    let start = (input.rating / 100 * 100 + 100).clamp(800, target);
    let mut rungs: Vec<i64> = (start / 100..=target / 100).map(|r| r * 100).collect();
    if rungs.len() > MAX_RUNGS {
        rungs.drain(..rungs.len() - MAX_RUNGS);
    }

    let mut steps = Vec::new();
    for (i, &r) in rungs.iter().enumerate() {
        let mut cands: Vec<(f64, &Problem, String)> = input
            .problems
            .iter()
            .filter(|p| p.platform == "codeforces" && p.rating == r && !input.accepted.contains(&key(p)))
            .map(|p| {
                let (w, tag) = p
                    .tags
                    .iter()
                    .map(|t| (weight.get(t).copied().unwrap_or(0.0), t.clone()))
                    .max_by(|a, b| a.0.total_cmp(&b.0))
                    .unwrap_or((0.0, "misc".to_string()));
                (w, p, tag)
            })
            .collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| b.1.id.cmp(&a.1.id)));

        let mut per_tag: HashMap<String, usize> = HashMap::new();
        let mut taken = 0;
        for (_, p, tag) in cands {
            if taken == PER_RUNG {
                break;
            }
            let n = per_tag.entry(tag.clone()).or_insert(0);
            if *n == 2 {
                continue;
            }
            *n += 1;
            taken += 1;
            steps.push(Step { problem: p.clone(), tag, rung: i + 1, rungs: rungs.len() });
        }
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prob(id: &str, rating: i64, tags: &[&str]) -> Problem {
        Problem { platform: "codeforces".into(), id: id.into(), rating, tags: tags.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }

    #[test]
    fn plan_climbs_to_the_goal_and_targets_the_gaps() {
        let mut input = Input { rating: 1150, ..Default::default() };
        for i in 0..6 {
            input.problems.push(prob(&format!("{i}A"), 1300, &["greedy"]));
            input.accepted.insert(format!("codeforces{i}A"));
        }
        for r in (1100..=1400).step_by(100) {
            for i in 0..5 {
                input.problems.push(prob(&format!("{r}{i}B"), r, &["dp"]));
                input.problems.push(prob(&format!("{r}{i}C"), r, &["greedy"]));
            }
        }
        let plan = analyze(&input, 1400);
        assert_eq!(plan.target_rank, "Specialist");
        assert_eq!(plan.focus.first().map(|t| (t.tag.as_str(), t.status)), Some(("dp", Status::New)));
        assert!(!plan.focus.iter().any(|t| t.tag == "greedy"), "greedy at 1300 is ready for 1400");
        assert!(plan.readiness_pct > 0 && plan.readiness_pct < 100);
        let rungs: Vec<i64> = plan.steps.iter().map(|s| s.problem.rating).collect();
        assert_eq!((rungs.first(), rungs.last()), (Some(&1200), Some(&1400)));
        assert!(plan.steps.iter().all(|s| !input.accepted.contains(&key(&s.problem))));
        // dp is the gap, so each rung starts with dp problems.
        assert_eq!(plan.steps[0].tag, "dp");
    }

    #[test]
    fn milestones_cycle_and_clamp() {
        assert_eq!(next_milestone(1250), 1400);
        assert_eq!(cycle_milestone(1400, 1), 1600);
        assert_eq!(cycle_milestone(1400, -1), 1200);
        assert_eq!(cycle_milestone(1000, -1), 900);
        assert_eq!(cycle_milestone(3000, 1), 3100);
    }
}
