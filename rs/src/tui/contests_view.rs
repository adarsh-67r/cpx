// The Contests tab: upcoming and running Codeforces contests, soonest first.

use crate::problem::Contest;
use crate::tui::text::{truncate, LLine};
use crate::tui::theme::Theme;

/// Cached contests that have not finished yet, soonest first (the cache
/// already orders by start_time).
pub fn upcoming(contests: &[Contest], now: i64) -> Vec<Contest> {
    contests.iter().filter(|k| k.start + k.duration > now).cloned().collect()
}

/// Describes when a contest starts, or that it is running.
fn countdown(k: &Contest, now: i64) -> String {
    if now >= k.start {
        return "running now".to_string();
    }
    let secs = k.start - now;
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("in {days}d {hours}h")
    } else if hours > 0 {
        format!("in {hours}h {mins}m")
    } else {
        format!("in {mins}m")
    }
}

fn format_start(unix: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(unix, 0)
        .single()
        .map(|t| t.format("%a %d %b %H:%M").to_string())
        .unwrap_or_default()
}

pub fn render(contests: &[Contest], cursor: usize, now: i64, width: usize, t: &Theme) -> Vec<LLine> {
    let mut out = Vec::new();
    out.push(LLine::new("Contests", t.bold));
    out.push(LLine::new("  codeforces · upcoming and running", t.muted));
    out.push(LLine::new(String::new(), t.text));

    let list = upcoming(contests, now);
    if list.is_empty() {
        out.push(LLine::new("No upcoming contests cached. Run cpx sync.", t.muted));
        return out;
    }

    let name_width = width.saturating_sub(44).max(10);
    for (i, k) in list.iter().enumerate() {
        let start = format_start(k.start);
        let dur = format!("{}h{:02}m", k.duration / 3600, (k.duration % 3600) / 60);
        let row = format!(
            "{:<nw$} {:<16} {:>6}  {}",
            truncate(&k.name, name_width),
            start,
            dur,
            countdown(k, now),
            nw = name_width
        );
        if i == cursor {
            out.push(LLine::new(format!("▸ {row}"), t.select));
        } else {
            out.push(LLine::new(format!("  {row}"), t.text));
        }
    }
    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("enter / b opens the contest page on codeforces.com", t.muted));
    out
}
