//! YAML frontmatter parser and writer for Markdown files.
//!
//! Frontmatter is YAML between `---` markers at the start of a Markdown file.
//! This crate parses it into a `serde_yaml::Value` and writes it back.
//!
//! Atomic writes: writes to a temp file first, then renames.

use std::fs;
use std::io::Write;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrontmatterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("No frontmatter found in {0}")]
    NotFound(String),
    #[error("File does not start with frontmatter delimiters")]
    BadDelim,
}

/// Parsed frontmatter and body from a Markdown file.
#[derive(Debug, Clone)]
pub struct Parsed {
    /// The YAML frontmatter as a serde_yaml Value. Empty if no frontmatter.
    pub frontmatter: serde_yaml::Value,
    /// The Markdown body after frontmatter (everything after the closing `---`).
    pub body: String,
}

impl Parsed {
    /// Check if this file has frontmatter.
    pub fn has_frontmatter(&self) -> bool {
        !self.frontmatter.is_null()
    }
}

/// Parse frontmatter from a Markdown string.
///
/// A file with frontmatter looks like:
/// ```text
/// ---
/// key: value
/// ---
/// Body text here.
/// ```
///
/// A file without frontmatter is just body text.
pub fn parse(content: &str) -> Result<Parsed, FrontmatterError> {
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        // No frontmatter — entire content is body
        return Ok(Parsed {
            frontmatter: serde_yaml::Value::Null,
            body: content.to_string(),
        });
    }

    // Skip the opening ---
    let after_open = &trimmed[3..]; // skip "---"
    let after_open = after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))
        .unwrap_or(after_open);

    // Find the closing --- at the start of a line
    // Handle empty frontmatter (--- immediately after opening ---)
    let (yaml_str, after_close) = if after_open.starts_with("---\n")
        || after_open.starts_with("---\r\n")
        || after_open == "---"
    {
        // Empty frontmatter
        let rest = after_open.strip_prefix("---").unwrap_or(after_open);
        let rest = rest
            .strip_prefix('\n')
            .or_else(|| rest.strip_prefix("\r\n"))
            .unwrap_or(rest);
        ("", rest)
    } else {
        let close_pos = after_open
            .find("\n---")
            .ok_or(FrontmatterError::NotFound(content.to_string()))?;
        let yaml = &after_open[..close_pos];
        let rest = &after_open[close_pos + 4..]; // skip "\n---"
        let rest = rest
            .strip_prefix('\n')
            .or_else(|| rest.strip_prefix("\r\n"))
            .unwrap_or(rest);
        (yaml, rest)
    };

    let body = after_close.to_string();

    let frontmatter: serde_yaml::Value = if yaml_str.trim().is_empty() {
        serde_yaml::Value::Null
    } else {
        serde_yaml::from_str(yaml_str)?
    };

    Ok(Parsed { frontmatter, body })
}

/// Parse frontmatter from a file on disk.
pub fn parse_file(path: &Path) -> Result<Parsed, FrontmatterError> {
    let content = fs::read_to_string(path)?;
    parse(&content)
}

/// Serialize frontmatter and body into a Markdown string.
pub fn serialize(frontmatter: &serde_yaml::Value, body: &str) -> Result<String, FrontmatterError> {
    if frontmatter.is_null() {
        return Ok(body.to_string());
    }

    let yaml = serde_yaml::to_string(frontmatter)?;
    // serde_yaml adds a trailing newline; trim it for cleaner output
    let yaml = yaml.trim_end();

    Ok(format!("---\n{}\n---\n\n{}", yaml, body))
}

