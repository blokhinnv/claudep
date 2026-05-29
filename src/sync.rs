use std::fs;
use std::io::{copy, Cursor};
use std::path::Path;

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use tar::Archive;

use crate::config::{claudep_home, templates_dir};
use crate::templates::EMBEDDED_DOCKERFILE;

const GITHUB_REPO: &str = "blokhinnv/claudep";
const USER_AGENT: &str = "claudep-sync";

#[derive(Debug, serde::Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

pub fn sync_templates() -> Result<()> {
    let home = claudep_home()?;
    let target = templates_dir(&home);
    fs::create_dir_all(&target).with_context(|| {
        format!(
            "failed to create templates directory {}",
            target.display()
        )
    })?;

    match download_templates(&target) {
        Ok(version) => {
            println!("templates updated from release {version}");
            Ok(())
        }
        Err(remote_err) => {
            eprintln!("warning: could not fetch release templates: {remote_err}");
            eprintln!("writing embedded Dockerfile to {}", target.display());
            write_embedded_templates(&target)?;
            Ok(())
        }
    }
}

fn download_templates(target: &Path) -> Result<String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("failed to create HTTP client")?;

    let release: Release = client
        .get(format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest"))
        .send()
        .context("failed to query GitHub releases")?
        .error_for_status()
        .context("GitHub releases API returned an error")?
        .json()
        .context("failed to parse GitHub release JSON")?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == "templates.tar.gz")
        .with_context(|| format!("release {} has no templates.tar.gz asset", release.tag_name))?;

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .context("failed to download templates.tar.gz")?
        .error_for_status()
        .context("templates download failed")?
        .bytes()
        .context("failed to read templates archive")?;

    extract_templates(target, &bytes)?;
    Ok(release.tag_name)
}

fn extract_templates(target: &Path, bytes: &[u8]) -> Result<()> {
    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(decoder);
    for entry in archive.entries().context("failed to read templates archive")? {
        let mut entry = entry.context("failed to read templates archive entry")?;
        let path = entry.path().context("invalid path in templates archive")?;
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            bail!("invalid path in templates archive");
        }
        let out_path = target.join(path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create directory {}", parent.display())
            })?;
        }
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&out_path).with_context(|| {
                format!("failed to create directory {}", out_path.display())
            })?;
        } else {
            let mut file = fs::File::create(&out_path)
                .with_context(|| format!("failed to create {}", out_path.display()))?;
            copy(&mut entry, &mut file)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
        }
    }
    Ok(())
}

fn write_embedded_templates(target: &Path) -> Result<()> {
    fs::write(target.join("Dockerfile"), EMBEDDED_DOCKERFILE).with_context(|| {
        format!(
            "failed to write embedded Dockerfile to {}",
            target.join("Dockerfile").display()
        )
    })?;
    fs::write(target.join("VERSION"), env!("CARGO_PKG_VERSION")).with_context(|| {
        format!(
            "failed to write VERSION to {}",
            target.join("VERSION").display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn writes_embedded_templates() {
        let tmp = TempDir::new().unwrap();
        write_embedded_templates(tmp.path()).unwrap();
        assert!(tmp.path().join("Dockerfile").is_file());
        assert!(tmp.path().join("VERSION").is_file());
    }
}
