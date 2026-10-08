// First-run wizard: a few questions on the terminal, with defaults so
// pressing enter through all of them gives a working config.

use crate::config::Config;
use anyhow::{anyhow, Result};
use std::io::{BufRead, Write};

fn ask<R: BufRead, W: Write>(input: &mut R, output: &mut W, prompt: &str, current: &str) -> Result<String> {
    write!(output, "{prompt} [{current}]: ")?;
    output.flush()?;
    let mut line = String::new();
    input.read_line(&mut line)?;
    let answer = line.trim().to_string();
    Ok(if answer.is_empty() { current.to_string() } else { answer })
}

/// Asks the setup questions, reading answers from input and writing prompts
/// to output. Blank answers keep the current value.
pub fn run<R: BufRead, W: Write>(input: &mut R, output: &mut W, mut cfg: Config) -> Result<Config> {
    writeln!(output, "Welcome to CPX. Press enter to keep the value in brackets.")?;

    let current_handle = cfg.handles.get("codeforces").cloned().unwrap_or_default();
    let handle = ask(input, output, "Codeforces handle (blank to skip)", &current_handle)?;
    if !handle.is_empty() {
        cfg.handles.insert("codeforces".into(), handle);
    }

    // BTreeMap keys are already sorted.
    let langs: Vec<&str> = cfg.compile_commands.keys().map(|s| s.as_str()).collect();
    let lang_list = langs.join(", ");
    let lang = ask(input, output, &format!("Default language ({lang_list})"), &cfg.default_language)?;
    if !cfg.compile_commands.contains_key(&lang) {
        return Err(anyhow!("unknown language {lang:?}; choose one of: {lang_list}"));
    }
    cfg.default_language = lang;

    cfg.workspace_dir = ask(input, output, "Workspace folder for solutions", &cfg.workspace_dir)?;
    cfg.editor = ask(input, output, "Editor command (blank to auto-detect)", &cfg.editor)?;

    writeln!(output, "Setup done. You can change these later in the Config tab.")?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn run_with(input: &str, cfg: Config) -> Result<(Config, String)> {
        let mut r = Cursor::new(input.as_bytes().to_vec());
        let mut out = Vec::new();
        let cfg = run(&mut r, &mut out, cfg)?;
        Ok((cfg, String::from_utf8(out).unwrap()))
    }

    #[test]
    fn enter_through_keeps_defaults() {
        let (cfg, _) = run_with("\n\n\n\n", Config::default()).unwrap();
        assert_eq!(cfg.default_language, "cpp");
        assert_eq!(cfg.theme, "catppuccin");
        assert!(cfg.handles.is_empty());
    }

    #[test]
    fn answers_are_applied() {
        let (cfg, _) = run_with("adarsh67\npython\n/work/cpx\ncode\n", Config::default()).unwrap();
        assert_eq!(cfg.handles["codeforces"], "adarsh67");
        assert_eq!(cfg.default_language, "python");
        assert_eq!(cfg.workspace_dir, "/work/cpx");
        assert_eq!(cfg.editor, "code");
    }

    #[test]
    fn unknown_language_is_rejected() {
        assert!(run_with("\nrust\n\n\n", Config::default()).is_err());
    }

    #[test]
    fn prompts_show_defaults() {
        let (_, out) = run_with("\n\n\n\n", Config::default()).unwrap();
        assert!(out.contains("Default language (cpp, python)"), "{out}");
    }
}
