// The Config tab: text fields open an edit line, choice fields cycle through
// options on enter.

use crate::config::Config;
use crate::tui::text::{truncate, LLine};
use crate::tui::theme::{Theme, THEMES};

pub struct Field {
    pub label: &'static str,
    pub choice: bool,
    pub secret: bool,
}

pub const FIELDS: [Field; 6] = [
    Field { label: "Codeforces handle", choice: false, secret: false },
    Field { label: "Default language", choice: true, secret: false },
    Field { label: "Workspace folder", choice: false, secret: false },
    Field { label: "Editor command", choice: false, secret: false },
    Field { label: "Theme", choice: true, secret: false },
    Field { label: "CSES session cookie", choice: false, secret: true },
];

pub fn get(cfg: &Config, i: usize) -> String {
    match i {
        0 => cfg.handles.get("codeforces").cloned().unwrap_or_default(),
        1 => cfg.default_language.clone(),
        2 => cfg.workspace_dir.clone(),
        3 => cfg.editor.clone(),
        4 => cfg.theme.clone(),
        5 => cfg.cses_session.clone(),
        _ => String::new(),
    }
}

pub fn set(cfg: &mut Config, i: usize, v: String) {
    match i {
        0 => {
            cfg.handles.insert("codeforces".into(), v);
        }
        1 => cfg.default_language = v,
        2 => cfg.workspace_dir = v,
        3 => cfg.editor = v,
        4 => cfg.theme = v,
        5 => cfg.cses_session = v,
        _ => {}
    }
}

/// Options for a choice field, in cycle order. BTreeMap keys are already sorted.
pub fn options(cfg: &Config, i: usize) -> Vec<String> {
    match i {
        1 => cfg.compile_commands.keys().cloned().collect(),
        4 => THEMES.iter().map(|s| s.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// The value as the list displays it. Empty means not set.
pub fn shown(v: &str) -> String {
    if v.is_empty() {
        "(not set)".to_string()
    } else {
        v.to_string()
    }
}

/// Hides all but the last four characters of a secret.
pub fn mask(v: &str) -> String {
    if v.is_empty() {
        return "(not set)".to_string();
    }
    if v.chars().count() <= 4 {
        return "****".to_string();
    }
    let tail: String = v.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    format!("****{tail}")
}

#[allow(clippy::too_many_arguments)]
pub fn render(cfg: &Config, cursor: usize, editing: bool, edit_value: &str, width: usize, t: &Theme) -> Vec<LLine> {
    let mut out = Vec::new();
    out.push(LLine::new("Config", t.bold));
    out.push(LLine::new("  saved to config.json", t.muted));
    out.push(LLine::new(String::new(), t.text));

    for (i, f) in FIELDS.iter().enumerate() {
        let mut value = shown(&get(cfg, i));
        if f.secret {
            value = mask(&get(cfg, i));
        }
        if editing && i == cursor {
            value = edit_value.to_string();
        }
        let row = format!("{:<18} {}", f.label, value);
        let hint = if f.choice { "  (enter cycles)" } else { "" };
        if i == cursor {
            out.push(LLine::new(format!("▸ {}{}", truncate(&row, width.saturating_sub(4)), hint), t.select));
        } else {
            out.push(LLine::new(format!("  {}", truncate(&row, width.saturating_sub(4))), t.text));
        }
    }
    out.push(LLine::new(String::new(), t.text));
    out.push(LLine::new("enter edits a text field · esc cancels an edit · changes save immediately".to_string(), t.muted));
    out
}