/// Write frontmatter and body to a file atomically.
///
/// Writes to a temp file first, then renames to the target path.
/// This prevents corruption if the process is interrupted mid-write.
pub fn write_file(
    path: &Path,
    frontmatter: &serde_yaml::Value,
    body: &str,
) -> Result<(), FrontmatterError> {
    let content = serialize(frontmatter, body)?;

    // Atomic write: write to temp file, then rename
    let tmp_path = path.with_extension("md.tmp");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Update a single field in the frontmatter of a file.
///
/// Reads the file, updates the field, writes it back atomically.
pub fn update_field(
    path: &Path,
    key: &str,
    value: &serde_yaml::Value,
) -> Result<(), FrontmatterError> {
    let mut parsed = parse_file(path)?;

    let mut frontmatter = if parsed.has_frontmatter() {
        match parsed.frontmatter {
            serde_yaml::Value::Mapping(ref mut m) => {
                let mut m = m.clone();
                m.insert(serde_yaml::Value::String(key.to_string()), value.clone());
                serde_yaml::Value::Mapping(m)
            }
            _ => {
                let mut m = serde_yaml::Mapping::new();
                m.insert(serde_yaml::Value::String(key.to_string()), value.clone());
                serde_yaml::Value::Mapping(m)
            }
        }
    } else {
        let mut m = serde_yaml::Mapping::new();
        m.insert(serde_yaml::Value::String(key.to_string()), value.clone());
        serde_yaml::Value::Mapping(m)
    };

    // Also update the `updated` field
    if let serde_yaml::Value::Mapping(ref mut m) = frontmatter {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        m.insert(
            serde_yaml::Value::String("updated".to_string()),
            serde_yaml::Value::String(today),
        );
    }

    write_file(path, &frontmatter, &parsed.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;
    use std::fs;

    #[test]
    fn parse_with_frontmatter() {
        let content = "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\n---\n\n# Title\n\nBody text.\n";
        let parsed = parse(content).unwrap();
        assert!(parsed.has_frontmatter());

        let id = parsed.frontmatter.get("id").unwrap();
        assert_eq!(id, &Value::String("ACME-WEB-001".to_string()));

        assert!(parsed.body.contains("# Title"));
        assert!(parsed.body.contains("Body text."));
    }

    #[test]
    fn parse_without_frontmatter() {
        let content = "# Just a title\n\nBody only.\n";
        let parsed = parse(content).unwrap();
        assert!(!parsed.has_frontmatter());
        assert!(parsed.body.contains("Body only."));
    }

    #[test]
    fn parse_empty_frontmatter() {
        let content = "---\n---\n\nBody.\n";
        let parsed = parse(content).unwrap();
        assert!(!parsed.has_frontmatter());
        assert!(parsed.body.contains("Body."));
    }

    #[test]
    fn round_trip() {
        let mut fm = serde_yaml::Mapping::new();
        fm.insert(
            Value::String("id".to_string()),
            Value::String("TEST-001".to_string()),
        );
        fm.insert(
            Value::String("status".to_string()),
            Value::String("open".to_string()),
        );
        let frontmatter = Value::Mapping(fm);
        let body = "# Test Finding\n\nContent here.\n";

        let serialized = serialize(&frontmatter, body).unwrap();
        let parsed = parse(&serialized).unwrap();

        assert_eq!(
            parsed.frontmatter.get("id"),
            Some(&Value::String("TEST-001".to_string()))
        );
        assert_eq!(
            parsed.frontmatter.get("status"),
            Some(&Value::String("open".to_string()))
        );
        assert!(parsed.body.contains("# Test Finding"));
    }

    #[test]
    fn serialize_null_frontmatter() {
        let body = "# Just body\n\nNo frontmatter.\n";
        let result = serialize(&Value::Null, body).unwrap();
        assert!(!result.starts_with("---"));
        assert!(result.contains("Just body"));
    }

    #[test]
    fn write_and_read_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.md");

        let mut fm = serde_yaml::Mapping::new();
        fm.insert(
            Value::String("id".to_string()),
            Value::String("F-001".to_string()),
        );
        write_file(&path, &Value::Mapping(fm), "# Title\n\nBody.\n").unwrap();

        let parsed = parse_file(&path).unwrap();
        assert_eq!(
            parsed.frontmatter.get("id"),
            Some(&Value::String("F-001".to_string()))
        );
    }

    #[test]
    fn update_field_in_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.md");

        let mut fm = serde_yaml::Mapping::new();
        fm.insert(
            Value::String("id".to_string()),
            Value::String("F-001".to_string()),
        );
        fm.insert(
            Value::String("status".to_string()),
            Value::String("open".to_string()),
        );
        write_file(&path, &Value::Mapping(fm), "# Title\n\nBody.\n").unwrap();

        update_field(&path, "status", &Value::String("fixed".to_string())).unwrap();

        let parsed = parse_file(&path).unwrap();
        assert_eq!(
            parsed.frontmatter.get("status"),
            Some(&Value::String("fixed".to_string()))
        );
        // updated field should be present
        assert!(parsed.frontmatter.get("updated").is_some());
    }

    #[test]
    fn no_closing_delimiter() {
        let content = "---\nid: test\nbody without closing\n";
        let result = parse(content);
        assert!(result.is_err());
    }
}
