use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::config::{ProjectContext, APP_SERVICE};
use crate::state::COMPOSE_FILE;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(60);
const HEALTH_POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,
    pub health: Option<String>,
}

pub fn compose_file(ctx: &ProjectContext) -> std::path::PathBuf {
    ctx.state_dir.join(COMPOSE_FILE)
}

fn base_command(ctx: &ProjectContext) -> Command {
    let mut cmd = Command::new("docker");
    cmd.arg("compose")
        .arg("-f")
        .arg(compose_file(ctx))
        .arg("-p")
        .arg(&ctx.compose_project);
    cmd
}

fn run_command(mut cmd: Command) -> Result<Output> {
    let output = cmd
        .output()
        .context("failed to run docker compose (is Docker installed and in PATH?)")?;
    Ok(output)
}

pub fn check_docker_available() -> Result<()> {
    let output = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run docker (is Docker installed?)")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Docker daemon is not available: {}", stderr.trim());
    }
    Ok(())
}

pub fn check_compose_available() -> Result<()> {
    let output = Command::new("docker")
        .arg("compose")
        .arg("version")
        .output()
        .context("failed to run docker compose")?;
    if !output.status.success() {
        bail!("docker compose v2 is required but not available");
    }
    Ok(())
}

pub fn compose_up(ctx: &ProjectContext) -> Result<()> {
    let mut cmd = base_command(ctx);
    cmd.arg("up").arg("-d").arg("--build");
    let output = run_command(cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker compose up failed:\n{stderr}");
    }
    Ok(())
}

pub fn compose_down(ctx: &ProjectContext) -> Result<()> {
    compose_down_with_options(ctx, false)
}

pub fn compose_down_with_options(ctx: &ProjectContext, remove_image: bool) -> Result<()> {
    if compose_file(ctx).is_file() {
        down_with_compose_file(ctx, remove_image)
    } else {
        down_by_project_name(&ctx.compose_project, remove_image)
    }
}

fn down_with_compose_file(ctx: &ProjectContext, remove_image: bool) -> Result<()> {
    let mut cmd = base_command(ctx);
    cmd.arg("down");
    if remove_image {
        cmd.arg("--rmi").arg("local");
    }
    let output = run_command(cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker compose down failed:\n{stderr}");
    }
    Ok(())
}

fn down_by_project_name(compose_project: &str, remove_image: bool) -> Result<()> {
    let mut cmd = Command::new("docker");
    cmd.arg("compose")
        .arg("-p")
        .arg(compose_project)
        .arg("down");
    if remove_image {
        cmd.arg("--rmi").arg("local");
    }
    let output = cmd
        .output()
        .context("failed to run docker compose (is Docker installed and in PATH?)")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker compose down failed:\n{stderr}");
    }
    Ok(())
}

pub fn compose_ps(ctx: &ProjectContext) -> Result<String> {
    let mut cmd = base_command(ctx);
    cmd.arg("ps");
    let output = run_command(cmd)?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker compose ps failed:\n{stderr}");
    }
    Ok(stdout)
}

pub fn service_statuses(ctx: &ProjectContext) -> Result<Vec<ServiceStatus>> {
    let mut cmd = base_command(ctx);
    cmd.args(["ps", "--format", "json"]);
    let output = run_command(cmd)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("docker compose ps failed:\n{stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut statuses = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .with_context(|| format!("failed to parse compose ps output: {line}"))?;
        statuses.push(ServiceStatus {
            name: value
                .get("Service")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            state: value
                .get("State")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            health: value
                .get("Health")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        });
    }
    Ok(statuses)
}

pub fn is_service_running(ctx: &ProjectContext, service: &str) -> Result<bool> {
    let statuses = service_statuses(ctx)?;
    Ok(statuses
        .iter()
        .any(|s| s.name == service && s.state.eq_ignore_ascii_case("running")))
}

pub fn stack_already_running(ctx: &ProjectContext) -> Result<bool> {
    let statuses = service_statuses(ctx)?;
    let gost_running = statuses
        .iter()
        .any(|s| s.name == "gost" && s.state.eq_ignore_ascii_case("running"));
    let app_running = statuses
        .iter()
        .any(|s| s.name == APP_SERVICE && s.state.eq_ignore_ascii_case("running"));
    Ok(gost_running && app_running)
}

pub fn wait_for_gost_healthy(ctx: &ProjectContext) -> Result<()> {
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    loop {
        let statuses = service_statuses(ctx)?;
        if let Some(gost) = statuses.iter().find(|s| s.name == "gost") {
            if gost.state.eq_ignore_ascii_case("running") {
                if gost
                    .health
                    .as_deref()
                    .is_some_and(|h| h.eq_ignore_ascii_case("healthy"))
                {
                    return Ok(());
                }
                if gost.health.is_none() {
                    return Ok(());
                }
            }
        }

        if Instant::now() >= deadline {
            bail!(
                "timed out waiting for gost to become healthy ({}s)",
                HEALTH_TIMEOUT.as_secs()
            );
        }
        thread::sleep(HEALTH_POLL);
    }
}

pub fn compose_exec_interactive(ctx: &ProjectContext, shell: bool) -> Result<()> {
    if !is_service_running(ctx, APP_SERVICE)? {
        bail!("service '{APP_SERVICE}' is not running; run `claudep` first to start the stack");
    }

    let exec_target = if shell { "bash" } else { "claude" };
    let mut cmd = base_command(ctx);
    cmd.arg("exec")
        .arg("-it")
        .arg(APP_SERVICE)
        .arg(exec_target)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status().context("failed to run docker compose exec")?;
    if !status.success() {
        bail!("docker compose exec exited with status {status}");
    }
    Ok(())
}

pub fn ensure_stack(ctx: &ProjectContext) -> Result<EnsureResult> {
    check_docker_available()?;
    check_compose_available()?;

    let already = stack_already_running(ctx)?;
    compose_up(ctx)?;
    wait_for_gost_healthy(ctx)?;

    if already {
        Ok(EnsureResult::AlreadyRunning)
    } else {
        Ok(EnsureResult::Started)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsureResult {
    Started,
    AlreadyRunning,
}

impl EnsureResult {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Started => "stack started and ready",
            Self::AlreadyRunning => "stack already running",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_result_messages() {
        assert_eq!(EnsureResult::Started.message(), "stack started and ready");
        assert_eq!(
            EnsureResult::AlreadyRunning.message(),
            "stack already running"
        );
    }
}
