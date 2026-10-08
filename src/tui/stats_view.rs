// The Dashboard and Analytics tabs.

use crate::config::Config;
use crate::practice::{Pick, LADDER};
use crate::problem::{Problem, RatingChange, Submission};
use crate::stats;
use crate::tui::text::{truncate, LLine};
use crate::tui::theme::Theme;
use chrono::{Datelike, Duration, TimeZone, Utc};

const HEAT_SHADES: [&str; 5] = ["·", "░", "▒", "▓", "█"];

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

/// Filled and empty blocks for a 0..=1 fraction, width cells wide.
fn bar(frac: f64, width: usize) -> String {
    let n = ((frac.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    format!("{}{}", "█".repeat(n), "░".repeat(width - n))
}

pub fn render_dashboard(cfg: &Config, problems: &[Problem], subs: &[Submission], ratings: &[RatingChange], up_next: &[Pick], width: usize, t: &Theme) -> Vec<LLine> {
    let now = Utc::now();
    let rating = current_rating(ratings);

    let mut out = Vec::new();
    out.push(LLine::new(format!("Dashboard  codeforces: {}", handle(cfg)), t.bold));
    out.push(LLine::new(String::new(), t.text));

    if rating > 0 {
        out.push(LLine::new(format!("{rating}  {}", stats::rank(rating)), t.title));
        let prev = LADDER.iter().rev().find(|&&r| r <= rating).copied().unwrap_or(0);
        match LADDER.iter().find(|&&r| r > rating) {
            Some(&next) => {
                let frac = (rating - prev) as f64 / (next - prev) as f64;
                out.push(LLine::new(
                    format!("{}  {} to {} at {next}", bar(frac, width.saturating_sub(34).clamp(10, 30)), next - rating, stats::rank(next)),
                    t.sub,
                ));
            }
            None => out.push(LLine::new("Top rank reached.", t.sub)),
        }
    } else {
        out.push(LLine::new("Unrated: take part in a rated Codeforces round to get a rating.", t.muted));
    }
    out.push(LLine::new(String::new(), t.text));

    let accepted = subs.iter().filter(|s| s.verdict == "OK").count();
    let week_ago = (now - Duration::days(7)).timestamp();
    out.push(LLine::new(
        format!(
            "{accepted} accepted   {} this week   {} day streak",
            stats::accepted_since(subs, week_ago),
            stats::streak(&stats::activity(subs), now)
        ),
        t.text,
    ));

    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("Up next", t.bold));
    if up_next.is_empty() {
        out.push(LLine::new("No picks yet. Sync problems and submissions first (r).", t.muted));
    }
    for pk in up_next {
        let p = &pk.problem;
        let reason = pk.reasons.first().cloned().unwrap_or_default();
        out.push(LLine::new(
            truncate(&format!("  {:<8} {:>5}  {}  ({reason})", p.id, or_dash(p.rating), p.name), width),
            t.text,
        ));
    }

    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("Weak topics  accepted / attempted", t.bold));
    let mut weak: Vec<_> = stats::topics(problems, subs).into_iter().filter(|tp| tp.attempted >= 2 && tp.accepted < tp.attempted).collect();
    weak.sort_by(|a, b| (a.accepted * b.attempted).cmp(&(b.accepted * a.attempted)).then(b.attempted.cmp(&a.attempted)));
    if weak.is_empty() {
        out.push(LLine::new("Nothing stands out yet.", t.muted));
    }
    for tp in weak.iter().take(5) {
        let frac = tp.accepted as f64 / tp.attempted as f64;
        out.push(LLine::new(format!("  {:<22} {} {:>3}/{:<3}", truncate(&tp.tag, 22), bar(frac, 12), tp.accepted, tp.attempted), t.text));
    }

    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("Recent submissions", t.bold));
    if subs.is_empty() {
        out.push(LLine::new("None yet. Set your handle in Config, then refresh (r).", t.muted));
        return out;
    }
    for s in subs.iter().take(8) {
        let when = Utc
            .timestamp_opt(s.submitted_at, 0)
            .single()
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        let (verdict, style) = if s.verdict == "OK" { ("OK".to_string(), t.pass) } else { (truncate(&s.verdict, 14), t.fail) };
        out.push(LLine::new(format!("  {verdict:<14} {:<8} {when}", truncate(&s.problem_id, 8)), style));
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
        // Start from the first contest's old rating so even one contest draws a rise.
        let vals: Vec<i64> = std::iter::once(ratings[0].old_rating).chain(ratings.iter().map(|r| r.new_rating)).collect();
        let shown = &vals[vals.len().saturating_sub(width.saturating_sub(7).max(1))..];
        let (lo, hi) = (shown.iter().min().copied().unwrap_or(0), shown.iter().max().copied().unwrap_or(0));
        let rows = stats::chart(&vals, 8, width.saturating_sub(7).max(1));
        let last = rows.len().saturating_sub(1);
        for (i, row) in rows.into_iter().enumerate() {
            let label = if i == 0 { hi.to_string() } else if i == last && lo != hi { lo.to_string() } else { String::new() };
            out.push(LLine::new(format!("{label:>5} │{row}"), t.title));
        }
        out.push(LLine::new(format!("{} rated contest{}, now {}", ratings.len(), if ratings.len() == 1 { "" } else { "s" }, stats::rank(*vals.last().unwrap())), t.sub));
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

/// The last 12 weeks as columns (oldest first) with one row per weekday,
/// Monday on top. Days after today stay blank.
fn heatmap(subs: &[Submission], now: chrono::DateTime<Utc>, t: &Theme) -> Vec<LLine> {
    let counts = stats::activity(subs);
    let today = now.date_naive();
    let start = today - Duration::days(today.weekday().num_days_from_monday() as i64 + 7 * 11);
    let labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut rows: [String; 7] = Default::default();
    for i in 0..84 {
        let day = start + Duration::days(i);
        let shade = if day > today { " " } else { HEAT_SHADES[stats::heat_level(counts.get(&day).copied().unwrap_or(0))] };
        rows[day.weekday().num_days_from_monday() as usize].push_str(shade);
        rows[day.weekday().num_days_from_monday() as usize].push(' ');
    }
    rows.into_iter().enumerate().map(|(i, row)| LLine::new(format!("{} {row}", labels[i]), t.text)).collect()
}
