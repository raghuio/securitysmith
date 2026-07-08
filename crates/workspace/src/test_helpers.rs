//! Test helpers for SecuritySmith integration tests.
//!
//! These utilities create temporary workspaces, clients, projects, and engagements
//! so tests are short and consistent.

use camino::Utf8PathBuf;
use std::fs;
use tempfile::TempDir;

/// A test workspace with a temp directory and config.toml.
pub struct TestWorkspace {
    pub _tmp: TempDir,
    pub root: Utf8PathBuf,
}

impl TestWorkspace {
    /// Create a new workspace in a temp directory.
    pub fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).unwrap();

        let config = "[workspace]\nversion = 1\nname = \"test\"\ncreated = \"2026-07-08\"\n";
        fs::write(root.join("config.toml"), config).unwrap();

        Self { _tmp: tmp, root }
    }

    /// Create a client directory with config.toml.
    pub fn create_client(&self, name: &str) -> Utf8PathBuf {
        let dir = self.root.join(name);
        fs::create_dir_all(&dir).unwrap();
        let config = format!(
            "[client]\nstatus = \"active\"\npriority = \"medium\"\ncreated = \"2026-07-08\"\nupdated = \"2026-07-08\"\n\n[client.id]\nprefix = \"TEST\"\n"
        );
        fs::write(dir.join("config.toml"), config).unwrap();
        dir
    }

    /// Create a project directory with config.toml under a client.
    pub fn create_project(&self, client: &str, project: &str) -> Utf8PathBuf {
        let dir = self.root.join(client).join(project);
        fs::create_dir_all(&dir).unwrap();
        let config = format!(
            "[project]\nabbreviation = \"WEB\"\nstatus = \"active\"\npriority = \"medium\"\n\n[project.id]\nsequence = 1\npadding = 3\n"
        );
        fs::write(dir.join("config.toml"), config).unwrap();
        dir
    }

    /// Create an engagement directory with config.toml under a project.
    pub fn create_engagement(&self, client: &str, project: &str, engagement: &str) -> Utf8PathBuf {
        let dir = self.root.join(client).join(project).join(engagement);
        fs::create_dir_all(&dir).unwrap();
        let config = "[engagement]\ntype = \"assessment\"\nstatus = \"in_progress\"\nstart_date = \"2026-07-01\"\nend_date = \"2026-07-14\"\n";
        fs::write(dir.join("config.toml"), config).unwrap();
        dir
    }

    /// Create a finding file under an engagement.
    pub fn create_finding(
        &self,
        engagement_dir: &Utf8PathBuf,
        id: &str,
        title: &str,
    ) -> Utf8PathBuf {
        let findings_dir = engagement_dir.join("findings");
        fs::create_dir_all(&findings_dir).unwrap();

        let slug: String = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let filename = format!("{}_{}.md", id.to_lowercase(), slug);
        let path = findings_dir.join(&filename);

        let content = format!(
            "---\nid: \"{}\"\nstatus: \"open\"\nseverity: \"high\"\ncreated: \"2026-07-02\"\nupdated: \"2026-07-02\"\n---\n\n# {}\n\nFinding content here.\n",
            id, title
        );
        fs::write(&path, content).unwrap();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_helper() {
        let ws = TestWorkspace::new();
        assert!(ws.root.join("config.toml").exists());
    }

    #[test]
    fn test_client_helper() {
        let ws = TestWorkspace::new();
        let client = ws.create_client("acme");
        assert!(client.join("config.toml").exists());
    }

    #[test]
    fn test_full_hierarchy() {
        let ws = TestWorkspace::new();
        ws.create_client("acme");
        ws.create_project("acme", "web_app");
        let eng = ws.create_engagement("acme", "web_app", "initial");
        assert!(eng.join("config.toml").exists());

        let finding = ws.create_finding(&eng, "TEST-WEB-001", "Stored XSS");
        assert!(finding.exists());
    }
}
