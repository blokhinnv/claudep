use serde::Serialize;
use thiserror::Error;

use crate::config::{ClaudepSettings, ProjectContext};

pub const TEMPLATE_VERSION: &str = "2";
pub const EMBEDDED_DOCKERFILE: &str = include_str!("../templates/Dockerfile");

const GOST_SERVICE: &str = "gost";

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("failed to serialize docker-compose.yml: {0}")]
    Yaml(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderInput<'a> {
    pub settings: &'a ClaudepSettings,
    pub project_dir: &'a str,
    pub state_dir: &'a str,
    pub user_id: &'a str,
    pub group_id: &'a str,
}

#[derive(Debug, Serialize)]
struct ComposeFile {
    services: ComposeServices,
}

#[derive(Debug, Serialize)]
struct ComposeServices {
    gost: GostService,
    app: AppService,
}

#[derive(Debug, Serialize)]
struct GostService {
    image: String,
    restart: String,
    command: Vec<String>,
    expose: Vec<String>,
    healthcheck: Healthcheck,
}

#[derive(Debug, Serialize)]
struct Healthcheck {
    test: String,
    interval: String,
}

#[derive(Debug, Serialize)]
struct AppService {
    build: BuildSpec,
    stdin_open: bool,
    tty: bool,
    user: String,
    environment: AppEnvironment,
    volumes: Vec<String>,
    restart: String,
    depends_on: DependsOn,
}

#[derive(Debug, Serialize)]
struct BuildSpec {
    context: String,
    dockerfile: String,
    args: BuildArgs,
}

#[derive(Debug, Serialize)]
struct BuildArgs {
    #[serde(rename = "NODE_IMAGE")]
    node_image: String,
}

#[derive(Debug, Serialize)]
struct AppEnvironment {
    #[serde(rename = "HTTP_PROXY")]
    http_proxy: String,
    #[serde(rename = "HTTPS_PROXY")]
    https_proxy: String,
    #[serde(rename = "ALL_PROXY")]
    all_proxy: String,
    #[serde(rename = "NO_PROXY")]
    no_proxy: String,
    #[serde(rename = "HOME")]
    home: String,
}

#[derive(Debug, Serialize)]
struct DependsOn {
    gost: ServiceCondition,
}

#[derive(Debug, Serialize)]
struct ServiceCondition {
    condition: String,
}

pub fn render_compose(input: &RenderInput<'_>) -> Result<String, TemplateError> {
    let upstream = &input.settings.upstream;

    let compose = ComposeFile {
        services: ComposeServices {
            gost: GostService {
                image: input.settings.gost_image.clone(),
                restart: "unless-stopped".into(),
                command: vec!["-L=auto://0.0.0.0:1080".into(), format!("-F={upstream}")],
                expose: vec!["1080".into()],
                healthcheck: Healthcheck {
                    test: format!(
                        "wget --quiet --spider http://{GOST_SERVICE}:1080 2>&1 | grep HTTP/1.1"
                    ),
                    interval: "10s".into(),
                },
            },
            app: AppService {
                build: BuildSpec {
                    context: input.state_dir.to_string(),
                    dockerfile: "Dockerfile".into(),
                    args: BuildArgs {
                        node_image: input.settings.node_image.clone(),
                    },
                },
                stdin_open: true,
                tty: true,
                user: format!("{}:{}", input.user_id, input.group_id),
                environment: AppEnvironment {
                    http_proxy: format!("http://{GOST_SERVICE}:1080"),
                    https_proxy: format!("http://{GOST_SERVICE}:1080"),
                    all_proxy: format!("http://{GOST_SERVICE}:1080"),
                    no_proxy: "localhost,127.0.0.1".into(),
                    home: "/var/home".into(),
                },
                volumes: vec![
                    format!("{}:/app", input.project_dir),
                    format!("{}/claude_config:/var/home/.claude", input.state_dir),
                    format!("{}/.claude.json:/var/home/.claude.json", input.state_dir),
                ],
                restart: "unless-stopped".into(),
                depends_on: DependsOn {
                    gost: ServiceCondition {
                        condition: "service_healthy".into(),
                    },
                },
            },
        },
    };

    serde_yaml::to_string(&compose).map_err(|e| TemplateError::Yaml(e.to_string()))
}

pub fn render_for_context(ctx: &ProjectContext, state_dir: &str) -> Result<String, TemplateError> {
    let project_dir = if ctx.runtime.project_dir.is_empty() {
        ctx.project_root.as_str()
    } else {
        ctx.runtime.project_dir.as_str()
    };
    render_compose(&RenderInput {
        settings: &ctx.settings,
        project_dir,
        state_dir,
        user_id: &ctx.runtime.user_id,
        group_id: &ctx.runtime.group_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        runtime_from_env, ClaudepSettings, DEFAULT_GOST_IMAGE, DEFAULT_NODE_IMAGE,
    };

    fn sample_context(upstream: &str) -> ProjectContext {
        ProjectContext {
            project_root: "/Users/me/my-project".into(),
            project_id: "abc".into(),
            compose_project: "claudep-abc".into(),
            settings: ClaudepSettings {
                upstream: upstream.into(),
                gost_image: DEFAULT_GOST_IMAGE.into(),
                node_image: DEFAULT_NODE_IMAGE.into(),
            },
            runtime: runtime_from_env("/Users/me/my-project"),
            home: "/tmp/claudep".into(),
            templates_dir: "/tmp/claudep/templates".into(),
            state_dir: "/tmp/claudep/state/abc".into(),
        }
    }

    #[test]
    fn rendered_yaml_has_literal_upstream() {
        let ctx = sample_context("relay+wss://user:pass@host:port");
        let yaml = render_for_context(&ctx, "/tmp/state").unwrap();
        assert!(yaml.contains("relay+wss://user:pass@host:port"));
        assert!(!yaml.contains("${"));
        assert!(!yaml.contains(".env"));
        let _: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    }

    #[test]
    fn special_chars_in_upstream_are_valid_yaml() {
        let ctx = sample_context("relay+wss://u:p@h:1+extra");
        let yaml = render_for_context(&ctx, "/tmp/state").unwrap();
        serde_yaml::from_str::<serde_yaml::Value>(&yaml).unwrap();
        assert!(yaml.contains("@"));
    }

    #[test]
    fn dockerfile_is_embedded() {
        assert!(EMBEDDED_DOCKERFILE.contains("claude-code"));
        assert!(EMBEDDED_DOCKERFILE.contains("/var/home"));
        assert!(!EMBEDDED_DOCKERFILE.contains("uv"));
    }

    #[test]
    fn uses_gost_and_app_service_names() {
        let ctx = sample_context("socks5://127.0.0.1:1080");
        let yaml = render_for_context(&ctx, "/tmp/state").unwrap();
        assert!(yaml.contains("gost:"));
        assert!(yaml.contains("app:"));
    }

    #[test]
    fn compose_mounts_claude_config_from_state_dir() {
        let ctx = sample_context("socks5://127.0.0.1:1080");
        let yaml = render_for_context(&ctx, "/tmp/state").unwrap();
        assert!(yaml.contains("/tmp/state/claude_config:/var/home/.claude"));
        assert!(yaml.contains("/tmp/state/.claude.json:/var/home/.claude.json"));
    }
}
