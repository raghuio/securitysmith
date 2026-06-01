use super::{AffectedEndpoint, FindingParser, ParsedFinding, RemediationItem};

pub struct NucleiParser;

impl FindingParser for NucleiParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String> {
        let text = std::str::from_utf8(content).map_err(|e| format!("Invalid UTF-8: {}", e))?;
        let mut findings = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(line).map_err(|e| format!("Invalid JSON line: {}", e))?;
            let info = value
                .get("info")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();
            let name = info
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Nuclei finding")
                .to_string();
            let severity = info
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("info")
                .to_string();
            let description = info
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let remediation = info
                .get("remediation")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let host = value
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let matched = value
                .get("matched-at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let template = value
                .get("template-id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let sev = match severity.to_lowercase().as_str() {
                "critical" => "critical",
                "high" => "high",
                "medium" => "medium",
                "low" => "low",
                _ => "informational",
            }
            .to_string();

            findings.push(ParsedFinding {
                title: name.chars().take(500).collect(),
                severity: sev,
                overview: description.clone(),
                summary: format!("{} Matched at: {}", template, matched),
                affected_endpoints: vec![AffectedEndpoint {
                    method: "GET".to_string(),
                    path: if matched.is_empty() {
                        host.clone()
                    } else {
                        matched
                    },
                    description: host,
                }],
                remediation_items: if remediation.is_empty() {
                    Vec::new()
                } else {
                    vec![RemediationItem {
                        action: "Fix vulnerability".to_string(),
                        fix: remediation,
                        code_snippet: None,
                    }]
                },
                references: Vec::new(),
                source_tool: "Nuclei".to_string(),
                source_id: Some(template),
                is_duplicate: false,
                duplicate_of: None,
            });
        }
        Ok(findings)
    }

    fn format_name(&self) -> &'static str {
        "nuclei"
    }
}
