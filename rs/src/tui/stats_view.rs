// The Dashboard and Analytics tabs.

use crate::config::Config;
use crate::practice::LADDER;
use crate::problem::{Problem, RatingChange, Submission};
use crate::stats;
use crate::tui::text::{truncate, LLine};
use crate::tui::theme::Theme;
use chrono::{Duration, TimeZone, Utc};

const HEAT_SHADES: [&str; 5] = [" ", "░", "▒", "▓", "█"];

/// The configured Codeforces handle, or a hint to set one.
fn handle(cfg: &Config) -> String {
    match cfg.handles.get("codeforces") {
        Some(h) if !h.is_empty() => h.clone(),
        _ => "not set (add a Codeforces handle to config.json, then cpx sync)".to_string(),
    }
}

fn or_dash(n: i64) -> String {
    if n == 0 {
        "—".to_string()
    } else {
        n.to_string()
    }
}

pub fn current_rating(ratings: &[RatingChange]) -> i64 {
    ratings.last().map(|r| r.new_rating).unwrap_or(0)
}

pub fn render_dashboard(cfg: &Config, subs: &[Submission], ratings: &[RatingChange], t: &Theme) -> Vec<LLine> {
    let now = Utc::now();
    let rating = current_rating(ratings);

    let mut out = Vec::new();
    out.push(LLine::new(format!("Dashboard  codeforces: {}", handle(cfg)), t.bold));
    out.push(LLine::new(String::new(), t.text));

    let accepted = subs.iter().filter(|s| s.verdict == "OK").count();
    let next_rung = LADDER.iter().find(|&&r| r > rating).map(|r| format!("{r} (+{})", r - rating)).unwrap_or_else(|| "—".to_string());
    let week_ago = (now - Duration::days(7)).timestamp();

    out.push(LLine::new(format!("Rating       {}", or_dash(rating)), t.text));
    out.push(LLine::new(format!("Next rung    {next_rung}"), t.text));
    out.push(LLine::new(
        format!("Accepted     {accepted} total, {} in the last 7 days", stats::accepted_since(subs, week_ago)),
        t.text,
    ));
    out.push(LLine::new(format!("Streak       {} day(s)", stats::streak(&stats::activity(subs), now)), t.text));

    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("Recent submissions", t.bold));
    if subs.is_empty() {
        out.push(LLine::new("None yet. Run cpx sync with your handle set.", t.muted));
        return out;
    }
    for s in subs.iter().take(8) {
        let when = Utc
            .timestamp_opt(s.submitted_at, 0)
            .single()
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        let (verdict, style) = if s.verdict == "OK" { ("OK".to_string(), t.pass) } else { (truncate(&s.verdict, 14), t.fail) };
        out.push(LLine::new(format!("{verdict:<14} {:<8} {when}", truncate(&s.problem_id, 8)), style));
    }
    out
}

pub fn render_analytics(cfg: &Config, problems: &[Problem], subs: &[Submission], ratings: &[RatingChange], width: usize, t: &Theme) -> Vec<LLine> {
    let now = Utc::now();
    let mut out = Vec::new();
    out.push(LLine::new(format!("Analytics  codeforces: {}", handle(cfg)), t.bold));
    out.push(LLine::new(String::new(), t.text));

    out.push(LLine::new("Rating history", t.bold));
    if ratings.is_empty() {
        out.push(LLine::new("No rated contests yet.", t.muted));
    } else {
        let vals: Vec<i64> = ratings.iter().map(|r| r.new_rating).collect();
        out.push(LLine::new(format!("{}  {} contests", stats::sparkline(&vals), ratings.len()), t.text));
        let start = ratings.len().saturating_sub(5);
        for r in &ratings[start..] {
            out.push(LLine::new(
                format!("  {}  {} → {}", truncate(&r.contest_name, width.saturating_sub(20)), r.old_rating, r.new_rating),
                t.sub,
            ));
        }
    }

    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("Activity, last 12 weeks", t.bold));
    out.extend(heatmap(subs, now, t));

    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("Top topics  accepted · attempted · avg rating", t.bold));
    let topics = stats::topics(problems, subs);
    if topics.is_empty() {
        out.push(LLine::new("No attempts yet.", t.muted));
    }
    for tp in topics.iter().take(10) {
        out.push(LLine::new(
            format!("  {:<22} {:>4} {:>4} {:>5}", truncate(&tp.tag, 22), tp.accepted, tp.attempted, or_dash(tp.avg_rating)),
            t.text,
        ));
    }
    out
}

/// The last 84 days as 12 columns of 7 rows (Mon..Sun), oldest first.
fn heatmap(subs: &[Submission], now: chrono::DateTime<Utc>, t: &Theme) -> Vec<LLine> {
    let counts = stats::activity(subs);
    let start = now.date_naive() - Duration::days(83);
    let labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut rows: [String; 7] = Default::default();
    for i in 0..84 {
        let day = start + Duration::days(i);
        let n = counts.get(&day).copied().unwrap_or(0);
        let col = i as usize % 7;
        rows[col].push_str(HEAT_SHADES[stats::heat_level(n)]);
        rows[col].push(' ');
    }
    rows.into_iter().enumerate().map(|(i, row)| LLine::new(format!("{} {row}", labels[i]), t.text)).collect()
}
