use super::{AffectedEndpoint, FindingParser, ParsedFinding, RemediationItem};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub struct NmapParser;

impl FindingParser for NmapParser {
    fn parse(&self, content: &[u8]) -> Result<Vec<ParsedFinding>, String> {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut findings = Vec::new();
        let mut host = String::new();
        let mut port_id = String::new();
        let mut protocol = String::new();
        let mut service_name = String::new();
        let mut state = String::new();
        let mut current_tag = String::new();
        let mut in_port = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(format!("XML parse error: {}", e)),
                Ok(Event::Eof) => break,
                Ok(Event::Start(ref e)) => {
                    let tag = std::str::from_utf8(e.name().as_ref())
                        .unwrap_or("")
                        .to_string();
                    current_tag.clone_from(&tag);
                    if tag == "address" {
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| format!("Attr error: {}", e))?;
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = attr.unescape_value().unwrap_or_default().to_string();
                            if key == "addr" {
                                host = val;
                            }
                        }
                    } else if tag == "port" {
                        in_port = true;
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| format!("Attr error: {}", e))?;
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = attr.unescape_value().unwrap_or_default().to_string();
                            if key == "portid" {
                                port_id = val;
                            } else if key == "protocol" {
                                protocol = val;
                            }
                        }
                    } else if tag == "state" {
                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| format!("Attr error: {}", e))?;
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = attr.unescape_value().unwrap_or_default().to_string();
                            if key == "state" {
                                state = val;
                            }
                        }
                    }
                }
                Ok(Event::Text(ref e)) if in_port && current_tag == "service" => {
                    service_name.push_str(&e.unescape().unwrap_or_default());
                }
                Ok(Event::End(ref e)) => {
                    let tag = std::str::from_utf8(e.name().as_ref())
                        .unwrap_or("")
                        .to_string();
                    if tag == "port" {
                        in_port = false;
                        findings.push(ParsedFinding {
                            title: format!("Open port {}/{} on {}", port_id, protocol, host),
                            severity: "informational".to_string(),
                            overview: format!(
                                "Nmap detected {}/{} (state: {})",
                                protocol, port_id, state
                            ),
                            summary: format!(
                                "Port scan identified {} {} {} on host {}.",
                                protocol, port_id, state, host
                            ),
                            affected_endpoints: vec![AffectedEndpoint {
                                method: "N/A".to_string(),
                                path: format!("{}:{}/{}", host, port_id, protocol),
                                description: format!("{} port {}", protocol, port_id),
                            }],
                            remediation_items: vec![RemediationItem {
                                action: "Review open ports".to_string(),
                                fix: "Close or restrict unnecessary services.".to_string(),
                                code_snippet: None,
                            }],
                            references: Vec::new(),
                            source_tool: "Nmap".to_string(),
                            source_id: None,
                            is_duplicate: false,
                            duplicate_of: None,
                        });
                        port_id.clear();
                        protocol.clear();
                        service_name.clear();
                        state.clear();
                    }
                }
                _ => {}
            }
            buf.clear();
        }
        Ok(findings)
    }

    fn format_name(&self) -> &'static str {
        "nmap"
    }
}
