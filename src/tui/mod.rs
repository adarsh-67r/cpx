// The CPX terminal interface: a Problems tab with filter, a Practice tab with
// the seven recommendation modes, Contests/Analytics/Dashboard/Config report
// tabs, and keys to open, run, submit, and view a problem's statement.
//
// Slow actions (open, run, statement, refresh) run on a background thread and
// report back over a channel; the loop wakes every 100ms to collect results
// and spin the busy indicator, so the interface never freezes. One job runs
// at a time.

mod config_view;
mod contests_view;
mod query;
mod statement_view;
mod stats_view;
mod system;
mod text;
mod theme;
mod view;

use crate::cache::Cache;
use crate::config::{self, Config};
use crate::judge::dispatch;
use crate::practice;
use crate::problem::{Contest, Problem, RatingChange, Submission};
use crate::runner::{self, CaseResult};
use crate::workspace;
use anyhow::Result;
use chrono::Utc;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration as StdDuration;

/// What a background job sends back.
enum Msg {
    Status(String),
    Opened(Result<(PathBuf, usize)>),
    Ran(String, Result<Vec<CaseResult>>),
    Statement(String, Vec<crate::judge::statement::Block>, Vec<crate::problem::Sample>, Option<String>),
    Synced(Result<Deps>),
}
use theme::Theme;

/// Everything the UI needs from the rest of CPX.
pub struct Deps {
    pub problems: Vec<Problem>,
    pub submissions: Vec<Submission>,
    pub rating_changes: Vec<RatingChange>,
    pub contests: Vec<Contest>,
    /// CSES task ids the user has solved.
    pub cses_solved: Vec<String>,
    pub config: Config,
    /// Holds config.json, templates/, build/, statements/.
    pub config_dir: PathBuf,
    /// config.json itself; empty means changes are not saved.
    pub config_path: PathBuf,
    /// Workspace root for solution files.
    pub root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Problems,
    Practice,
    Goal,
    Contests,
    Analytics,
    Dashboard,
    Config,
}

const TABS: [Tab; 7] = [Tab::Problems, Tab::Practice, Tab::Goal, Tab::Contests, Tab::Analytics, Tab::Dashboard, Tab::Config];

impl Tab {
    fn index(self) -> usize {
        TABS.iter().position(|t| *t == self).unwrap_or(0)
    }
    fn next(self) -> Tab {
        TABS[(self.index() + 1) % TABS.len()]
    }
}

pub struct Model {
    deps: Deps,

    tab: Tab,
    visible: Vec<usize>,
    picks: Vec<practice::Pick>,
    /// The Dashboard's "Up next": top auto picks, refreshed with the data.
    up_next: Vec<practice::Pick>,
    mode: usize,
    /// Practice target rating set with [ and ]; 0 follows the current rating.
    target: i64,
    /// Goal tab rating set with [ and ]; 0 means the next rank milestone.
    goal: i64,
    plan: Option<crate::target::Plan>,
    /// Problem key to true when accepted, false when only attempted.
    solved: std::collections::HashMap<String, bool>,
    cursor: usize,

    filtering: bool,
    filter: String,

    status: String,

    ran_id: String,
    results: Vec<CaseResult>,

    cfg_editing: bool,
    cfg_input: String,

    stmt_open: bool,
    stmt_key: String,
    stmt_blocks: Vec<crate::judge::statement::Block>,
    stmt_samples: Vec<crate::problem::Sample>,
    stmt_scroll: i64,
    detail_scroll: i64,

    width: u16,
    height: u16,
    theme: Theme,
    quit: bool,

    job: Option<Receiver<Msg>>,
    spin: usize,
}

/// Starts the interface and blocks until the user quits.
pub fn run(deps: Deps) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut model = Model::new(deps);
    let result = run_loop(&mut terminal, &mut model);

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, model: &mut Model) -> Result<()> {
    loop {
        let size = terminal.size()?;
        model.width = size.width;
        model.height = size.height;
        terminal.draw(|f| view::draw(f, model))?;

        if event::poll(StdDuration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    model.handle_key(key);
                }
            }
        }
        model.poll_job();
        if model.quit {
            break;
        }
    }
    Ok(())
}

