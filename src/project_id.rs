use sha2::{Digest, Sha256};

/// Stable 12-character hex id from the project root path.
pub fn project_id(project_root: &str) -> String {
    let hash = Sha256::digest(project_root.as_bytes());
    hex::encode(&hash[..6])
}

/// Docker Compose project name for this workspace.
pub fn compose_project(project_root: &str) -> String {
    format!("claudep-{}", project_id(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_id_is_stable() {
        let root = "/Users/me/my-project";
        assert_eq!(project_id(root), project_id(root));
        assert_eq!(project_id(root).len(), 12);
    }

    #[test]
    fn compose_project_prefix() {
        let root = "/tmp/foo";
        assert!(compose_project(root).starts_with("claudep-"));
    }
}
