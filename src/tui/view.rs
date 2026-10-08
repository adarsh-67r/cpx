// Drawing: header, tab bar, the list/detail split or a full-width report
// pane, the status line, and the keys hint.

use crate::practice;
use crate::problem::Problem;
use crate::tui::text::{self, LLine};
use crate::tui::theme::{self, Theme};
use crate::tui::{config_view, contests_view, statement_view, stats_view};
use crate::tui::{Model, Tab};
use chrono::Utc;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

const KEYS_HINT: &str = "j/k move · tab switch · / filter · p judge · c clear · m mode · o open · t test · s submit · v statement · d/u scroll · b browser · r refresh · T theme · q quit";
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(f: &mut Frame, m: &Model) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(m.theme.bg)), area);
    if m.width == 0 || m.height == 0 {
        return;
    }

    let (header_lines, header_h) = build_header(m);
    let status = match m.job {
        Some(_) => format!("{} {}", SPINNER[m.spin % SPINNER.len()], m.status),
        None => m.status.clone(),
    };
    let status_lines = text::wrap(&status, m.width as usize);
    let footer_h = status_lines.len().max(1) as u16;
    let keys_lines = text::wrap(KEYS_HINT, m.width as usize);
    let keys_h = keys_lines.len().max(1) as u16;

    let with_keys = m.height.saturating_sub(header_h).saturating_sub(footer_h).saturating_sub(keys_h);
    let show_keys = with_keys >= 3;
    let mut body_h = if show_keys { with_keys } else { m.height.saturating_sub(header_h).saturating_sub(footer_h) };
    if body_h < 3 {
        body_h = 3;
    }

    let mut constraints = vec![Constraint::Length(header_h), Constraint::Length(body_h), Constraint::Length(footer_h)];
    if show_keys {
        constraints.push(Constraint::Length(keys_h));
    }
    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    f.render_widget(Paragraph::new(header_lines), chunks[0]);
    render_body(f, chunks[1], m);
    render_muted(f, chunks[2], &status_lines, &m.theme);
    if show_keys {
        render_muted(f, chunks[3], &keys_lines, &m.theme);
    }
}

fn render_muted(f: &mut Frame, area: Rect, lines: &[String], t: &Theme) {
    let rl: Vec<Line> = lines.iter().map(|l| Line::from(Span::styled(l.clone(), t.muted))).collect();
    f.render_widget(Paragraph::new(rl), area);
}

/// The title line and tab bar, plus the filter line when it is open.
fn build_header(m: &Model) -> (Vec<Line<'static>>, u16) {
    let t = &m.theme;
    let mut spans = vec![Span::styled("CPX", t.title), Span::raw("  ")];
    let tabs = ["Problems", "Practice", "Goal", "Contests", "Analytics", "Dashboard", "Config"];
    for (i, name) in tabs.iter().enumerate() {
        if i == m.tab.index() {
            spans.push(Span::styled(format!("[{name}]"), t.active));
        } else {
            spans.push(Span::styled(format!(" {name} "), t.muted));
        }
        spans.push(Span::raw(" "));
    }
    if m.tab == Tab::Practice {
        let rating = m.deps.rating_changes.last().map(|r| r.new_rating).unwrap_or(0);
        let target = if m.target > 0 { m.target } else { practice::default_target(rating) };
        spans.push(Span::styled(format!("  mode: {}  target: {target}  [ ] change", practice::MODES[m.mode].label()), t.muted));
    } else if let (Tab::Goal, Some(p)) = (m.tab, &m.plan) {
        spans.push(Span::styled(format!("  goal: {} {}  ready {}%  [ ] change", p.target, p.target_rank, p.readiness_pct), t.muted));
    } else if m.tab == Tab::Problems {
        spans.push(Span::styled(format!("  {}/{}", m.visible.len(), m.deps.problems.len()), t.muted));
    }

    let mut lines = vec![Line::from(spans)];
    let mut h = 1u16;
    if m.tab == Tab::Problems && (m.filtering || !m.filter.is_empty()) {
        let mut spans = vec![Span::styled(format!("/ {}", m.filter), t.text)];
        if m.filtering {
            spans.push(Span::styled("  @cf @cses @atcoder · 1200-1600 · #dp,greedy · words", t.muted));
        }
        lines.push(Line::from(spans));
        h += 1;
    }
    (lines, h)
}

