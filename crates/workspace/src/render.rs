//! Rendering — Markdown → Markdown (identity), HTML, PDF, JSON.
//!
//! Used for finding exports, reports, and SOWs.

use serde_json::json;

/// Output format for exports and reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Html,
    Pdf,
    Json,
}

impl OutputFormat {
    pub fn parse_format(s: &str) -> Option<Self> {
        match s {
            "markdown" | "md" => Some(Self::Markdown),
            "html" => Some(Self::Html),
            "pdf" => Some(Self::Pdf),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Pdf => "pdf",
            Self::Json => "json",
        }
    }
}

/// Render Markdown content to the specified format.
pub fn render(markdown: &str, format: OutputFormat) -> Result<Vec<u8>, String> {
    match format {
        OutputFormat::Markdown => Ok(markdown.as_bytes().to_vec()),
        OutputFormat::Html => Ok(render_html(markdown).into_bytes()),
        OutputFormat::Pdf => render_pdf(markdown),
        OutputFormat::Json => Ok(render_json(markdown).into_bytes()),
    }
}

/// Render Markdown to HTML using pulldown-cmark.
fn render_html(markdown: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Wrap in a basic HTML document
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>SecuritySmith Report</title>\n<style>\nbody {{ max-width: 800px; margin: 2em auto; padding: 0 1em; font-family: sans-serif; line-height: 1.6; }}\ntable {{ border-collapse: collapse; width: 100%; }}\nth, td {{ border: 1px solid #ddd; padding: 8px; }}\nth {{ background: #f4f4f4; }}\ncode {{ background: #f4f4f4; padding: 2px 4px; }}\npre {{ background: #f4f4f4; padding: 1em; overflow-x: auto; }}\n</style>\n</head>\n<body>\n{}\n</body>\n</html>",
        html_output
    )
}

/// Render Markdown to PDF via embedded Typst compiler.
/// Uses a simple built-in template that renders the content as-is.
fn render_pdf(markdown: &str) -> Result<Vec<u8>, String> {
    let typst_markup = crate::typst_engine::markdown_to_typst(markdown)?;
    let template = "#let simple(doc) = {\n  set page(paper: \"a4\", margin: 2cm)\n  set text(font: \"DejaVu Sans\", 11pt)\n  doc\n}\n";
    let source = format!("{template}\n#show: simple\n\n{typst_markup}\n");
    crate::typst_engine::compile_source(&source)
}

/// Render Markdown to a simple JSON structure.
/// Extracts frontmatter (if present) and body.
fn render_json(markdown: &str) -> String {
    // Try to parse frontmatter
    let parsed = ss_frontmatter::parse(markdown).unwrap_or(ss_frontmatter::Parsed {
        frontmatter: serde_yaml::Value::Null,
        body: markdown.to_string(),
    });

    let frontmatter_json = if parsed.frontmatter.is_null() {
        serde_json::Value::Null
    } else {
        yaml_to_json(&parsed.frontmatter)
    };

    let json = json!({
        "frontmatter": frontmatter_json,
        "body": parsed.body,
    });

    serde_json::to_string_pretty(&json).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Convert a serde_yaml::Value to a serde_json::Value.
fn yaml_to_json(yaml: &serde_yaml::Value) -> serde_json::Value {
    match yaml {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    _ => serde_yaml::to_string(k)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                };
                obj.insert(key, yaml_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(_) => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_markdown_identity() {
        let md = "# Test\n\nHello.\n";
        let result = render(md, OutputFormat::Markdown).unwrap();
        assert_eq!(String::from_utf8(result).unwrap(), md);
    }

    #[test]
    fn render_html_basic() {
        let md = "# Test\n\nHello.\n";
        let result = render(md, OutputFormat::Html).unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<h1>Test</h1>"));
    }

    #[test]
    fn render_json_with_frontmatter() {
        let md = "---\nid: \"ACME-WEB-001\"\nstatus: \"open\"\n---\n\n# Test\n\nBody.\n";
        let result = render(md, OutputFormat::Json).unwrap();
        let json = String::from_utf8(result).unwrap();
        assert!(json.contains("ACME-WEB-001"));
        assert!(json.contains("open"));
        assert!(json.contains("Body."));
    }

    #[test]
    fn render_json_without_frontmatter() {
        let md = "# Test\n\nBody.\n";
        let result = render(md, OutputFormat::Json).unwrap();
        let json = String::from_utf8(result).unwrap();
        assert!(json.contains("Body."));
    }

    #[test]
    fn output_format_from_str() {
        assert_eq!(
            OutputFormat::parse_format("markdown"),
            Some(OutputFormat::Markdown)
        );
        assert_eq!(OutputFormat::parse_format("html"), Some(OutputFormat::Html));
        assert_eq!(OutputFormat::parse_format("pdf"), Some(OutputFormat::Pdf));
        assert_eq!(OutputFormat::parse_format("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse_format("invalid"), None);
    }
}
