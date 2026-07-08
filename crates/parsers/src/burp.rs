use super::{AffectedEndpoint, FindingParser, ParsedFinding, RemediationItem, normalize_severity};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub struct BurpParser;

impl FindingParser for BurpParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut findings = Vec::new();
        let mut current: Option<ParsedFinding> = None;
        let mut in_issue = false;
        let mut text_buf = String::new();
        let mut tag_stack: Vec<String> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(format!("XML parse error: {e}")),
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) => {
                    let name = std::str::from_utf8(e.name().as_ref())
                        .unwrap_or("")
                        .to_string();
                    text_buf.clear();
                    tag_stack.push(name.clone());
                    if name == "issue" {
                        in_issue = true;
                        current = Some(ParsedFinding {
                            title: String::new(),
                            severity: "informational".to_string(),
                            overview: String::new(),
                            summary: String::new(),
                            affected_endpoints: Vec::new(),
                            remediation_items: Vec::new(),
                            references: Vec::new(),
                            source_tool: "Burp Suite".to_string(),
                            source_id: None,
                            is_duplicate: false,
                            duplicate_of: None,
                        });
                    }
                }
                Ok(Event::Text(ref e)) if in_issue => {
                    text_buf.push_str(&e.unescape().unwrap_or_default());
                }
                Ok(Event::End(ref e)) => {
                    let name = std::str::from_utf8(e.name().as_ref())
                        .unwrap_or("")
                        .to_string();
                    if let Some(ref mut c) = current {
                        match name.as_str() {
                            "name" => {
                                c.title.clone_from(&text_buf);
                            }
                            "severity" => c.severity = normalize_severity(&text_buf),
                            "host" => {
                                c.affected_endpoints.push(AffectedEndpoint {
                                    method: "GET".to_string(),
                                    path: text_buf.clone(),
                                    description: "Burp identified target host".to_string(),
                                });
                            }
                            "path" => {
                                if let Some(last) = c.affected_endpoints.last_mut() {
                                    last.path.push_str(&text_buf);
                                }
                            }
                            "issueBackground" => {
                                c.overview.clone_from(&text_buf);
                            }
                            "issueDetail" => {
                                c.summary.clone_from(&text_buf);
                            }
                            "remediationBackground" => {
                                c.remediation_items.push(RemediationItem {
                                    action: "Remediation".to_string(),
                                    fix: text_buf.clone(),
                                    code_snippet: None,
                                });
                            }
                            "issue" => {
                                findings.push(current.take().unwrap());
                                in_issue = false;
                            }
                            _ => {}
                        }
                    }
                    tag_stack.pop();
                    text_buf.clear();
                }
                _ => {}
            }
            buf.clear();
        }
        Ok(findings)
    }

    fn format_name(&self) -> &'static str {
        "burp"
    }
}
