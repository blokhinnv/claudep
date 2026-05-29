use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::project_id;

pub const DEFAULT_GOST_IMAGE: &str = "ginuerzh/gost:2.12.0";
pub const DEFAULT_NODE_IMAGE: &str = "node:22-slim";
pub const DEFAULT_USER_ID: &str = "1000";
pub const DEFAULT_GROUP_ID: &str = "1000";
pub const APP_SERVICE: &str = "app";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudepSettings {
    pub upstream: String,
    #[serde(default = "default_gost_image")]
    pub gost_image: String,
    #[serde(default = "default_node_image")]
    pub node_image: String,
}

fn default_gost_image() -> String {
    DEFAULT_GOST_IMAGE.to_string()
}

fn default_node_image() -> String {
    DEFAULT_NODE_IMAGE.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeContext {
    pub project_dir: String,
    pub user_id: String,
    pub group_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectContext {
    pub project_root: String,
    pub project_id: String,
    pub compose_project: String,
    pub settings: ClaudepSettings,
    pub runtime: RuntimeContext,
    pub home: PathBuf,
    pub templates_dir: PathBuf,
    pub state_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "CLAUDEP_UPSTREAM is required (set upstream proxy for gost, e.g. socks5://127.0.0.1:1080)"
    )]
    MissingUpstream,
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

impl ClaudepSettings {
    pub fn from_env() -> Result<Self, ConfigError> {
        let upstream = env::var("CLAUDEP_UPSTREAM").unwrap_or_default();
        let settings = Self {
            upstream,
            gost_image: env::var("CLAUDEP_GOST_IMAGE")
                .unwrap_or_else(|_| DEFAULT_GOST_IMAGE.to_string()),
            node_image: env::var("CLAUDEP_NODE_IMAGE")
                .unwrap_or_else(|_| DEFAULT_NODE_IMAGE.to_string()),
        };
        settings.validate()?;
        Ok(settings)
    }

    pub fn from_env_optional_upstream() -> Result<Self, ConfigError> {
        let upstream = env::var("CLAUDEP_UPSTREAM").unwrap_or_default();
        let settings = Self {
            upstream,
            gost_image: env::var("CLAUDEP_GOST_IMAGE")
                .unwrap_or_else(|_| DEFAULT_GOST_IMAGE.to_string()),
            node_image: env::var("CLAUDEP_NODE_IMAGE")
                .unwrap_or_else(|_| DEFAULT_NODE_IMAGE.to_string()),
        };
        Ok(settings)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.upstream.trim().is_empty() {
            return Err(ConfigError::MissingUpstream);
        }
        Ok(())
    }
}

pub fn claudep_home() -> Result<PathBuf> {
    if let Ok(home) = env::var("CLAUDEP_HOME") {
        if !home.trim().is_empty() {
            return Ok(PathBuf::from(home));
        }
    }
    let base = BaseDirs::new().context("could not resolve home directory")?;
    Ok(base.data_local_dir().join("claudep"))
}

pub fn templates_dir(home: &Path) -> PathBuf {
    env::var("CLAUDEP_TEMPLATES")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("templates"))
}

pub fn runtime_from_env(project_dir: impl Into<String>) -> RuntimeContext {
    let project_dir = project_dir.into();
    let user_id = env::var("UID").unwrap_or_else(|_| DEFAULT_USER_ID.to_string());
    let group_id = env::var("GID").unwrap_or_else(|_| DEFAULT_GROUP_ID.to_string());
    RuntimeContext {
        project_dir,
        user_id,
        group_id,
    }
}

pub fn resolve_project_root(project_dir: Option<&Path>) -> Result<PathBuf> {
    let root = match project_dir {
        Some(path) => path.to_path_buf(),
        None => env::current_dir().context("could not read current directory")?,
    };
    root.canonicalize()
        .with_context(|| format!("could not resolve project directory: {}", root.display()))
}

impl ProjectContext {
    pub fn from_env(project_dir: Option<&Path>, require_upstream: bool) -> Result<Self> {
        let project_root = resolve_project_root(project_dir)?;
        let project_root_str = project_root.to_string_lossy().into_owned();
        let settings = if require_upstream {
            ClaudepSettings::from_env().map_err(|e| anyhow::anyhow!("{e}"))?
        } else {
            ClaudepSettings::from_env_optional_upstream().map_err(|e| anyhow::anyhow!("{e}"))?
        };
        let home = claudep_home()?;
        let templates_dir = templates_dir(&home);
        let project_id = project_id::project_id(&project_root_str);
        let compose_project = project_id::compose_project(&project_root_str);
        let state_dir = home.join("state").join(&project_id);
        let runtime = runtime_from_env(&project_root_str);
        Ok(Self {
            project_root: project_root_str,
            project_id,
            compose_project,
            settings,
            runtime,
            home,
            templates_dir,
            state_dir,
        })
    }
}

pub fn config_error_message(err: ConfigError) -> String {
    err.to_string()
}

pub fn ensure_upstream(settings: &ClaudepSettings) -> Result<()> {
    settings.validate().map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_upstream() {
        let settings = ClaudepSettings {
            upstream: "  ".into(),
            gost_image: DEFAULT_GOST_IMAGE.into(),
            node_image: DEFAULT_NODE_IMAGE.into(),
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn builds_project_context() {
        let ctx = ProjectContext {
            project_root: "/tmp/proj".into(),
            project_id: project_id::project_id("/tmp/proj"),
            compose_project: project_id::compose_project("/tmp/proj"),
            settings: ClaudepSettings {
                upstream: "socks5://127.0.0.1:1080".into(),
                gost_image: DEFAULT_GOST_IMAGE.into(),
                node_image: DEFAULT_NODE_IMAGE.into(),
            },
            runtime: runtime_from_env("/tmp/proj"),
            home: PathBuf::from("/tmp/claudep"),
            templates_dir: PathBuf::from("/tmp/claudep/templates"),
            state_dir: PathBuf::from("/tmp/claudep/state/abc"),
        };
        assert_eq!(ctx.project_id.len(), 12);
        assert!(ctx.compose_project.starts_with("claudep-"));
    }
}