fn render_body(f: &mut Frame, area: Rect, m: &Model) {
    let t = &m.theme;

    if m.stmt_open {
        let w = area.width.saturating_sub(2).min(area.width);
        render_statement_pane(f, Rect { width: w, ..area }, m);
        return;
    }

    if m.is_stats_tab() {
        let w = area.width.saturating_sub(2);
        let pane_area = Rect { width: w, ..area };
        let text_w = (w as usize).saturating_sub(4);
        let report = match m.tab {
            Tab::Dashboard => stats_view::render_dashboard(&m.deps.config, &m.deps.problems, &m.deps.submissions, &m.deps.rating_changes, &m.up_next, text_w, t),
            Tab::Contests => contests_view::render(&m.deps.contests, m.cursor, Utc::now().timestamp(), text_w, t),
            Tab::Config => config_view::render(&m.deps.config, m.cursor, m.cfg_editing, &m.cfg_input, text_w, t),
            _ => stats_view::render_analytics(&m.deps.config, &m.deps.problems, &m.deps.submissions, &m.deps.rating_changes, text_w, t),
        };
        render_pane(f, pane_area, &report, m.right_scroll(), t);
        return;
    }

    if area.width < 90 {
        let list_h = (area.height / 2).max(3);
        let detail_h = (area.height.saturating_sub(list_h)).max(3);
        let box_w = area.width.saturating_sub(4);
        // The max(3) floors can push the boxes past a tiny terminal; ratatui
        // panics on out-of-buffer draws where lipgloss just clipped.
        let list_area = Rect { x: area.x, y: area.y, width: box_w, height: list_h }.intersection(area);
        let detail_area = Rect { x: area.x, y: area.y + list_h, width: box_w, height: detail_h }.intersection(area);
        render_list_pane(f, list_area, m, t);
        let detail_lines = render_detail(m, t, (box_w as usize).saturating_sub(4));
        render_pane(f, detail_area, &detail_lines, m.right_scroll(), t);
    } else {
        let list_w = area.width / 2 - 2;
        let detail_w = area.width.saturating_sub(list_w).saturating_sub(4);
        let list_area = Rect { x: area.x, y: area.y, width: list_w, height: area.height };
        let detail_area = Rect { x: area.x + list_w, y: area.y, width: detail_w, height: area.height };
        render_list_pane(f, list_area, m, t);
        let detail_lines = render_detail(m, t, (detail_w as usize).saturating_sub(4));
        render_pane(f, detail_area, &detail_lines, m.right_scroll(), t);
    }
}

fn bordered(t: &Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(t.pane_border)
        .style(Style::default().bg(t.bg))
        .padding(Padding::horizontal(1))
}

fn render_list_pane(f: &mut Frame, area: Rect, m: &Model, t: &Theme) {
    let block = bordered(t);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let n = m.row_count();
    if n == 0 {
        let msg = match m.tab {
            Tab::Practice => "No picks for this mode. Press m for another.",
            Tab::Goal => "No unsolved Codeforces problems for this goal. Sync (r) or press [ ] for another goal.",
            _ => "No matches.",
        };
        f.render_widget(Paragraph::new(Line::from(Span::styled(msg, t.muted))), inner);
        return;
    }

    let width = (inner.width as usize).saturating_sub(2);
    let items: Vec<ListItem> = (0..n)
        .map(|i| {
            let p = m.row_problem(i);
            let style = if i == m.cursor { t.select } else { t.text };
            let prefix = if i == m.cursor { "▸ " } else { "  " };
            let rating_style = if t.rating_colors { style.fg(theme::rating_color(p.rating)) } else { style };
            let rest = text::truncate(&format!(" {}", p.name), width.saturating_sub(18));
            let (mark, mark_style) = match m.solved.get(&practice::key(&p)) {
                Some(true) => ("● ", style.fg(t.pass.fg.unwrap_or_default())),
                Some(false) => ("◐ ", style.fg(t.active.fg.unwrap_or_default())),
                None => ("○ ", style.fg(t.muted.fg.unwrap_or_default())),
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(mark, mark_style),
                Span::styled(format!("{:<8} ", p.id), style),
                Span::styled(format!("{:<6}", rating_label(&p)), rating_style),
                Span::styled(rest, style),
            ]))
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(m.cursor));
    f.render_stateful_widget(List::new(items), inner, &mut state);
}

