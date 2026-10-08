// Stress testing: a generator writes a random input for each seed, a brute
// force answers it, and the solution must agree. The first disagreement,
// crash, or timeout is reported with its input.

use crate::config::Config;
use crate::problem::Sample;
use crate::runner::{self, CaseResult, Spec};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::time::Duration;

/// Builds a file with the language whose extension matches it.
fn build(cfg: &Config, source: &Path, dir: &Path) -> Result<String> {
    let ext = source.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let cc = cfg
        .compile_commands
        .values()
        .find(|c| c.extension == ext)
        .ok_or_else(|| anyhow!("no language in config for {} files", if ext.is_empty() { "extensionless" } else { &ext }))?;
    let source = std::path::absolute(source).map_err(|e| anyhow!("{}: {e}", source.display()))?;
    runner::build(&Spec { compile: &cc.compile, run: &cc.run, source: &source, dir })
        .map_err(|e| anyhow!("{}: {e}", source.display()))
}

/// Runs up to iterations seeds. Returns the seed and result of the first
/// failing case, or None when every case matched. The generator gets the seed
/// as its only argument.
pub fn run(
    cfg: &Config,
    solution: &Path,
    brute: &Path,
    generator: &Path,
    build_dir: &Path,
    iterations: u64,
    limit: Duration,
    progress: &mut dyn FnMut(u64),
) -> Result<Option<(u64, CaseResult)>> {
    std::fs::create_dir_all(build_dir)?;
    let sol = build(cfg, solution, build_dir)?;
    let brute = build(cfg, brute, build_dir)?;
    let gen = build(cfg, generator, build_dir)?;
    let none = Sample::default();
    for seed in 1..=iterations {
        progress(seed);
        let g = runner::run_one(&format!("{gen} {seed}"), build_dir, 0, &none, limit);
        if let Some(e) = g.error {
            return Err(anyhow!("generator failed on seed {seed}: {e}"));
        }
        let b = runner::run_one(&brute, build_dir, 0, &Sample { input: g.actual.clone(), ..Default::default() }, limit);
        if let Some(e) = b.error {
            return Err(anyhow!("brute force failed on seed {seed}: {e}\ninput:\n{}", g.actual));
        }
        let r = runner::run_one(&sol, build_dir, seed as usize, &Sample { input: g.actual, output: b.actual }, limit);
        if !r.passed {
            return Ok(Some((seed, r)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_seed_where_solution_and_brute_disagree() {
        let dir = std::env::temp_dir().join(format!("cpx-stress-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let w = |name: &str, body: &str| {
            let p = dir.join(name);
            std::fs::write(&p, format!("#include <bits/stdc++.h>\nusing namespace std;\nint main(int argc, char** argv) {{ {body} }}\n")).unwrap();
            p
        };
        let gen = w("gen.cpp", "cout << atoi(argv[1]) << endl;");
        let brute = w("brute.cpp", "long long n; cin >> n; cout << n * 2 << endl;");
        let sol = w("sol.cpp", "long long n; cin >> n; cout << (n == 4 ? 7 : n * 2) << endl;");
        let good = w("good.cpp", "long long n; cin >> n; cout << n + n << endl;");
        let cfg = Config::default();
        let limit = Duration::from_secs(5);
        let mut seen = 0;
        let found = run(&cfg, &sol, &brute, &gen, &dir.join("build"), 10, limit, &mut |s| seen = s).unwrap();
        let (seed, r) = found.expect("mismatch");
        assert_eq!((seed, seen), (4, 4));
        assert_eq!(r.input.trim(), "4");
        assert_eq!((r.expected.trim(), r.actual.trim()), ("8", "7"));
        assert!(run(&cfg, &good, &brute, &gen, &dir.join("build"), 5, limit, &mut |_| {}).unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
