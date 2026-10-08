// Talking to the OS: opening a URL or file, and the clipboard.

use anyhow::{anyhow, Result};
use std::io::Write;
use std::process::{Command, Stdio};

/// Opens a page in the default browser.
pub fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

/// Opens a file in the configured editor, VS Code if one is on PATH, or the
/// OS default app. The editor setting may contain `{file}`; otherwise the
/// path is appended.
pub fn open_in_editor(editor: &str, path: &str) -> Result<()> {
    if !editor.is_empty() {
        let line = if editor.contains("{file}") { editor.replace("{file}", path) } else { format!("{editor} {path}") };
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd").args(["/C", &line]).spawn()?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh").args(["-c", &line]).spawn()?;
        }
        return Ok(());
    }
    if which_code() {
        Command::new("code").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", "", path]).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

fn which_code() -> bool {
    Command::new("code").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok()
}

/// Puts text on the system clipboard.
pub fn copy_clipboard(text: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    let mut cmd = Command::new("clip");
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("pbcopy");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard"]);
        c
    };
    let mut child = cmd.stdin(Stdio::piped()).spawn()?;
    child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?.write_all(text.as_bytes())?;
    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow!("clipboard command failed"));
    }
    Ok(())
}
