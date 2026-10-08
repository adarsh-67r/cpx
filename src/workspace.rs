// Where solution files live, what they start with, and the sample files kept
// beside them. Names match the Go version, so both builds share one folder.

use crate::config::Config;
use crate::problem::{Problem, Sample};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub const CPP_TEMPLATE: &str = "#include <bits/stdc++.h>
using namespace std;

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);

    return 0;
}
";

pub const PYTHON_TEMPLATE: &str = "import sys


def main():
    data = sys.stdin.read().split()


if __name__ == \"__main__\":
    main()
";

/// The starting text for a language key ("cpp", "python"). Unknown keys get an empty file.
pub fn template(lang: &str) -> &'static str {
    match lang {
        "cpp" => CPP_TEMPLATE,
        "python" => PYTHON_TEMPLATE,
        _ => "",
    }
}

/// The workspace root: the configured folder, or ~/cpx when none is set.
pub fn root(cfg: &Config) -> Result<PathBuf> {
    if !cfg.workspace_dir.is_empty() {
        return Ok(PathBuf::from(&cfg.workspace_dir));
    }
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home directory on this system"))?;
    Ok(home.join("cpx"))
}

/// Title to PascalCase: "Weird Algorithm" becomes "WeirdAlgorithm".
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in name.chars() {
        if !ch.is_alphanumeric() {
            upper = true;
            continue;
        }
        if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Codeforces uses the id (4A). CSES uses the PascalCase title. Other judges use the lowercased id.
pub fn file_base(p: &Problem) -> String {
    match p.platform.as_str() {
        "codeforces" => p.id.clone(),
        "cses" => slug(&p.name),
        _ => p.id.to_lowercase(),
    }
}

/// <root>/<platform>/<base><ext>
pub fn solution_path(root: &Path, p: &Problem, ext: &str) -> PathBuf {
    root.join(&p.platform).join(format!("{}{}", file_base(p), ext))
}

/// The sample file kept beside a solution: 4A.cpp -> 4A.tests.json.
pub fn samples_path(solution: &Path) -> PathBuf {
    solution.with_extension("tests.json")
}

/// Writes the samples beside the solution, creating the folder if needed.
pub fn save_samples(solution: &Path, samples: &[Sample]) -> Result<()> {
    if let Some(dir) = solution.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(samples)?;
    text.push('\n');
    fs::write(samples_path(solution), text)?;
    Ok(())
}

/// The saved samples. A missing file gives an empty list.
pub fn load_samples(solution: &Path) -> Result<Vec<Sample>> {
    match fs::read_to_string(samples_path(solution)) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}

/// Writes the starting file unless one already exists. Returns true if written,
/// so a solution in progress is never overwritten.
pub fn scaffold(path: &Path, text: &str) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(platform: &str, id: &str, name: &str) -> Problem {
        Problem { platform: platform.into(), id: id.into(), name: name.into(), ..Default::default() }
    }

    #[test]
    fn file_names_per_platform() {
        assert_eq!(file_base(&p("codeforces", "4A", "Watermelon")), "4A");
        assert_eq!(file_base(&p("cses", "1068", "Weird Algorithm")), "WeirdAlgorithm");
        assert_eq!(file_base(&p("atcoder", "ABC001_1", "x")), "abc001_1");
        assert_eq!(slug("Two Sets (easy)"), "TwoSetsEasy");
    }

    #[test]
    fn samples_round_trip_beside_solution() {
        let dir = std::env::temp_dir().join(format!("cpx-ws-{}", std::process::id()));
        let sol = dir.join("codeforces").join("4A.cpp");
        assert!(load_samples(&sol).unwrap().is_empty());
        save_samples(&sol, &[Sample { input: "8".into(), output: "YES".into() }]).unwrap();
        assert_eq!(samples_path(&sol).file_name().unwrap(), "4A.tests.json");
        let got = load_samples(&sol).unwrap();
        assert_eq!(got, vec![Sample { input: "8".into(), output: "YES".into() }]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scaffold_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("cpx-scaffold-{}", std::process::id()));
        let path = dir.join("4A.cpp");
        assert!(scaffold(&path, "first\n").unwrap());
        assert!(!scaffold(&path, "second\n").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "first\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn templates_by_language() {
        assert!(template("cpp").contains("int main()"));
        assert!(template("python").contains("def main()"));
        assert_eq!(template("brainfuck"), "");
    }
}
