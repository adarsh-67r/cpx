// Command cpx is the terminal app.
//
//   cpx        open the problem list
//   cpx sync   fetch judge problems (and submissions/ratings) into the cache
//   cpx setup  run the setup wizard
//   cpx update  install the newest release build
//   cpx stress <solution> <brute> <generator> [runs]
//              compare a solution with a brute force on generated inputs

use anyhow::Result;
use cpx::cache::Cache;
use cpx::config;
use cpx::{setup, stress, sync, tui, update};
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
    if subcommand == Some("update") {
        return update::run(&mut |line| println!("cpx: {line}"));
    }

    // First run, or an explicit `cpx setup`: ask the setup questions on the
    // terminal and save the answers before opening the app.
    let first_run = !cfg_path.exists() && std::io::stdin().is_terminal();
    if subcommand == Some("setup") || first_run {
        let mut input = std::io::BufReader::new(std::io::stdin());
        let mut output = std::io::stdout();
        cfg = setup::run(&mut input, &mut output, cfg, cfg_path.parent().unwrap())?;
        config::save(&cfg_path, &cfg)?;
        println!("cpx: saved {}", cfg_path.display());
        if subcommand == Some("setup") {
            return Ok(());
        }
    }

    if subcommand == Some("stress") {
        return run_stress(&cfg, &args[2..], &cfg_path.parent().unwrap().join("build").join("stress"));
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

fn run_stress(cfg: &config::Config, args: &[String], build_dir: &std::path::Path) -> Result<()> {
    if args.len() < 3 {
        anyhow::bail!("usage: cpx stress <solution> <brute> <generator> [runs]
The generator gets the seed as its argument and prints one input.");
    }
    let runs: u64 = match args.get(3) {
        Some(n) => n.parse().map_err(|_| anyhow::anyhow!("runs must be a number, got {n:?}"))?,
        None => 100,
    };
    let p = |i: usize| std::path::PathBuf::from(&args[i]);
    let limit = std::time::Duration::from_secs(5);
    let found = stress::run(cfg, &p(0), &p(1), &p(2), build_dir, runs, limit, &mut |seed| {
        print!("\rcpx: run {seed}/{runs}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })?;
    println!();
    match found {
        None => println!("cpx: all {runs} runs matched"),
        Some((seed, r)) => {
            println!("cpx: seed {seed} fails{}", r.error.as_ref().map(|e| format!(" ({e})")).unwrap_or_default());
            println!("--- input
{}--- expected (brute)
{}--- got
{}", r.input, r.expected, r.actual);
            std::process::exit(1);
        }
    }
    Ok(())
}
