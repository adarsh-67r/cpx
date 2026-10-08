// User settings, stored as JSON in the config dir. The field names match the
// Go version, so an existing config.json loads unchanged.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub handles: BTreeMap<String, String>,
    pub default_language: String,
    pub theme: String,
    pub workspace_dir: String,
    pub editor: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cses_session: String,
    pub compile_commands: BTreeMap<String, CompileCommand>,
}

/// How to build and run one language. {source}, {output}, {dir} are filled in
/// by the runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CompileCommand {
    pub compile: String,
    pub run: String,
    pub extension: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut compile_commands = BTreeMap::new();
        compile_commands.insert(
            "cpp".to_string(),
            CompileCommand {
                compile: "g++ -std=c++17 -O2 -o {output} {source}".into(),
                run: "{dir}/{output}".into(),
                extension: ".cpp".into(),
            },
        );
        compile_commands.insert(
            "python".to_string(),
            CompileCommand {
                compile: String::new(),
                run: "python3 {source}".into(),
                extension: ".py".into(),
            },
        );
        Config {
            handles: BTreeMap::new(),
            default_language: "cpp".into(),
            theme: "catppuccin".into(),
            workspace_dir: String::new(),
            editor: String::new(),
            cses_session: String::new(),
            compile_commands,
        }
    }
}

/// Config file location: <config dir>/cpx/config.json.
pub fn path() -> Result<PathBuf> {
    let dir = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config directory on this system"))?;
    Ok(dir.join("cpx").join("config.json"))
}

/// Reads the file at path. A missing file returns the defaults.
pub fn load(path: &Path) -> Result<Config> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e.into()),
    }
}

/// Writes cfg to path, creating parent folders.
pub fn save(path: &Path, cfg: &Config) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(cfg)?;
    text.push('\n');
    fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = std::env::temp_dir().join("cpx-config-test-missing");
        let cfg = load(&dir.join("nope.json")).unwrap();
        assert_eq!(cfg.default_language, "cpp");
        assert_eq!(cfg.theme, "catppuccin");
    }

    #[test]
    fn round_trip_keeps_fields() {
        let dir = std::env::temp_dir().join(format!("cpx-config-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let mut cfg = Config::default();
        cfg.handles.insert("codeforces".into(), "tourist".into());
        save(&path, &cfg).unwrap();
        let got = load(&path).unwrap();
        assert_eq!(got.handles["codeforces"], "tourist");
        assert!(got.compile_commands.contains_key("cpp"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn go_written_config_loads() {
        // The shape the Go version writes, including the theme field.
        let text = r#"{"handles":{"codeforces":"adarsh67"},"default_language":"cpp","theme":"catppuccin","workspace_dir":"","editor":"","compile_commands":{"cpp":{"compile":"g++ -std=c++17 -O2 -o {output} {source}","run":"{dir}/{output}","extension":".cpp"}}}"#;
        let cfg: Config = serde_json::from_str(text).unwrap();
        assert_eq!(cfg.handles["codeforces"], "adarsh67");
    }
}