/// Maps a crossterm key to the token bubbletea's msg.String() would have
/// produced, so the key-handling logic below reads the same as the Go
/// version's switch statements.
fn key_token(key: KeyEvent) -> String {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            return format!("ctrl+{c}");
        }
    }
    match key.code {
        KeyCode::Esc => "esc".into(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::PageUp => "pgup".into(),
        KeyCode::PageDown => "pgdown".into(),
        KeyCode::Backspace => "backspace".into(),
        KeyCode::Char(c) => c.to_string(),
        _ => String::new(),
    }
}

impl Model {
    fn new(deps: Deps) -> Self {
        let theme = theme::apply(&deps.config.theme);
        let status = if deps.problems.is_empty() {
            "No problems cached. Quit and run: cpx sync".to_string()
        } else {
            "j/k move · / filter · tab practice · o open · t test · s submit · b browser · q quit".to_string()
        };
        let mut m = Model {
            deps,
            tab: Tab::Problems,
            visible: Vec::new(),
            picks: Vec::new(),
            up_next: Vec::new(),
            mode: 0,
            target: 0,
            goal: 0,
            plan: None,
            solved: Default::default(),
            cursor: 0,
            filtering: false,
            filter: String::new(),
            status,
            ran_id: String::new(),
            results: Vec::new(),
            cfg_editing: false,
            cfg_input: String::new(),
            stmt_open: false,
            stmt_key: String::new(),
            stmt_blocks: Vec::new(),
            stmt_samples: Vec::new(),
            stmt_scroll: 0,
            detail_scroll: 0,
            width: 0,
            height: 0,
            theme,
            quit: false,
            job: None,
            spin: 0,
        };
        m.apply_filter();
        m.refresh_derived();
        m
    }

    /// Recomputes what depends on the synced data: solve marks and Up next.
    fn refresh_derived(&mut self) {
        let input = self.practice_input();
        self.solved = input.attempted.keys().map(|k| (k.clone(), input.accepted.contains(k))).collect();
        self.solved.extend(input.accepted.iter().map(|k| (k.clone(), true)));
        self.up_next = practice::recommend(&input, practice::Mode::Auto, 5);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.cfg_editing {
            self.update_config_edit(key);
        } else if self.filtering {
            self.update_filter(key);
        } else {
            self.update_normal(key);
        }
    }

