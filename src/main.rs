use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use claudep::{
    check_compose_available, check_docker_available, compose_down, compose_down_with_options,
    compose_exec_interactive, compose_file, compose_ps, ensure_stack, ensure_upstream,
    is_service_running, remove_state_dir, sync_templates, write_artifacts, ProjectContext,
    APP_SERVICE,
};

#[derive(Parser, Debug)]
#[command(
    name = "claudep",
    version,
    about = "Claude Code in an isolated Docker stack with gost proxy"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Project directory (default: current working directory)
    #[arg(long, global = true)]
    project_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Ensure the Docker stack for the current project is running
    Up,
    /// Open an interactive claude session (or shell with --shell) in the app container
    Attach {
        /// Open bash instead of claude
        #[arg(long)]
        shell: bool,
    },
    /// Stop the Docker stack for the current project
    Down,
    /// Stop the stack, remove local app image, and delete generated state
    Remove {
        /// Also remove the locally built app image
        #[arg(long)]
        image: bool,
    },
    /// Show stack status
    Status,
    /// Check Docker, environment variables, and tooling
    Doctor,
    /// Update templates from the latest GitHub release
    Sync,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Up) => cmd_up(cli.project_dir.as_deref()),
        Some(Commands::Attach { shell }) => cmd_attach(cli.project_dir.as_deref(), shell),
        Some(Commands::Down) => cmd_down(cli.project_dir.as_deref()),
        Some(Commands::Remove { image }) => cmd_remove(cli.project_dir.as_deref(), image),
        Some(Commands::Status) => cmd_status(cli.project_dir.as_deref()),
        Some(Commands::Doctor) => cmd_doctor(),
        Some(Commands::Sync) => cmd_sync(),
    }
}

fn cmd_up(project_dir: Option<&std::path::Path>) -> Result<()> {
    let ctx = ProjectContext::from_env(project_dir, true)?;
    ensure_upstream(&ctx.settings)?;

    let write = write_artifacts(&ctx)?;
    if write.regenerated {
        println!("generated artifacts in {}", write.state_dir.display());
    }

    let result = ensure_stack(&ctx)?;
    println!(
        "{} (project {}, compose {})",
        result.message(),
        ctx.project_id,
        ctx.compose_project
    );
    println!("run `claudep attach` to start Claude Code");
    Ok(())
}

fn cmd_attach(project_dir: Option<&std::path::Path>, shell: bool) -> Result<()> {
    let ctx = ProjectContext::from_env(project_dir, true)?;
    ensure_upstream(&ctx.settings)?;
    write_artifacts(&ctx)?;

    if !is_service_running(&ctx, APP_SERVICE)? {
        println!("stack not running, starting...");
        ensure_stack(&ctx)?;
    }

    compose_exec_interactive(&ctx, shell)
}

fn cmd_down(project_dir: Option<&std::path::Path>) -> Result<()> {
    let ctx = ProjectContext::from_env(project_dir, false)?;
    compose_down(&ctx)?;
    println!("stopped stack for project {}", ctx.project_id);
    Ok(())
}

fn cmd_remove(project_dir: Option<&std::path::Path>, remove_image: bool) -> Result<()> {
    let ctx = ProjectContext::from_env(project_dir, false)?;

    if ctx.state_dir.is_dir() || compose_file(&ctx).is_file() {
        match compose_down_with_options(&ctx, remove_image) {
            Ok(()) => println!("stopped stack for project {}", ctx.project_id),
            Err(err) => {
                eprintln!("warning: could not stop stack: {err:#}");
            }
        }
    }

    let removed = remove_state_dir(&ctx.state_dir)
        .map_err(|e| anyhow::anyhow!("failed to remove state dir: {e}"))?;
    if removed {
        println!("removed state {}", ctx.state_dir.display());
    } else {
        println!("no state found for project {}", ctx.project_id);
    }
    Ok(())
}

fn cmd_status(project_dir: Option<&std::path::Path>) -> Result<()> {
    let ctx = ProjectContext::from_env(project_dir, false)?;
    println!("project root:  {}", ctx.project_root);
    println!("project id:    {}", ctx.project_id);
    println!("compose name:  {}", ctx.compose_project);
    println!("state dir:     {}", ctx.state_dir.display());
    println!("templates dir: {}", ctx.templates_dir.display());
    if ctx.settings.upstream.trim().is_empty() {
        println!("upstream:      (not set)");
    } else {
        println!("upstream:      {}", ctx.settings.upstream);
    }

    match compose_ps(&ctx) {
        Ok(ps) => {
            if ps.trim().is_empty() {
                println!("\ncontainers: (none running)");
            } else {
                println!("\n{ps}");
            }
        }
        Err(err) => {
            println!("\ncontainers: unavailable ({err})");
        }
    }
    Ok(())
}

fn cmd_doctor() -> Result<()> {
    let mut ok = true;

    print!("docker: ");
    match check_docker_available() {
        Ok(()) => println!("ok"),
        Err(err) => {
            ok = false;
            println!("{err}");
        }
    }

    print!("docker compose: ");
    match check_compose_available() {
        Ok(()) => println!("ok"),
        Err(err) => {
            ok = false;
            println!("{err}");
        }
    }

    let home = claudep::claudep_home()?;
    println!("CLAUDEP_HOME: {}", home.display());

    let templates = claudep::templates_dir(&home);
    println!("CLAUDEP_TEMPLATES: {}", templates.display());
    if templates.join("Dockerfile").is_file() {
        println!("templates Dockerfile: present");
    } else {
        println!("templates Dockerfile: missing (run `claudep sync` or reinstall)");
    }

    match std::env::var("CLAUDEP_UPSTREAM") {
        Ok(value) if !value.trim().is_empty() => println!("CLAUDEP_UPSTREAM: {value}"),
        _ => {
            ok = false;
            println!("CLAUDEP_UPSTREAM: not set (required for `claudep` and `claudep attach`)");
        }
    }

    if let Ok(install_dir) = std::env::var("CLAUDEP_INSTALL_DIR") {
        println!("CLAUDEP_INSTALL_DIR: {install_dir}");
    }

    println!("claudep version: {}", env!("CARGO_PKG_VERSION"));

    if ok {
        println!("\nall checks passed");
    } else {
        println!("\nsome checks failed (see above)");
    }
    Ok(())
}

fn cmd_sync() -> Result<()> {
    sync_templates()
}
