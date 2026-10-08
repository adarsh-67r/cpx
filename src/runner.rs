// Compiles a solution and runs it against samples. Only the child process is
// timed. A timeout kills the whole process tree, so a program that starts
// children cannot keep the test running.

use crate::problem::Sample;
use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct Spec<'a> {
    pub compile: &'a str,
    pub run: &'a str,
    pub source: &'a Path,
    pub dir: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseResult {
    pub index: usize,
    pub passed: bool,
    pub input: String,
    pub expected: String,
    pub actual: String,
    pub duration: Duration,
    pub error: Option<String>,
}

/// Trailing spaces on each line and trailing blank lines do not count.
pub fn normalize(s: &str) -> String {
    let joined: Vec<&str> = s.split('\n').map(|l| l.trim_end_matches([' ', '\t', '\r'])).collect();
    joined.join("\n").trim_end_matches('\n').to_string()
}

/// Builds (if the language needs it), then runs every sample. A compile
/// failure is an error; a bad sample is reported in its result.
pub fn run_samples(spec: &Spec<'_>, samples: &[Sample], time_limit: Duration) -> Result<Vec<CaseResult>> {
    let output = spec.source.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if !spec.compile.trim().is_empty() {
        let line = expand(spec.compile, spec.source, &output, spec.dir);
        let out = shell(&line, spec.dir).output()?;
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let msg = if msg.is_empty() { String::from_utf8_lossy(&out.stdout).trim().to_string() } else { msg };
            return Err(anyhow!("compilation failed:\n{msg}"));
        }
    }

    let line = expand(spec.run, spec.source, &output, spec.dir);
    Ok(samples
        .iter()
        .enumerate()
        .map(|(i, s)| run_one(&line, spec.dir, i + 1, s, time_limit))
        .collect())
}

fn run_one(line: &str, dir: &Path, index: usize, s: &Sample, limit: Duration) -> CaseResult {
    let mut res = CaseResult {
        index,
        passed: false,
        input: s.input.clone(),
        expected: s.output.clone(),
        actual: String::new(),
        duration: Duration::ZERO,
        error: None,
    };

    let mut child = match shell(line, dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            res.error = Some(format!("failed to start: {e}"));
            return res;
        }
    };

    // Feed the input from its own thread, so a program that does not read it
    // cannot block the timer.
    if let Some(mut stdin) = child.stdin.take() {
        let input = s.input.clone();
        thread::spawn(move || {
            let _ = stdin.write_all(input.as_bytes());
        });
    }
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_reader = thread::spawn(move || {
        let mut buf = String::new();
        if let Some(s) = stdout.as_mut() {
            let _ = s.read_to_string(&mut buf);
        }
        buf
    });
    let err_reader = thread::spawn(move || {
        let mut buf = String::new();
        if let Some(s) = stderr.as_mut() {
            let _ = s.read_to_string(&mut buf);
        }
        buf
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) => {
                if started.elapsed() >= limit {
                    kill_tree(&mut child);
                    let _ = child.wait();
                    break None;
                }
                thread::sleep(Duration::from_millis(2));
            }
            Err(_) => break None,
        }
    };
    res.duration = started.elapsed();
    res.actual = out_reader.join().unwrap_or_default();
    let stderr_text = err_reader.join().unwrap_or_default();

    match status {
        None if res.duration >= limit => res.error = Some("time limit exceeded".into()),
        None => res.error = Some("process error".into()),
        Some(st) if !st.success() => {
            let msg = stderr_text.trim().to_string();
            res.error = Some(if msg.is_empty() { format!("exit code {:?}", st.code()) } else { msg });
        }
        Some(_) => res.passed = normalize(&res.actual) == normalize(&s.output),
    }
    res
}

/// Kills the child and, on Windows, everything it started.
fn kill_tree(child: &mut std::process::Child) {
    if cfg!(windows) {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

fn shell(line: &str, dir: &Path) -> Command {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", line]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", line]);
        c
    };
    cmd.current_dir(dir);
    cmd
}

/// Replaces {source}, {output}, and {dir}, quoting paths that contain spaces.
pub fn expand(template: &str, source: &Path, output: &str, dir: &Path) -> String {
    template
        .replace("{source}", &quote(&source.to_string_lossy()))
        .replace("{output}", &quote(output))
        .replace("{dir}", &quote(&dir.to_string_lossy()))
}

fn quote(s: &str) -> String {
    if !s.contains([' ', '\t', '\'', '"']) {
        return s.to_string();
    }
    if cfg!(windows) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cpx-runner-{}-{}", name, std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn python_available() -> bool {
        Command::new("python").arg("--version").output().is_ok()
    }

    #[test]
    fn normalize_ignores_trailing_space_and_blank_lines() {
        assert_eq!(normalize("3 \n4\n\n"), normalize("3\n4"));
        assert_ne!(normalize("3\n4"), normalize("3\n5"));
    }

    #[test]
    fn expand_quotes_paths_with_spaces() {
        let s = expand("g++ -o {output} {source}", Path::new("/tmp/Comp Prog/a.cpp"), "a", Path::new("/tmp/b"));
        assert!(s.contains("a.cpp") && s.contains("Comp Prog"));
    }

    #[test]
    fn passes_and_fails_with_python() {
        if !python_available() {
            return;
        }
        let dir = scratch("pass");
        let src = dir.join("sum.py");
        std::fs::write(&src, "a, b = map(int, input().split())\nprint(a + b)\n").unwrap();
        let spec = Spec { compile: "", run: "python {source}", source: &src, dir: &dir };
        let samples = vec![
            Sample { input: "1 2".into(), output: "3".into() },
            Sample { input: "2 2".into(), output: "5".into() },
        ];
        let res = run_samples(&spec, &samples, Duration::from_secs(5)).unwrap();
        assert!(res[0].passed);
        assert!(!res[1].passed);
        assert_eq!(res[1].actual.trim(), "4");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn time_limit_is_reported_and_timer_is_not_blocked() {
        if !python_available() {
            return;
        }
        let dir = scratch("slow");
        let src = dir.join("slow.py");
        std::fs::write(&src, "import time\ntime.sleep(5)\n").unwrap();
        let spec = Spec { compile: "", run: "python {source}", source: &src, dir: &dir };
        let started = Instant::now();
        let res = run_samples(&spec, &[Sample::default()], Duration::from_millis(400)).unwrap();
        assert_eq!(res[0].error.as_deref(), Some("time limit exceeded"));
        assert!(started.elapsed() < Duration::from_secs(4), "should stop near the limit");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn compile_failure_is_an_error() {
        if !python_available() {
            return;
        }
        let dir = scratch("compile");
        let src = dir.join("bad.py");
        std::fs::write(&src, "import sys\nsys.exit(3)\n").unwrap();
        let spec = Spec { compile: "python {source}", run: "python {source}", source: &src, dir: &dir };
        assert!(run_samples(&spec, &[Sample::default()], Duration::from_secs(5)).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
