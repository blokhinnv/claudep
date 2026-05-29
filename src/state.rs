use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::config::ProjectContext;
use crate::templates::{self, EMBEDDED_DOCKERFILE, TEMPLATE_VERSION};

pub const COMPOSE_FILE: &str = "docker-compose.yml";
pub const MANIFEST_FILE: &str = ".render-manifest.json";
const DOCKERFILE: &str = "Dockerfile";
const CLAUDE_CONFIG_DIR: &str = "claude_config";
const CLAUDE_JSON_FILE: &str = ".claude.json";

#[derive(Debug, Error)]
pub enum StateError {
    #[error("template render failed: {0}")]
    Template(#[from] templates::TemplateError),
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("manifest error: {0}")]
    Manifest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderManifest {
    pub fingerprint: String,
    pub template_version: String,
    pub project_id: String,
    pub compose_project: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteResult {
    pub state_dir: PathBuf,
    pub regenerated: bool,
    pub manifest: RenderManifest,
}

pub fn fingerprint(ctx: &ProjectContext, state_dir_abs: &str) -> String {
    let payload = serde_json::json!({
        "template_version": TEMPLATE_VERSION,
        "project_root": ctx.project_root,
        "settings": ctx.settings,
        "runtime": ctx.runtime,
        "state_dir": state_dir_abs,
    });
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    hex::encode(Sha256::digest(bytes))
}

pub fn ensure_state_dir(state_dir: &Path) -> Result<(), StateError> {
    fs::create_dir_all(state_dir).map_err(|e| StateError::Io {
        path: state_dir.display().to_string(),
        source: e,
    })
}

fn read_manifest(path: &Path) -> Result<Option<RenderManifest>, StateError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|e| StateError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    serde_json::from_str(&content).map_err(|e| StateError::Manifest(e.to_string()))
}

fn write_file(path: &Path, content: &str) -> Result<(), StateError> {
    fs::write(path, content).map_err(|e| StateError::Io {
        path: path.display().to_string(),
        source: e,
    })
}

fn dockerfile_content(ctx: &ProjectContext) -> String {
    let template_path = ctx.templates_dir.join(DOCKERFILE);
    fs::read_to_string(&template_path).unwrap_or_else(|_| EMBEDDED_DOCKERFILE.to_string())
}

/// Claude Code config bind-mounts from `state_dir`; permissive modes for dynamic container UID.
pub fn ensure_claude_home_artifacts(state_dir: &Path) -> Result<(), StateError> {
    let config_dir = state_dir.join(CLAUDE_CONFIG_DIR);
    fs::create_dir_all(&config_dir).map_err(|e| StateError::Io {
        path: config_dir.display().to_string(),
        source: e,
    })?;
    #[cfg(unix)]
    fs::set_permissions(&config_dir, fs::Permissions::from_mode(0o777)).map_err(|e| {
        StateError::Io {
            path: config_dir.display().to_string(),
            source: e,
        }
    })?;

    let claude_json = state_dir.join(CLAUDE_JSON_FILE);
    if !claude_json.is_file() {
        write_file(&claude_json, "{}\n")?;
    }
    #[cfg(unix)]
    fs::set_permissions(&claude_json, fs::Permissions::from_mode(0o666)).map_err(|e| {
        StateError::Io {
            path: claude_json.display().to_string(),
            source: e,
        }
    })?;
    Ok(())
}

/// Write or refresh artifacts under `ctx.state_dir`.
pub fn write_artifacts(ctx: &ProjectContext) -> Result<WriteResult, StateError> {
    let state_dir = ctx.state_dir.clone();
    ensure_state_dir(&state_dir)?;
    ensure_claude_home_artifacts(&state_dir)?;
    let state_dir_str = state_dir.to_string_lossy().into_owned();
    let fp = fingerprint(ctx, &state_dir_str);
    let manifest_path = state_dir.join(MANIFEST_FILE);
    let existing = read_manifest(&manifest_path)?;

    let new_manifest = RenderManifest {
        fingerprint: fp.clone(),
        template_version: TEMPLATE_VERSION.to_string(),
        project_id: ctx.project_id.clone(),
        compose_project: ctx.compose_project.clone(),
    };

    let regenerated = existing
        .as_ref()
        .map(|m| m.fingerprint != fp)
        .unwrap_or(true);

    if regenerated {
        let compose = templates::render_for_context(ctx, &state_dir_str)?;
        write_file(&state_dir.join(COMPOSE_FILE), &compose)?;
        write_file(&state_dir.join(DOCKERFILE), &dockerfile_content(ctx))?;
        let manifest_json = serde_json::to_string_pretty(&new_manifest)
            .map_err(|e| StateError::Manifest(e.to_string()))?;
        write_file(&manifest_path, &manifest_json)?;
    }

    Ok(WriteResult {
        state_dir,
        regenerated,
        manifest: new_manifest,
    })
}

/// Remove generated artifacts for a project (`state_dir`).
pub fn remove_state_dir(state_dir: &Path) -> Result<bool, StateError> {
    if !state_dir.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(state_dir).map_err(|e| StateError::Io {
        path: state_dir.display().to_string(),
        source: e,
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        runtime_from_env, ClaudepSettings, DEFAULT_GOST_IMAGE, DEFAULT_NODE_IMAGE,
    };
    use tempfile::TempDir;

    fn test_context(proxy: &str, root: &str, home: &Path) -> ProjectContext {
        let project_id = crate::project_id::project_id(root);
        ProjectContext {
            project_root: root.into(),
            project_id: project_id.clone(),
            compose_project: format!("claudep-{project_id}"),
            settings: ClaudepSettings {
                upstream: proxy.into(),
                gost_image: DEFAULT_GOST_IMAGE.into(),
                node_image: DEFAULT_NODE_IMAGE.into(),
            },
            runtime: runtime_from_env(root),
            home: home.to_path_buf(),
            templates_dir: home.join("templates"),
            state_dir: home.join("state").join(&project_id),
        }
    }

    #[test]
    fn writes_compose_and_dockerfile() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context("socks5://127.0.0.1:1080", "/tmp/proj-a", tmp.path());
        let result = write_artifacts(&ctx).unwrap();
        assert!(result.regenerated);
        assert!(result.state_dir.join(COMPOSE_FILE).is_file());
        assert!(result.state_dir.join(DOCKERFILE).is_file());
        assert!(result.state_dir.join(CLAUDE_CONFIG_DIR).is_dir());
        assert!(result.state_dir.join(CLAUDE_JSON_FILE).is_file());
        let compose = fs::read_to_string(result.state_dir.join(COMPOSE_FILE)).unwrap();
        assert!(compose.contains("socks5://127.0.0.1:1080"));
        assert!(compose.contains("/var/home/.claude"));
    }

    #[test]
    fn skips_rewrite_when_unchanged() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context("socks5://127.0.0.1:1080", "/tmp/proj-b", tmp.path());
        write_artifacts(&ctx).unwrap();
        let second = write_artifacts(&ctx).unwrap();
        assert!(!second.regenerated);
    }

    #[test]
    fn regenerates_when_upstream_changes() {
        let tmp = TempDir::new().unwrap();
        let root = "/tmp/proj-c";
        write_artifacts(&test_context("socks5://127.0.0.1:1080", root, tmp.path())).unwrap();
        let second =
            write_artifacts(&test_context("socks5://127.0.0.1:1081", root, tmp.path())).unwrap();
        assert!(second.regenerated);
    }

    #[test]
    fn remove_state_dir_deletes_artifacts() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context("socks5://127.0.0.1:1080", "/tmp/proj-d", tmp.path());
        write_artifacts(&ctx).unwrap();
        assert!(ctx.state_dir.is_dir());
        assert!(remove_state_dir(&ctx.state_dir).unwrap());
        assert!(!ctx.state_dir.exists());
        assert!(!remove_state_dir(&ctx.state_dir).unwrap());
    }
}