    /// Starts f on a background thread, unless a job is already running.
    fn spawn(&mut self, status: String, f: impl FnOnce(&Sender<Msg>) + Send + 'static) {
        if self.job.is_some() {
            self.status = "Busy: wait for the current job to finish".to_string();
            return;
        }
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || f(&tx));
        self.job = Some(rx);
        self.status = status;
    }

    /// Applies whatever the running job has sent so far.
    fn poll_job(&mut self) {
        let Some(rx) = &self.job else { return };
        self.spin = self.spin.wrapping_add(1);
        let mut done = false;
        let mut msgs = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(m) => msgs.push(m),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    done = true;
                    break;
                }
            }
        }
        for m in msgs {
            self.apply(m);
        }
        if done {
            self.job = None;
        }
    }

    fn apply(&mut self, m: Msg) {
        match m {
            Msg::Status(s) => self.status = s,
            Msg::Opened(Ok((path, samples))) => {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                self.status = format!("Opened {name} · {samples} sample(s)");
            }
            Msg::Opened(Err(e)) => self.status = format!("Open: {e}"),
            Msg::Ran(id, Ok(results)) => {
                let passed = results.iter().filter(|r| r.passed).count();
                self.status = format!("Tests: {passed}/{} passed", results.len());
                self.ran_id = id;
                self.results = results;
            }
            Msg::Ran(_, Err(e)) => {
                self.results.clear();
                self.status = format!("Test: {e}");
            }
            Msg::Statement(key, blocks, samples, err) => {
                if key != self.stmt_key {
                    return; // the user moved on while it loaded
                }
                if let Some(e) = err {
                    self.status = format!("Statement: {e}");
                } else {
                    self.stmt_blocks = blocks;
                    self.stmt_samples = samples;
                    self.stmt_scroll = 0;
                    self.stmt_open = true;
                    self.status = "Statement open · j/k scroll · v closes".to_string();
                }
            }
            Msg::Synced(Ok(deps)) => {
                let n = deps.problems.len();
                self.deps = deps;
                self.apply_filter();
                self.refresh_derived();
                if self.is_pick_tab() {
                    self.recompute_picks();
                }
                self.status = format!("Refreshed · {n} problems");
            }
            Msg::Synced(Err(e)) => self.status = format!("Refresh failed: {e}"),
        }
    }

    fn update_filter(&mut self, key: KeyEvent) {
        match key_token(key).as_str() {
            "esc" => {
                self.filtering = false;
                self.filter.clear();
                self.apply_filter();
            }
            "enter" => self.filtering = false,
            "ctrl+c" => self.quit = true,
            "backspace" => {
                self.filter.pop();
                self.apply_filter();
            }
            _ => {
                if let KeyCode::Char(c) = key.code {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.filter.push(c);
                        self.apply_filter();
                    }
                }
            }
        }
    }

    fn update_config_edit(&mut self, key: KeyEvent) {
        match key_token(key).as_str() {
            "esc" => self.cfg_editing = false,
            "enter" => {
                self.cfg_editing = false;
                let value = self.cfg_input.trim().to_string();
                let label = config_view::FIELDS[self.cursor].label;
                config_view::set(&mut self.deps.config, self.cursor, value);
                self.after_config_change(label);
            }
            "ctrl+c" => self.quit = true,
            "backspace" => {
                self.cfg_input.pop();
            }
            _ => {
                if let KeyCode::Char(c) = key.code {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                        self.cfg_input.push(c);
                    }
                }
            }
        }
    }

    fn update_statement(&mut self, key: KeyEvent) {
        match key_token(key).as_str() {
            "q" | "ctrl+c" => self.quit = true,
            "v" | "esc" => self.stmt_open = false,
            "m" => self.cycle_mode(),
            "j" | "down" => self.stmt_scroll += 1,
            "k" | "up" => self.stmt_scroll = text::clamp(self.stmt_scroll - 1, 0, 1 << 20),
            "d" | "pgdown" => self.stmt_scroll += 10,
            "u" | "pgup" => self.stmt_scroll = text::clamp(self.stmt_scroll - 10, 0, 1 << 20),
            "g" => self.stmt_scroll = 0,
            "G" => self.stmt_scroll = 1 << 20,
            "b" => {
                if let Some(p) = self.statement_problem() {
                    if let Err(e) = system::open_url(&p.url) {
                        self.status = format!("Browser: {e}");
                    }
                }
            }
            _ => {}
        }
    }

    fn cycle_mode(&mut self) {
        self.stmt_open = false;
        if self.tab != Tab::Practice {
            self.tab = Tab::Practice;
            self.cursor = 0;
        }
        self.mode = (self.mode + 1) % practice::MODES.len();
        self.recompute_picks();
    }

    fn update_normal(&mut self, key: KeyEvent) {
        if self.stmt_open {
            self.update_statement(key);
            return;
        }
        let token = key_token(key);
        match token.as_str() {
            "d" => {
                self.detail_scroll += 5;
                return;
            }
            "u" => {
                self.detail_scroll = (self.detail_scroll - 5).max(0);
                return;
            }
            _ => {}
        }
        self.detail_scroll = 0;

        if self.tab == Tab::Config && token == "enter" {
            self.activate_config();
            return;
        }
        if self.tab == Tab::Contests && matches!(token.as_str(), "enter" | "o" | "b") {
            if let Some(k) = self.selected_contest() {
                match system::open_url(&k.url) {
                    Ok(()) => self.status = format!("Opened {}", k.name),
                    Err(e) => self.status = format!("Browser: {e}"),
                }
            }
            return;
        }

        match token.as_str() {
            "q" | "ctrl+c" => self.quit = true,
            "tab" => self.switch_tab(),
            "T" => {
                self.deps.config.theme = theme::next(&self.deps.config.theme).to_string();
                let name = self.deps.config.theme.clone();
                self.after_config_change(&format!("Theme {name}"));
            }
            "m" => self.cycle_mode(),
            "j" | "down" => {
                if self.cursor + 1 < self.row_count() {
                    self.cursor += 1;
                }
            }
            "k" | "up" => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            "g" => self.cursor = 0,
            "G" => {
                let n = self.row_count();
                if n > 0 {
                    self.cursor = n - 1;
                }
            }
            "/" => {
                if self.tab == Tab::Problems {
                    self.filtering = true;
                }
            }
            "p" if self.tab == Tab::Problems => {
                self.filter = query::cycle_platform(&self.filter);
                self.apply_filter();
            }
            "[" | "]" if self.tab == Tab::Practice => {
                let rating = self.deps.rating_changes.last().map(|r| r.new_rating).unwrap_or(0);
                let current = if self.target > 0 { self.target } else { practice::default_target(rating) };
                self.target = (current + if token == "]" { 100 } else { -100 }).clamp(800, 3500);
                self.recompute_picks();
            }
            "[" | "]" if self.tab == Tab::Goal => {
                let current = self.plan.as_ref().map(|p| p.target).unwrap_or(0);
                self.goal = crate::target::cycle_milestone(current, if token == "]" { 1 } else { -1 });
                self.recompute_picks();
            }
            "c" if self.tab == Tab::Problems => {
                self.filter.clear();
                self.apply_filter();
            }
            "o" | "enter" => self.do_open(),
            "t" => self.do_run(),
            "v" => self.do_statement(),
            "s" => self.do_submit(),
            "r" => self.do_refresh(),
            "b" => {
                if let Some(p) = self.selected() {
                    if let Err(e) = system::open_url(&p.url) {
                        self.status = format!("Browser: {e}");
                    }
                }
            }
            _ => {}
        }
    }

    fn do_open(&mut self) {
        let Some(p) = self.selected() else { return };
        let (cfg, root, dir) = (self.deps.config.clone(), self.deps.root.clone(), self.deps.config_dir.clone());
        self.spawn(format!("Opening {}…", p.id), move |tx| {
            let _ = tx.send(Msg::Opened(open_solution(&cfg, &root, &dir, &p)));
        });
    }

    fn do_run(&mut self) {
        let Some(p) = self.selected() else { return };
        let (cfg, root, dir) = (self.deps.config.clone(), self.deps.root.clone(), self.deps.config_dir.clone());
        self.spawn(format!("Running {}…", p.id), move |tx| {
            let _ = tx.send(Msg::Ran(p.id.clone(), run_solution(&cfg, &root, &dir, &p)));
        });
    }

    fn do_submit(&mut self) {
        let Some(p) = self.selected() else { return };
        match submit_solution(&self.deps.config, &self.deps.root, &p) {
            Ok(url) => self.status = format!("Code copied. Paste on the judge page: {url}"),
            Err(e) => self.status = format!("Submit: {e}"),
        }
    }

    fn do_statement(&mut self) {
        let Some(p) = self.selected() else { return };
        let key = format!("{}{}", p.platform, p.id);
        if self.stmt_open && self.stmt_key == key {
            self.stmt_open = false;
            return;
        }
        self.stmt_key = key.clone();
        let (cfg, root, dir) = (self.deps.config.clone(), self.deps.root.clone(), self.deps.config_dir.clone());
        self.spawn(format!("Loading statement for {}…", p.id), move |tx| {
            let (blocks, samples, err) = statement_view::load(&dir, &root, &cfg, &p);
            let _ = tx.send(Msg::Statement(key, blocks, samples, err));
        });
    }

    /// Re-syncs from the judges into the cache, then reloads everything from it.
    fn do_refresh(&mut self) {
        let (cfg, dir, path) = (self.deps.config.clone(), self.deps.config_dir.clone(), self.deps.config_path.clone());
        self.spawn("Refreshing from the judges…".to_string(), move |tx| {
            let result = (|| {
                let mut cache = Cache::open(&dir.join("cache.db"))?;
                crate::sync::run(&mut cache, &cfg, &mut |line| {
                    let _ = tx.send(Msg::Status(format!("Refreshing: {line}")));
                })?;
                load_deps(&cache, cfg, dir, path)
            })();
            let _ = tx.send(Msg::Synced(result));
        });
    }

    fn switch_tab(&mut self) {
        self.stmt_open = false;
        self.tab = self.tab.next();
        if self.is_pick_tab() {
            self.recompute_picks();
        }
        self.cursor = 0;
    }

    fn is_stats_tab(&self) -> bool {
        matches!(self.tab, Tab::Contests | Tab::Analytics | Tab::Dashboard | Tab::Config)
    }

    fn row_count(&self) -> usize {
        match self.tab {
            Tab::Practice | Tab::Goal => self.picks.len(),
            Tab::Problems => self.visible.len(),
            Tab::Contests => contests_view::upcoming(&self.deps.contests, Utc::now().timestamp()).len(),
            Tab::Config => config_view::FIELDS.len(),
            _ => 0,
        }
    }

    /// Tabs whose rows are self.picks.
    fn is_pick_tab(&self) -> bool {
        matches!(self.tab, Tab::Practice | Tab::Goal)
    }

    fn recompute_picks(&mut self) {
        let input = self.practice_input();
        if self.tab == Tab::Goal {
            let goal = if self.goal > 0 { self.goal } else { crate::target::next_milestone(input.rating) };
            let plan = crate::target::analyze(&input, goal);
            self.picks = plan
                .steps
                .iter()
                .map(|s| practice::Pick {
                    problem: s.problem.clone(),
                    score: 0.0,
                    reasons: vec![format!("Rung {}/{} · trains {}", s.rung, s.rungs, s.tag)],
                })
                .collect();
            self.plan = Some(plan);
            self.cursor = 0;
            self.status = format!("Goal plan: {} problems · [ ] change goal", self.picks.len());
            return;
        }
        let mode = practice::MODES[self.mode];
        self.picks = practice::recommend(&input, mode, 30);
        self.cursor = 0;
        if self.picks.is_empty() {
            self.status = format!("{}: no recommendations (sync more, or try another mode with m)", mode.label());
        } else {
            self.status = format!("{}: {} picks · m next mode", mode.label(), self.picks.len());
        }
    }

    fn practice_input(&self) -> practice::Input {
        let mut input = practice::Input { problems: self.deps.problems.clone(), now: Utc::now().timestamp(), ..Default::default() };
        for s in &self.deps.submissions {
            let key = format!("{}{}", s.platform, s.problem_id);
            let e = input.attempted.entry(key.clone()).or_insert(0);
            if s.submitted_at > *e {
                *e = s.submitted_at;
            }
            if s.verdict == "OK" {
                input.accepted.insert(key);
            }
        }
        for id in &self.deps.cses_solved {
            input.accepted.insert(format!("cses{id}"));
        }
        if let Some(r) = self.deps.rating_changes.last() {
            input.rating = r.new_rating;
        }
        input.target = self.target;
        input
    }

    fn apply_filter(&mut self) {
        let keep = if self.tab == Tab::Problems { self.selected().map(|p| format!("{}{}", p.platform, p.id)) } else { None };
        let q = query::parse(&self.filter);
        self.visible.clear();
        for (i, p) in self.deps.problems.iter().enumerate() {
            if q.matches(p) {
                self.visible.push(i);
            }
        }
        self.cursor = 0;
        if let Some(keep) = keep {
            for (i, &idx) in self.visible.iter().enumerate() {
                let p = &self.deps.problems[idx];
                if format!("{}{}", p.platform, p.id) == keep {
                    self.cursor = i;
                    break;
                }
            }
        }
    }

    fn selected(&self) -> Option<Problem> {
        if self.tab != Tab::Problems && !self.is_pick_tab() {
            return None;
        }
        if self.cursor >= self.row_count() {
            return None;
        }
        if self.is_pick_tab() {
            return Some(self.picks[self.cursor].problem.clone());
        }
        Some(self.deps.problems[self.visible[self.cursor]].clone())
    }

    fn row_problem(&self, i: usize) -> Problem {
        if self.is_pick_tab() {
            self.picks[i].problem.clone()
        } else {
            self.deps.problems[self.visible[i]].clone()
        }
    }

    fn selected_contest(&self) -> Option<Contest> {
        contests_view::upcoming(&self.deps.contests, Utc::now().timestamp()).get(self.cursor).cloned()
    }

    fn right_scroll(&self) -> i64 {
        if self.stmt_open {
            self.stmt_scroll
        } else {
            self.detail_scroll
        }
    }

    fn statement_problem(&self) -> Option<Problem> {
        self.deps.problems.iter().find(|p| format!("{}{}", p.platform, p.id) == self.stmt_key).cloned()
    }

    fn activate_config(&mut self) {
        if self.cursor >= config_view::FIELDS.len() {
            return;
        }
        let f = &config_view::FIELDS[self.cursor];
        if !f.choice {
            self.cfg_editing = true;
            self.cfg_input = config_view::get(&self.deps.config, self.cursor);
            return;
        }
        let opts = config_view::options(&self.deps.config, self.cursor);
        if opts.is_empty() {
            return;
        }
        let current = config_view::get(&self.deps.config, self.cursor);
        let mut next = opts[0].clone();
        for (i, o) in opts.iter().enumerate() {
            if *o == current {
                next = opts[(i + 1) % opts.len()].clone();
                break;
            }
        }
        let label = config_view::FIELDS[self.cursor].label;
        config_view::set(&mut self.deps.config, self.cursor, next);
        self.after_config_change(label);
    }

    fn after_config_change(&mut self, label: &str) {
        if self.deps.config.theme.is_empty() {
            self.deps.config.theme = "catppuccin".to_string();
        }
        self.theme = theme::apply(&self.deps.config.theme);
        if let Ok(root) = workspace::root(&self.deps.config) {
            self.deps.root = root;
        }
        if self.deps.config_path.as_os_str().is_empty() {
            self.status = format!("{label} changed (not saved: no config path)");
            return;
        }
        match config::save(&self.deps.config_path, &self.deps.config) {
            Ok(()) => self.status = format!("{label} saved"),
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }
}

