use super::{AffectedEndpoint, FindingParser, ParsedFinding, RemediationItem, normalize_severity};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub struct NessusParser;

impl FindingParser for NessusParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut findings = Vec::new();
        let mut current: Option<ParsedFinding> = None;
        let mut in_item = false;
        let mut text_buf = String::new();
        let mut host = String::new();
        let mut port = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(format!("XML parse error: {}", e)),
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) => {
                    let name = std::str::from_utf8(e.name().as_ref())
                        .unwrap_or("")
                        .to_string();
                    text_buf.clear();
                    if name == "ReportItem" {
                        in_item = true;
                        let mut title = String::new();
                        let mut sev = String::from("informational");
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| format!("Attr error: {}", e))?;
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = attr.unescape_value().unwrap_or_default().to_string();
                            match key {
                                "pluginName" => title = val,
                                "severity" => {
                                    sev = match val.as_str() {
                                        "4" => "critical".to_string(),
                                        "3" => "high".to_string(),
                                        "2" => "medium".to_string(),
                                        "1" => "low".to_string(),
                                        _ => "informational".to_string(),
                                    }
                                }
                                _ => {}
                            }
                        }
                        current = Some(ParsedFinding {
                            title,
                            severity: sev,
                            overview: String::new(),
                            summary: String::new(),
                            affected_endpoints: Vec::new(),
                            remediation_items: Vec::new(),
                            references: Vec::new(),
                            source_tool: "Nessus".to_string(),
                            source_id: None,
                            is_duplicate: false,
                            duplicate_of: None,
                        });
                        host.clear();
                        port.clear();
                    }
                }
                Ok(Event::Text(ref e)) if in_item => {
                    text_buf.push_str(&e.unescape().unwrap_or_default());
                }
                Ok(Event::End(ref e)) => {
                    let name = std::str::from_utf8(e.name().as_ref())
                        .unwrap_or("")
                        .to_string();
                    if let Some(ref mut c) = current {
                        match name.as_str() {
                            "plugin_output" if !text_buf.is_empty() => {
                                c.summary = text_buf.clone();
                                c.overview.clone_from(&text_buf);
                            }
                            "risk_factor" => {
                                c.severity = normalize_severity(&text_buf);
                            }
                            "solution" => {
                                c.remediation_items.push(RemediationItem {
                                    action: "Apply solution".to_string(),
                                    fix: text_buf.clone(),
                                    code_snippet: None,
                                });
                            }
                            "synopsis" => {
                                c.overview = text_buf.clone();
                            }
                            "description" => {
                                c.summary = text_buf.clone();
                            }
                            "plugin_information" => {
                                if let Some(parts) = text_buf.split_once(':') {
                                    c.source_id = Some(parts.1.trim().to_string());
                                }
                            }
                            "ReportItem" => {
                                if !host.is_empty() {
                                    let path = if !port.is_empty() {
                                        format!("{}:{}", host, port)
                                    } else {
                                        host.clone()
                                    };
                                    c.affected_endpoints.push(AffectedEndpoint {
                                        method: "GET".to_string(),
                                        path,
                                        description: "Nessus detected target".to_string(),
                                    });
                                }
                                findings.push(current.take().unwrap());
                                in_item = false;
                            }
                            _ => {}
                        }
                    }
                    if name == "HostProperties" || name == "tag" {
                        // ignore host property tags for simplicity
                    }
                    text_buf.clear();
                }
                _ => {}
            }
            buf.clear();
        }
        Ok(findings)
    }

    fn format_name(&self) -> &'static str {
        "nessus"
    }
}
