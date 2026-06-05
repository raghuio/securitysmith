use super::{AffectedEndpoint, CsvColumnMapping, FindingParser, ParsedFinding, RemediationItem};

pub struct CsvParser {
    mapping: CsvColumnMapping,
}

impl CsvParser {
    pub fn new(mapping: CsvColumnMapping) -> Self {
        Self { mapping }
    }

    pub fn mapping(&self) -> &CsvColumnMapping {
        &self.mapping
    }
}

impl FindingParser for CsvParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String> {
        let mut reader = csv::Reader::from_reader(content);
        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| format!("CSV header error: {e}"))?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let idx_max = [Some(self.mapping.title), Some(self.mapping.severity)]
            .into_iter()
            .chain([
                self.mapping.description,
                self.mapping.remediation,
                self.mapping.affected_url,
            ])
            .flatten()
            .max()
            .unwrap_or(0);

        if idx_max >= headers.len() {
            return Err(format!(
                "Column index {} exceeds header count {}.",
                idx_max,
                headers.len()
            ));
        }

        let mut findings = Vec::new();
        for (i, result) in reader.records().enumerate() {
            let record = result.map_err(|e| format!("CSV row {} parse error: {}", i + 1, e))?;
            let title = record
                .get(self.mapping.title)
                .unwrap_or("")
                .trim()
                .to_string();
            let severity = record
                .get(self.mapping.severity)
                .unwrap_or("informational")
                .trim()
                .to_string();
            if title.is_empty() {
                continue;
            }
            let description = self
                .mapping
                .description
                .and_then(|idx| record.get(idx))
                .map(|s| s.trim().to_string());
            let remediation = self
                .mapping
                .remediation
                .and_then(|idx| record.get(idx))
                .map(|s| s.trim().to_string());
            let affected_url = self
                .mapping
                .affected_url
                .and_then(|idx| record.get(idx))
                .map(|s| s.trim().to_string());

            let endpoints = if let Some(ref url) = affected_url {
                vec![AffectedEndpoint {
                    method: "GET".to_string(),
                    path: url.clone(),
                    description: "Identified from CSV import".to_string(),
                }]
            } else {
                Vec::new()
            };

            let remediation_items = if let Some(ref fix) = remediation {
                vec![RemediationItem {
                    action: "Apply fix".to_string(),
                    fix: fix.clone(),
                    code_snippet: None,
                }]
            } else {
                Vec::new()
            };

            findings.push(ParsedFinding {
                title,
                severity: severity.to_lowercase(),
                overview: description.clone().unwrap_or_default(),
                summary: description.unwrap_or_default(),
                affected_endpoints: endpoints,
                remediation_items,
                references: Vec::new(),
                source_tool: "CSV".to_string(),
                source_id: None,
                is_duplicate: false,
                duplicate_of: None,
            });
        }
        Ok(findings)
    }

    fn format_name(&self) -> &'static str {
        "csv"
    }
}
