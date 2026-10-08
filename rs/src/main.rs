// Command cpx is the terminal app.
//
//   cpx        open the problem list
//   cpx sync   fetch judge problems (and submissions/ratings) into the cache
//   cpx setup  run the setup wizard

use anyhow::Result;
use cpx::cache::Cache;
use cpx::config::{self, Config};
use cpx::judge::{atcoder, codeforces, cses};
use cpx::{setup, tui};
use std::io::IsTerminal;
use std::time::{SystemTime, UNIX_EPOCH};

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
        return sync(&mut cache, &cfg);
    }

    let config_dir = cfg_path.parent().unwrap().to_path_buf();
    let deps = tui::load_deps(&cache, cfg, config_dir, cfg_path)?;
    tui::run(deps)
}

/// Refreshes the Codeforces problemset and upcoming contests, CSES and
/// AtCoder catalogs, and (when a Codeforces handle is set) submissions and
/// ratings. CSES and AtCoder failures are reported but do not stop the sync.
fn sync(cache: &mut Cache, cfg: &Config) -> Result<()> {
    println!("cpx: fetching Codeforces problemset...");
    let probs = codeforces::fetch_problems()?;
    cache.upsert_problems(&probs)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    cache.set_meta("last_sync_codeforces", &now.to_string())?;
    println!("cpx: synced {} Codeforces problems", probs.len());

    let contests = codeforces::fetch_contests()?;
    cache.upsert_contests(&contests)?;
    println!("cpx: synced {} upcoming or running Codeforces contests", contests.len());

    match cses::fetch_list(&cfg.cses_session) {
        Err(e) => println!("cpx: CSES sync failed: {e}"),
        Ok((probs, progress)) => {
            cache.upsert_problems(&probs)?;
            cache.set_meta("cses_progress", &serde_json::to_string(&progress)?)?;
            println!(
                "cpx: synced {} CSES tasks ({} solved, {} attempted)",
                probs.len(),
                progress.solved.len(),
                progress.attempted.len()
            );
        }
    }

    match atcoder::fetch_catalog() {
        Err(e) => println!("cpx: AtCoder sync failed: {e}"),
        Ok(probs) => {
            cache.upsert_problems(&probs)?;
            println!("cpx: synced {} AtCoder tasks", probs.len());
        }
    }

    let handle = cfg.handles.get("codeforces").cloned().unwrap_or_default();
    if handle.is_empty() {
        println!("cpx: no Codeforces handle set in config; skipping submissions and ratings");
        return Ok(());
    }
    let subs = codeforces::fetch_submissions(&handle)?;
    cache.upsert_submissions(&subs)?;
    let ratings = codeforces::fetch_ratings(&handle)?;
    cache.upsert_rating_changes(&ratings)?;
    println!("cpx: {handle} — {} submissions, {} rated contests", subs.len(), ratings.len());
    Ok(())
}
