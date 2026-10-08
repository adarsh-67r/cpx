// Command cpx is the terminal app.
//
//   cpx        open the problem list
//   cpx sync   fetch judge problems (and submissions/ratings) into the cache
//   cpx setup  run the setup wizard

use anyhow::Result;
use cpx::cache::Cache;
use cpx::config;
use cpx::{setup, sync, tui};
use std::io::IsTerminal;

fn main() {
    if let Err(e) = run() {
        eprintln!("cpx: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cfg_path = config::path()?;
    let mut cfg = config::load(&cfg_path)?;
    let args: Vec<String> = std::env::args().collect();
    let subcommand = args.get(1).map(|s| s.as_str());

    // First run, or an explicit `cpx setup`: ask the setup questions on the
    // terminal and save the answers before opening the app.
    let first_run = !cfg_path.exists() && std::io::stdin().is_terminal();
    if subcommand == Some("setup") || first_run {
        let mut input = std::io::BufReader::new(std::io::stdin());
        let mut output = std::io::stdout();
        cfg = setup::run(&mut input, &mut output, cfg)?;
        config::save(&cfg_path, &cfg)?;
        println!("cpx: saved {}", cfg_path.display());
        if subcommand == Some("setup") {
            return Ok(());
        }
    }

    let db_path = cfg_path.parent().unwrap().join("cache.db");
    let mut cache = Cache::open(&db_path)?;

    if subcommand == Some("sync") {
        return sync::run(&mut cache, &cfg, &mut |line| println!("cpx: {line}"));
    }

    let config_dir = cfg_path.parent().unwrap().to_path_buf();
    let deps = tui::load_deps(&cache, cfg, config_dir, cfg_path)?;
    tui::run(deps)
}
