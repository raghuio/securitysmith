//! Evidence management — files stored under `evidence/` in engagement directories.
//!
//! SHA-256 hash computed on add. TOML index tracks metadata. Secret detection
//! scans text evidence files for common credential patterns.

use camino::Utf8Path;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;

use crate::WorkspaceError;

/// Evidence index entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub filename: String,
    pub original_path: String,
    pub sha256: String,
    pub size: u64,
    pub date_added: String,
}

/// Evidence index file (TOML).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceIndex {
    #[serde(default, rename = "evidence")]
    pub entries: Vec<EvidenceEntry>,
}

const INDEX_FILE: &str = "evidence-index.toml";
const MAX_FILENAME_LEN: usize = 255;

/// Sanitize a filename: only [a-zA-Z0-9._-] allowed, max 255 chars.
fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.len() > MAX_FILENAME_LEN {
        sanitized[..MAX_FILENAME_LEN].to_string()
    } else {
        sanitized
    }
}

/// Resolve a unique filename in the evidence directory (add suffix for duplicates).
fn unique_filename(evidence_dir: &Utf8Path, filename: &str) -> String {
    let sanitized = sanitize_filename(filename);
    if !evidence_dir.join(&sanitized).exists() {
        return sanitized;
    }

    // Add numeric suffix
    let (stem, ext) = match sanitized.rsplit_once('.') {
        Some((s, e)) => (s, format!(".{e}")),
        None => (sanitized.as_str(), String::new()),
    };

    for i in 1..10000 {
        let candidate = format!("{stem}_{i}{ext}");
        if !evidence_dir.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{stem}_dup{ext}")
}

/// Compute SHA-256 hash of a file.
fn compute_hash(path: &Utf8Path) -> Result<String, WorkspaceError> {
    let mut file = fs::File::open(path.as_std_path())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    Ok(hash.iter().map(|b| format!("{b:02x}")).collect())
}

/// Add an evidence file to an engagement.
///
/// Copies the file to `evidence/`, computes SHA-256, updates `evidence-index.toml`.
/// Returns the sanitized filename used.
pub fn add_evidence(
    workspace_root: &Utf8Path,
    engagement_path: &Utf8Path,
    source_file: &Utf8Path,
) -> Result<String, WorkspaceError> {
    if !source_file.exists() {
        return Err(WorkspaceError::NotFound(source_file.to_path_buf()));
    }

    // Symlink escape check on source file (must be within home or workspace)
    if let Some(home) = dirs::home_dir() {
        let home_canonical = fs::canonicalize(&home)?;
        let source_canonical = fs::canonicalize(source_file.as_std_path())?;
        let ws_canonical = fs::canonicalize(workspace_root.as_std_path())?;
        if !source_canonical.starts_with(&home_canonical)
            && !source_canonical.starts_with(&ws_canonical)
        {
            return Err(WorkspaceError::SymlinkEscape(source_file.to_path_buf()));
        }
    }

    let evidence_dir = engagement_path.join("evidence");
    fs::create_dir_all(evidence_dir.as_std_path())?;

    let original_name = source_file
        .file_name()
        .unwrap_or("evidence_file")
        .to_string();
    let filename = unique_filename(&evidence_dir, &original_name);
    let dest = evidence_dir.join(&filename);

    // Copy file
    fs::copy(source_file.as_std_path(), dest.as_std_path())?;

    // Compute hash and size
    let sha256 = compute_hash(&dest)?;
    let size = fs::metadata(dest.as_std_path())?.len();
    let date_added = Utc::now().format("%Y-%m-%d").to_string();

    // Update index
    let index_path = engagement_path.join(INDEX_FILE);
    let mut index = load_index(&index_path)?;
    index.entries.push(EvidenceEntry {
        filename: filename.clone(),
        original_path: source_file.to_string(),
        sha256,
        size,
        date_added,
    });
    save_index(&index_path, &index)?;

    Ok(filename)
}

/// List evidence files for an engagement.
pub fn list_evidence(engagement_path: &Utf8Path) -> Result<Vec<EvidenceEntry>, WorkspaceError> {
    let index_path = engagement_path.join(INDEX_FILE);
    if !index_path.exists() {
        return Ok(Vec::new());
    }
    let index = load_index(&index_path)?;
    Ok(index.entries)
}

/// Remove an evidence file and its index entry.
pub fn remove_evidence(
    workspace_root: &Utf8Path,
    engagement_path: &Utf8Path,
    filename: &str,
) -> Result<(), WorkspaceError> {
    let evidence_dir = engagement_path.join("evidence");
    let file_path = evidence_dir.join(filename);

    // Symlink check
    crate::check_symlink_escape(workspace_root, &file_path)?;

    if file_path.exists() {
        trash::delete(file_path.as_std_path())
            .map_err(|e| WorkspaceError::Io(std::io::Error::other(format!("Trash error: {e}"))))?;
    }

    // Remove from index
    let index_path = engagement_path.join(INDEX_FILE);
    let mut index = load_index(&index_path)?;
    index.entries.retain(|e| e.filename != filename);
    save_index(&index_path, &index)?;

    Ok(())
}

/// Verify evidence hashes. Returns list of mismatches.
pub fn verify_hashes(engagement_path: &Utf8Path) -> Result<Vec<String>, WorkspaceError> {
    let index_path = engagement_path.join(INDEX_FILE);
    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let index = load_index(&index_path)?;
    let evidence_dir = engagement_path.join("evidence");
    let mut mismatches = Vec::new();

    for entry in &index.entries {
        let file_path = evidence_dir.join(&entry.filename);
        if !file_path.exists() {
            mismatches.push(format!("Missing: {}", entry.filename));
            continue;
        }
        let current_hash = compute_hash(&file_path)?;
        if current_hash != entry.sha256 {
            mismatches.push(format!("Hash mismatch: {}", entry.filename));
        }
    }

    Ok(mismatches)
}

/// Scan text evidence files for common secret patterns.
/// Returns list of warnings (filename: pattern found).
pub fn scan_for_secrets(engagement_path: &Utf8Path) -> Result<Vec<String>, WorkspaceError> {
    let index_path = engagement_path.join(INDEX_FILE);
    if !index_path.exists() {
        return Ok(Vec::new());
    }

    let index = load_index(&index_path)?;
    let evidence_dir = engagement_path.join("evidence");
    let mut warnings = Vec::new();

    let patterns: &[&str] = &[
        "password=",
        "passwd=",
        "api_key=",
        "apikey=",
        "secret=",
        "token=",
        "AKIA",       // AWS access key
        "-----BEGIN", // Private key
        "private_key",
        "access_key=",
    ];

    for entry in &index.entries {
        let file_path = evidence_dir.join(&entry.filename);
        if !file_path.exists() {
            continue;
        }

        // Only scan text files (check extension)
        let ext = file_path.extension().unwrap_or("");
        let is_text = matches!(
            ext,
            "txt"
                | "log"
                | "md"
                | "json"
                | "xml"
                | "csv"
                | "yaml"
                | "yml"
                | "conf"
                | "cfg"
                | "ini"
                | "env"
                | ""
        );
        if !is_text {
            continue;
        }

        // Read file content (limit to 1MB to avoid reading huge files)
        let content = match fs::read_to_string(file_path.as_std_path()) {
            Ok(c) if c.len() < 1_000_000 => c,
            _ => continue,
        };

        for pattern in patterns {
            if content.to_lowercase().contains(&pattern.to_lowercase()) {
                warnings.push(format!(
                    "{}: potential secret pattern '{}' found",
                    entry.filename, pattern
                ));
                break;
            }
        }
    }

    Ok(warnings)
}

/// Load the evidence index from a TOML file.
fn load_index(path: &Utf8Path) -> Result<EvidenceIndex, WorkspaceError> {
    if !path.exists() {
        return Ok(EvidenceIndex::default());
    }
    let content = fs::read_to_string(path.as_std_path())?;
    let index: EvidenceIndex = toml::from_str(&content)?;
    Ok(index)
}

/// Save the evidence index to a TOML file (atomic write).
fn save_index(path: &Utf8Path, index: &EvidenceIndex) -> Result<(), WorkspaceError> {
    let toml = toml::to_string_pretty(index)?;
    crate::atomic_write(path, toml.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn sanitize_filename_basic() {
        assert_eq!(sanitize_filename("screenshot.png"), "screenshot.png");
        assert_eq!(
            sanitize_filename("file with spaces.txt"),
            "file_with_spaces.txt"
        );
        assert_eq!(sanitize_filename("../../etc/passwd"), ".._.._etc_passwd");
    }

    #[test]
    fn add_evidence_copies_and_hashes() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");

        // Create a source file
        let src = tw.root.join("test_evidence.txt");
        fs::write(&src, "evidence content").unwrap();

        let filename = add_evidence(&tw.root, &eng, &src).unwrap();
        assert!(eng.join("evidence").join(&filename).exists());
        assert!(eng.join(INDEX_FILE).exists());

        let entries = list_evidence(&eng).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, filename);
        assert!(!entries[0].sha256.is_empty());
        assert_eq!(entries[0].size, 16);
    }

    #[test]
    fn duplicate_filename_gets_suffix() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");

        let src = tw.root.join("test.txt");
        fs::write(&src, "content1").unwrap();
        let f1 = add_evidence(&tw.root, &eng, &src).unwrap();
        let f2 = add_evidence(&tw.root, &eng, &src).unwrap();
        assert_ne!(f1, f2);
        assert!(f2.starts_with("test_1"));
    }

    #[test]
    fn remove_evidence_works() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");

        let src = tw.root.join("test.txt");
        fs::write(&src, "content").unwrap();
        let filename = add_evidence(&tw.root, &eng, &src).unwrap();

        remove_evidence(&tw.root, &eng, &filename).unwrap();
        let entries = list_evidence(&eng).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn verify_hashes_detects_mismatch() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");

        let src = tw.root.join("test.txt");
        fs::write(&src, "original").unwrap();
        let filename = add_evidence(&tw.root, &eng, &src).unwrap();

        // Modify the evidence file
        fs::write(eng.join("evidence").join(&filename), "modified").unwrap();

        let mismatches = verify_hashes(&eng).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("Hash mismatch"));
    }

    #[test]
    fn secret_scan_finds_passwords() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");

        let src = tw.root.join("config.txt");
        fs::write(&src, "password=secret123\napi_key=abc123").unwrap();
        add_evidence(&tw.root, &eng, &src).unwrap();

        let warnings = scan_for_secrets(&eng).unwrap();
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("potential secret"));
    }

    #[test]
    fn secret_scan_ignores_binary_files() {
        let tw = TestWorkspace::new();
        tw.create_client("acme");
        tw.create_project("acme", "web");
        let eng = tw.create_engagement("acme", "web", "initial");

        let src = tw.root.join("image.png");
        fs::write(&src, "password=secret but in a .png file").unwrap();
        add_evidence(&tw.root, &eng, &src).unwrap();

        let warnings = scan_for_secrets(&eng).unwrap();
        assert!(warnings.is_empty());
    }
}
