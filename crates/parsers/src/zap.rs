use super::{AffectedEndpoint, FindingParser, ParsedFinding, RemediationItem};
use serde::Deserialize;

#[derive(Deserialize)]
struct ZapJson {
    site: Vec<ZapSite>,
}

#[derive(Deserialize)]
struct ZapSite {
    alerts: Vec<ZapAlert>,
}

#[derive(Deserialize)]
struct ZapAlert {
    name: String,
    riskdesc: String,
    desc: String,
    solution: String,
    instances: Vec<ZapInstance>,
}

#[derive(Deserialize)]
struct ZapInstance {
    uri: String,
    method: String,
}

pub struct ZapJsonParser;

impl FindingParser for ZapJsonParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String> {
        let zap: ZapJson = serde_json::from_slice(content)
            .map_err(|e| format!("Failed to parse ZAP JSON: {e}"))?;
        let mut findings = Vec::new();
        for site in zap.site {
            for alert in site.alerts {
                let severity = alert.riskdesc.split(' ').next().unwrap_or("Informational");
                let sev = match severity.to_lowercase().as_str() {
                    "high" => "high",
                    "medium" => "medium",
                    "low" => "low",
                    _ => "informational",
                }
                .to_string();
                let mut endpoints: Vec<AffectedEndpoint> = alert
                    .instances
                    .into_iter()
                    .map(|i| AffectedEndpoint {
                        method: i.method.to_uppercase(),
                        path: i.uri.chars().take(500).collect(),
                        description: "ZAP identified instance".to_string(),
                    })
                    .collect();
                if endpoints.is_empty() {
                    endpoints.push(AffectedEndpoint {
                        method: "GET".to_string(),
                        path: "/".to_string(),
                        description: "ZAP alert".to_string(),
                    });
                }
                findings.push(ParsedFinding {
                    title: alert.name.chars().take(500).collect(),
                    severity: sev,
                    overview: alert.desc.clone(),
                    summary: alert.solution.clone(),
                    affected_endpoints: endpoints,
                    remediation_items: vec![RemediationItem {
                        action: "Apply fix".to_string(),
                        fix: alert.solution,
                        code_snippet: None,
                    }],
                    references: Vec::new(),
                    source_tool: "OWASP ZAP".to_string(),
                    source_id: None,
                    is_duplicate: false,
                    duplicate_of: None,
                });
            }
        }
        Ok(findings)
    }

    fn format_name(&self) -> &'static str {
        "zap_json"
    }
}