/// The configured language's compile settings, or an error naming the problem.
fn language(cfg: &Config) -> Result<&crate::config::CompileCommand> {
    cfg.compile_commands
        .get(&cfg.default_language)
        .ok_or_else(|| anyhow::anyhow!("no compile settings for {:?} in config", cfg.default_language))
}

/// Creates the solution from a template if missing, saves samples, and opens
/// the file in the editor. An existing solution is never overwritten.
fn open_solution(cfg: &Config, root: &Path, config_dir: &Path, p: &Problem) -> Result<(PathBuf, usize)> {
    let cc = language(cfg)?;
    let path = workspace::solution_path(root, p, &cc.extension);
    let own = workspace::user_template(config_dir, &cfg.default_language, &cc.extension);
    workspace::scaffold(&path, own.as_deref().unwrap_or(workspace::template(&cfg.default_language)))?;

    let mut samples = workspace::load_samples(&path)?;
    if samples.is_empty() {
        samples = dispatch::fetch_samples(p).map_err(|e| anyhow::anyhow!("fetch samples: {e}"))?;
        workspace::save_samples(&path, &samples)?;
    }
    system::open_in_editor(&cfg.editor, &path.to_string_lossy()).map_err(|e| anyhow::anyhow!("editor: {e}"))?;
    Ok((path, samples.len()))
}

