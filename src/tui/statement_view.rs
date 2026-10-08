// The full-width statement pane: cpos's block/spacing model rendered as
// styled lines, with a cached copy on disk per problem.

use crate::config::Config;
use crate::judge::statement::{self, Block};
use crate::problem::{Problem, Sample};
use crate::tui::text::{center, truncate, wrap, LLine};
use crate::tui::theme::Theme;
use crate::workspace;
use std::path::Path;

/// Loads the statement for p: a cached copy from the config dir if present,
/// otherwise fetched from the judge and cached there. Samples come from the
/// solution folder, when the problem has been opened.
pub fn load(config_dir: &Path, root: &Path, cfg: &Config, p: &Problem) -> (Vec<Block>, Vec<Sample>, Option<String>) {
    let dir = config_dir.join("statements");
    let key = format!("{}{}", p.platform, p.id);
    // v2: the parser now keeps CSES math, images, and drops sample sections.
    let path = dir.join(format!("{key}.v2.json"));

    let ext = cfg.compile_commands.get(&cfg.default_language).map(|c| c.extension.as_str()).filter(|e| !e.is_empty()).unwrap_or(".cpp");
    let solution = workspace::solution_path(root, p, ext);
    let samples = workspace::load_samples(&solution).unwrap_or_default();

    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(blocks) = serde_json::from_str::<Vec<Block>>(&text) {
            return (blocks, samples, None);
        }
    }
    match statement::fetch(p) {
        Err(e) => (Vec::new(), samples, Some(e.to_string())),
        Ok(blocks) => {
            if std::fs::create_dir_all(&dir).is_ok() {
                if let Ok(data) = serde_json::to_string_pretty(&blocks) {
                    let _ = std::fs::write(&path, data);
                }
            }
            (blocks, samples, None)
        }
    }
}

fn spacer(out: &mut Vec<LLine>, text_style: ratatui::style::Style) {
    if out.last().map(|l| !l.text.is_empty()).unwrap_or(false) {
        out.push(LLine::new(String::new(), text_style));
    }
}

/// Lays out the statement blocks with cpos's spacing: one blank line between
/// blocks, headings with a rule, centered titles and metadata, and the sample
/// boxes before the Note section.
pub fn build_page(blocks: &[Block], samples: &[Sample], width: usize, t: &Theme) -> Vec<LLine> {
    let width = width.max(12);
    let mut out: Vec<LLine> = Vec::new();
    let rule = "─".repeat(width);

    let mut samples_done = false;
    let mut prev_kind = String::new();
    let mut item_no = 0;
    for b in blocks {
        match b.kind.as_str() {
            statement::TITLE => {
                spacer(&mut out, t.text);
                for l in wrap(&b.text, width) {
                    out.push(LLine::new(center(&l, width), t.bold));
                }
                spacer(&mut out, t.text);
            }
            statement::META => {
                for l in wrap(&b.text, width) {
                    out.push(LLine::new(center(&l, width), t.muted));
                }
            }
            statement::HEADING => {
                if b.text.eq_ignore_ascii_case("note") && !samples_done && !samples.is_empty() {
                    spacer(&mut out, t.text);
                    out.extend(samples_lines(samples, width, t));
                    samples_done = true;
                }
                spacer(&mut out, t.text);
                for l in wrap(&b.text, width) {
                    out.push(LLine::new(l, t.bold));
                }
                out.push(LLine::new(rule.clone(), t.muted));
            }
            statement::PARAGRAPH => {
                for l in wrap(&b.text, width) {
                    out.push(LLine::new(l, t.text));
                }
                spacer(&mut out, t.text);
            }
            statement::MATH => {
                spacer(&mut out, t.text);
                for l in b.text.split('\n') {
                    out.push(LLine::new(center(l, width), t.text));
                }
                spacer(&mut out, t.text);
            }
            statement::ITEM => {
                if prev_kind != statement::ITEM {
                    spacer(&mut out, t.text);
                    item_no = 0;
                }
                item_no += 1;
                let prefix = if b.ordered { format!("{item_no}. ") } else { "• ".to_string() };
                let wrapped = wrap(&b.text, width.saturating_sub(prefix.chars().count()).max(1));
                for (i, l) in wrapped.iter().enumerate() {
                    if i == 0 {
                        out.push(LLine::new(format!("{prefix}{l}"), t.text));
                    } else {
                        out.push(LLine::new(format!("{}{l}", " ".repeat(prefix.chars().count())), t.text));
                    }
                }
            }
            statement::IMAGE => {
                spacer(&mut out, t.text);
                out.push(LLine::new("▣ Image (b opens the problem in the browser)", t.sub));
                out.push(LLine::new(truncate(&b.text, width), t.muted));
                spacer(&mut out, t.text);
            }
            statement::CODE => {
                spacer(&mut out, t.text);
                for l in b.text.split('\n') {
                    out.push(LLine::new(format!("    {}", truncate(l, width.saturating_sub(4))), t.sub));
                }
                spacer(&mut out, t.text);
            }
            _ => {}
        }
        prev_kind = b.kind.clone();
    }
    if !samples_done && !samples.is_empty() {
        spacer(&mut out, t.text);
        out.extend(samples_lines(samples, width, t));
    }

    while out.last().map(|l| l.text.is_empty()).unwrap_or(false) {
        out.pop();
    }
    while out.first().map(|l| l.text.is_empty()).unwrap_or(false) {
        out.remove(0);
    }
    out
}

fn samples_lines(samples: &[Sample], width: usize, t: &Theme) -> Vec<LLine> {
    let mut out = Vec::new();
    out.push(LLine::new("Samples", t.bold));
    out.push(LLine::new("─".repeat(width), t.muted));
    for (i, s) in samples.iter().enumerate() {
        out.push(LLine::new(format!("Input {}", i + 1), t.bold));
        for l in s.input.split('\n') {
            out.push(LLine::new(format!("    {}", truncate(l, width.saturating_sub(4))), t.sub));
        }
        out.push(LLine::new(format!("Output {}", i + 1), t.bold));
        for l in s.output.split('\n') {
            out.push(LLine::new(format!("    {}", truncate(l, width.saturating_sub(4))), t.sub));
        }
        out.push(LLine::new(String::new(), t.text));
    }
    out
}

/// The title line and scroll percentage, then the page lines sliced to the
/// viewport. w and h are the outer box size (border included), matching
/// pane()'s convention.
pub fn render(blocks: &[Block], samples: &[Sample], scroll: i64, title: &str, w: usize, h: usize, t: &Theme) -> Vec<LLine> {
    let width = w.saturating_sub(4).max(1);
    let lines = build_page(blocks, samples, width, t);

    let viewport = (h as i64 - 3).max(1); // border (2) and the title line
    let max_scroll = (lines.len() as i64 - viewport).max(0);
    let off = scroll.clamp(0, max_scroll);
    let pct = if max_scroll > 0 { off * 100 / max_scroll } else { 100 };

    let head = format!("{}  j/k scroll · d/u page · b browser · v back · {pct}%", truncate(title, width));
    let mut out = vec![LLine::new(head, t.bold)];
    let end = (off + viewport).min(lines.len() as i64) as usize;
    out.extend_from_slice(&lines[off as usize..end]);
    out
}
