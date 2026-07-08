#![allow(
    unused_mut,
    clippy::must_use_candidate,
    clippy::double_must_use,
    clippy::too_many_arguments
)]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ParsedFinding {
    pub title: String,
    pub severity: String,
    pub overview: String,
    pub summary: String,
    pub affected_endpoints: Vec<AffectedEndpoint>,
    pub remediation_items: Vec<RemediationItem>,
    pub references: Vec<Reference>,
    pub source_tool: String,
    pub source_id: Option<String>,
    pub is_duplicate: bool,
    pub duplicate_of: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AffectedEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemediationItem {
    pub action: String,
    pub fix: String,
    pub code_snippet: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Reference {
    pub title: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImportPreview {
    pub findings: Vec<ParsedFinding>,
    pub total_parsed: u32,
    pub duplicates_found: u32,
    pub format: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImportResult {
    pub imported_count: u32,
    pub skipped_count: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CsvColumnMapping {
    pub title: usize,
    pub severity: usize,
    pub description: Option<usize>,
    pub remediation: Option<usize>,
    pub affected_url: Option<usize>,
}

pub trait FindingParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String>;
    fn format_name(&self) -> &'static str;
}

pub mod burp;
pub mod csv_import;
pub mod nessus;
pub mod nmap;
pub mod nuclei;
pub mod zap;

pub fn normalize_severity(raw: &str) -> String {
    match raw.to_lowercase().trim() {
        "critical" | "severe" | "high (critical)" => "critical".to_string(),
        "high" | "serious" => "high".to_string(),
        "medium" | "moderate" | "2" => "medium".to_string(),
        "low" | "minor" | "1" => "low".to_string(),
        "informational" | "info" | "information" | "none" | "0" => "informational".to_string(),
        _ => "informational".to_string(),
    }
}