/// Compiles the solution and runs it against the saved samples.
fn run_solution(cfg: &Config, root: &Path, config_dir: &Path, p: &Problem) -> Result<Vec<CaseResult>> {
    let cc = language(cfg)?;
    let path = workspace::solution_path(root, p, &cc.extension);
    let samples = workspace::load_samples(&path)?;
    if samples.is_empty() {
        return Err(anyhow::anyhow!("no samples yet: press o first"));
    }
    let build_dir = config_dir.join("build");
    std::fs::create_dir_all(&build_dir)?;
    let spec = runner::Spec { compile: &cc.compile, run: &cc.run, source: &path, dir: &build_dir };
    runner::run_samples(&spec, &samples, StdDuration::from_secs(5))
}

/// Copies the solution to the clipboard and opens the judge's submit page.
/// CPX does not post to the judge itself.
fn submit_solution(cfg: &Config, root: &Path, p: &Problem) -> Result<String> {
    let cc = language(cfg)?;
    let url = dispatch::submit_url(p).ok_or_else(|| anyhow::anyhow!("no submit page known for this problem"))?;
    let path = workspace::solution_path(root, p, &cc.extension);
    let code = std::fs::read_to_string(&path).map_err(|_| anyhow::anyhow!("no solution file yet: press o first"))?;
    system::copy_clipboard(&code).map_err(|e| anyhow::anyhow!("clipboard: {e}"))?;
    system::open_url(&url).map_err(|e| anyhow::anyhow!("browser: {e}"))?;
    Ok(url)
}

