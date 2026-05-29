use std::path::Path;

use anyhow::Result;
use claudep::{
    claudep_home, project_id, write_artifacts, ClaudepSettings, ProjectContext, runtime_from_env,
};

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/claudep-fixture-project".to_string());
    let upstream = std::env::var("CLAUDEP_UPSTREAM")
        .unwrap_or_else(|_| "socks5://127.0.0.1:1080".to_string());

    let home = Path::new("/tmp/claudep-fixture-state");
    let id = project_id(&root);
    let ctx = ProjectContext {
        project_root: root.clone(),
        project_id: id.clone(),
        compose_project: format!("claudep-{id}"),
        settings: ClaudepSettings {
            upstream,
            gost_image: claudep::DEFAULT_GOST_IMAGE.into(),
            node_image: claudep::DEFAULT_NODE_IMAGE.into(),
        },
        runtime: runtime_from_env(&root),
        home: claudep_home().unwrap_or_else(|_| home.to_path_buf()),
        templates_dir: home.join("templates"),
        state_dir: home.join("state").join(&id),
    };

    let result = write_artifacts(&ctx)?;
    println!("state dir: {}", result.state_dir.display());
    println!("compose project: {}", result.manifest.compose_project);
    println!("regenerated: {}", result.regenerated);
    Ok(())
}
