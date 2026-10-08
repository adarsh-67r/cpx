// `cpx update`: replaces this binary with the newest GitHub release build for
// this platform. Builds from source are told to pull and rebuild instead.

use crate::judge::get;
use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use std::process::Command;

const RELEASES: &str = "https://api.github.com/repos/adarsh-67r/cpx/releases?per_page=1";

/// The release asset name for this platform, matching release.yml.
fn asset_name() -> Result<&'static str> {
    Ok(match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "cpx-x86_64-pc-windows-msvc.zip",
        ("linux", "x86_64") => "cpx-x86_64-unknown-linux-gnu.tar.gz",
        ("macos", "aarch64") => "cpx-aarch64-apple-darwin.tar.gz",
        ("macos", "x86_64") => "cpx-x86_64-apple-darwin.tar.gz",
        (os, arch) => bail!("no release build for {os}/{arch}; build from source with cargo install --path ."),
    })
}

/// The newest release's tag and the download URL of the named asset in it.
pub fn latest(releases_json: &str, asset: &str) -> Result<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(releases_json)?;
    let rel = v.get(0).ok_or_else(|| anyhow!("no releases published yet"))?;
    let tag = rel["tag_name"].as_str().unwrap_or_default().to_string();
    let url = rel["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"] == asset))
        .and_then(|x| x["browser_download_url"].as_str())
        .ok_or_else(|| anyhow!("release {tag} has no {asset}"))?;
    Ok((tag, url.to_string()))
}

pub fn run(log: &mut dyn FnMut(String)) -> Result<()> {
    let exe = std::env::current_exe()?;
    let p = exe.to_string_lossy();
    if p.contains("/target/") || p.contains("\\target\\") {
        bail!("this cpx was built from source: run git pull, then cargo install --path .");
    }
    let asset = asset_name()?;
    let (tag, url) = latest(&get(RELEASES)?, asset)?;
    let current = format!("v{}", env!("CARGO_PKG_VERSION"));
    if tag == current {
        log(format!("already up to date ({current})"));
        return Ok(());
    }
    log(format!("downloading {tag} ({asset})"));

    let dir = std::env::temp_dir().join(format!("cpx-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let archive = dir.join(asset);
    let resp = ureq::get(&url).set("User-Agent", crate::judge::USER_AGENT).call().map_err(|e| anyhow!("GET {url}: {e}"))?;
    std::io::copy(&mut resp.into_reader(), &mut std::fs::File::create(&archive)?)?;
    extract(&archive, &dir)?;

    let name = if cfg!(windows) { "cpx.exe" } else { "cpx" };
    let new = dir.join(name);
    if !new.is_file() {
        bail!("{asset} did not contain {name}");
    }
    // A running exe cannot be overwritten on Windows, but it can be renamed.
    let old = exe.with_extension("old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).with_context(|| format!("move {}", exe.display()))?;
    if let Err(e) = std::fs::copy(&new, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(anyhow!("install {}: {e}", exe.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755))?;
        let _ = std::fs::remove_file(&old);
    }
    let _ = std::fs::remove_dir_all(&dir);
    log(format!("updated {current} → {tag}"));
    Ok(())
}

/// Unpacks with the system tar; Windows' own tar.exe also reads zip files.
fn extract(archive: &Path, dir: &Path) -> Result<()> {
    let tar = if cfg!(windows) { r"C:\Windows\System32\tar.exe" } else { "tar" };
    let ok = Command::new(tar).arg("-xf").arg(archive).arg("-C").arg(dir).status().context("run tar")?.success();
    if !ok {
        bail!("could not unpack {}", archive.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_this_platforms_asset_from_the_newest_release() {
        let json = r#"[{"tag_name":"v0.2.0","assets":[
            {"name":"cpx-x86_64-unknown-linux-gnu.tar.gz","browser_download_url":"https://x/linux"},
            {"name":"cpx-x86_64-pc-windows-msvc.zip","browser_download_url":"https://x/win"}]}]"#;
        assert_eq!(latest(json, "cpx-x86_64-pc-windows-msvc.zip").unwrap(), ("v0.2.0".into(), "https://x/win".into()));
        assert!(latest(json, "cpx-aarch64-apple-darwin.tar.gz").is_err());
        assert!(latest("[]", "x").is_err());
    }
}