fn render_pane(f: &mut Frame, area: Rect, lines: &[LLine], offset: i64, t: &Theme) {
    let block = bordered(t);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let width = (inner.width as usize).max(1);
    let wrapped = text::wrap_llines(lines, width);
    let inner_h = inner.height as usize;
    let max_off = (wrapped.len() as i64 - inner_h as i64).max(0);
    let off = offset.clamp(0, max_off) as usize;
    let end = (off + inner_h).min(wrapped.len());
    f.render_widget(Paragraph::new(text::llines_to_ratatui(&wrapped[off..end])), inner);
}

fn render_statement_pane(f: &mut Frame, area: Rect, m: &Model) {
    let t = &m.theme;
    let title = match m.statement_problem() {
        Some(p) => format!("Statement — {} {}", p.id, p.name),
        None => "Statement".to_string(),
    };
    let lines = statement_view::render(&m.stmt_blocks, &m.stmt_samples, m.stmt_scroll, &title, area.width as usize, area.height as usize, t);
    let block = bordered(t);
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(text::llines_to_ratatui(&lines)), inner);
}

fn render_detail(m: &Model, t: &Theme, width: usize) -> Vec<LLine> {
    let Some(p) = m.selected() else {
        return vec![LLine::new("Select a problem.", t.muted)];
    };

    let mut out = Vec::new();
    out.push(LLine::new(p.name.clone(), t.bold));
    out.push(LLine::new(format!("{} · {} · rating {}", p.platform, p.id, rating_label(&p)), t.sub));
    out.push(LLine::new(p.url.clone(), t.muted));
    if !p.tags.is_empty() {
        out.push(LLine::new(p.tags.join(", "), t.sub));
    }

    if m.tab == Tab::Practice || m.tab == Tab::Goal {
        out.push(LLine::new(String::new(), t.text));
        out.push(LLine::new("Why this pick", t.bold));
        for r in &m.picks[m.cursor].reasons {
            out.push(LLine::new(format!("· {r}"), t.sub));
        }
    }

    if let (Tab::Goal, Some(plan)) = (m.tab, &m.plan) {
        out.push(LLine::new(String::new(), t.text));
        let rating = if plan.rating > 0 { format!("rating {}", plan.rating) } else { "unrated".to_string() };
        out.push(LLine::new(format!("Goal {} {}  ·  {rating}  ·  {}% ready", plan.target, plan.target_rank, plan.readiness_pct), t.bold));
        out.push(LLine::new(String::new(), t.text));
        out.push(LLine::new("Topics to work on  (skill · share of problems near the goal)", t.bold));
        if plan.focus.is_empty() {
            out.push(LLine::new("Every common topic at this goal looks ready.", t.muted));
        }
        for tp in &plan.focus {
            let skill = if tp.skill > 0 { tp.skill.to_string() } else { "—".to_string() };
            out.push(LLine::new(
                text::truncate(&format!("  {:<24} {:<6} {:>5} {:>4.0}%", tp.tag, tp.status.label(), skill, tp.share * 100.0), width),
                t.text,
            ));
        }
    }

    if m.ran_id == p.id && !m.results.is_empty() {
        out.push(LLine::new(String::new(), t.text));
        out.push(LLine::new("Last run", t.bold));
        out.extend(render_results(m, t));
    }
    let _ = width;
    out
}

fn render_results(m: &Model, t: &Theme) -> Vec<LLine> {
    let mut out = Vec::new();
    for r in &m.results {
        if r.passed {
            out.push(LLine::new(format!("✓ sample {}  {}ms", r.index, r.duration.as_millis()), t.pass));
            continue;
        }
        let reason = r.error.as_deref().map(first_line).unwrap_or_else(|| "wrong answer".to_string());
        out.push(LLine::new(format!("✗ sample {}  {reason}", r.index), t.fail));
        if r.error.is_none() {
            out.push(LLine::new(format!("  expected {}", first_line(&r.expected)), t.sub));
            out.push(LLine::new(format!("  got      {}", first_line(&r.actual)), t.sub));
        }
    }
    out
}

fn first_line(s: &str) -> String {
    s.trim().split('\n').next().unwrap_or("").to_string()
}

fn rating_label(p: &Problem) -> String {
    if p.rating == 0 {
        "—".to_string()
    } else {
        p.rating.to_string()
    }
}