/// Builds Deps from the cached data: the TUI's entry point from main.
pub fn load_deps(cache: &Cache, cfg: Config, config_dir: PathBuf, config_path: PathBuf) -> Result<Deps> {
    let mut problems = Vec::new();
    for platform in ["codeforces", "cses", "atcoder"] {
        problems.extend(cache.list_problems(platform)?);
    }
    let root = workspace::root(&cfg)?;
    let contests = cache.list_contests("codeforces")?;
    let cses_solved: Vec<String> = cache
        .get_meta("cses_progress")?
        .and_then(|raw| serde_json::from_str::<crate::judge::cses::Progress>(&raw).ok())
        .map(|p| p.solved)
        .unwrap_or_default();
    let submissions = cache.list_submissions("codeforces")?;
    let rating_changes = cache.list_rating_changes("codeforces")?;
    Ok(Deps { problems, submissions, rating_changes, contests, cses_solved, config: cfg, config_dir, config_path, root })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn screen(m: &mut Model, w: u16, h: u16) -> String {
        m.width = w;
        m.height = h;
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| view::draw(f, m)).unwrap();
        let buf = term.backend().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn sample_deps() -> Deps {
        let problems = (1..=40)
            .map(|i| Problem {
                platform: "codeforces".into(),
                id: format!("{}A", 1900 + i),
                name: format!("Problem number {i}"),
                url: String::new(),
                rating: 800 + (i % 10) * 100,
                tags: vec!["greedy".into(), "math".into()],
                category: String::new(),
            })
            .collect();
        Deps {
            problems,
            submissions: Vec::new(),
            rating_changes: Vec::new(),
            contests: Vec::new(),
            cses_solved: Vec::new(),
            config: Config::default(),
            config_dir: PathBuf::new(),
            config_path: PathBuf::new(),
            root: PathBuf::new(),
        }
    }

    // Every tab renders at a normal and a cramped size without panicking,
    // and the Problems list shows its rows.
    #[test]
    fn every_tab_renders() {
        let mut m = Model::new(sample_deps());
        for (w, h) in [(120, 35), (40, 10), (10, 4)] {
            for tab in TABS {
                m.tab = tab;
                m.cursor = 0;
                screen(&mut m, w, h);
            }
        }
        m.tab = Tab::Problems;
        let s = screen(&mut m, 120, 35);
        assert!(s.contains("Problem number 1"), "{s}");
        assert!(s.contains("Problems"), "{s}");
    }

    // A job's messages reach the model, and the job clears when its thread ends.
    #[test]
    fn background_job_reports_back() {
        let mut m = Model::new(sample_deps());
        m.spawn("working".into(), |tx| {
            tx.send(Msg::Status("halfway".into())).unwrap();
            tx.send(Msg::Ran("1901A".into(), Ok(Vec::new()))).unwrap();
        });
        m.spawn("second".into(), |_| {});
        assert!(m.status.starts_with("Busy"), "one job at a time: {}", m.status);
        let started = std::time::Instant::now();
        while m.job.is_some() && started.elapsed() < StdDuration::from_secs(5) {
            m.poll_job();
            thread::sleep(StdDuration::from_millis(5));
        }
        assert!(m.job.is_none());
        assert_eq!(m.ran_id, "1901A");
        assert_eq!(m.status, "Tests: 0/0 passed");
    }

    // Eyeball check against the real cache: cargo test dump_real -- --ignored --nocapture
    #[test]
    #[ignore]
    fn dump_real() {
        let path = crate::config::path().unwrap();
        let cfg = crate::config::load(&path).unwrap();
        let dir = path.parent().unwrap().to_path_buf();
        let cache = Cache::open(&dir.join("cache.db")).unwrap();
        let mut m = Model::new(load_deps(&cache, cfg, dir, PathBuf::new()).unwrap());
        for tab in TABS {
            m.tab = tab;
            m.cursor = 0;
            m.recompute_picks();
            println!("===== {tab:?} =====\n{}", screen(&mut m, 110, 30));
        }
    }
}
