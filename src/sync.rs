// Refreshes the cache from the judges. Shared by `cpx sync` and the app's
// refresh key; progress goes to `log` as one line per step.

use crate::cache::Cache;
use crate::config::Config;
use crate::judge::{atcoder, codeforces, cses};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

/// Refreshes the Codeforces problemset and upcoming contests, CSES and
/// AtCoder catalogs, and (when a Codeforces handle is set) submissions and
/// ratings. CSES and AtCoder failures are reported but do not stop the sync.
pub fn run(cache: &mut Cache, cfg: &Config, log: &mut dyn FnMut(String)) -> Result<()> {
    log("fetching Codeforces problemset...".into());
    let probs = codeforces::fetch_problems()?;
    cache.upsert_problems(&probs)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    cache.set_meta("last_sync_codeforces", &now.to_string())?;
    log(format!("synced {} Codeforces problems", probs.len()));

    let contests = codeforces::fetch_contests()?;
    cache.upsert_contests(&contests)?;
    log(format!("synced {} upcoming or running Codeforces contests", contests.len()));

    match cses::fetch_list(&cfg.cses_session) {
        Err(e) => log(format!("CSES sync failed: {e}")),
        Ok((probs, progress)) => {
            cache.upsert_problems(&probs)?;
            cache.set_meta("cses_progress", &serde_json::to_string(&progress)?)?;
            log(format!(
                "synced {} CSES tasks ({} solved, {} attempted)",
                probs.len(),
                progress.solved.len(),
                progress.attempted.len()
            ));
        }
    }

    match atcoder::fetch_catalog() {
        Err(e) => log(format!("AtCoder sync failed: {e}")),
        Ok(probs) => {
            cache.upsert_problems(&probs)?;
            log(format!("synced {} AtCoder tasks", probs.len()));
        }
    }

    let handle = cfg.handles.get("codeforces").cloned().unwrap_or_default();
    if handle.is_empty() {
        log("no Codeforces handle set in config; skipping submissions and ratings".into());
        return Ok(());
    }
    let subs = codeforces::fetch_submissions(&handle)?;
    cache.upsert_submissions(&subs)?;
    let ratings = codeforces::fetch_ratings(&handle)?;
    cache.upsert_rating_changes(&ratings)?;
    log(format!("{handle}: {} submissions, {} rated contests", subs.len(), ratings.len()));
    Ok(())
}
