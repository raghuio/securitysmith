//! Typst document engine — Markdown→Typst conversion and PDF compilation.
//!
//! Embeds the Typst compiler with bundled fonts. Converts Markdown content
//! to Typst markup, combines it with a Typst template and structured data,
//! and compiles to PDF bytes.

use pulldown_cmark::{Event, Tag, TagEnd};
use std::collections::BTreeMap;

/// Maximum nesting depth for Markdown→Typst conversion.
const MAX_NESTING_DEPTH: usize = 50;

/// Input data for a Typst template.
#[derive(Debug, Clone, Default)]
pub struct TypstInputs {
    /// Findings: each has id, status, severity, body (Typst markup).
    pub findings: Vec<FindingInput>,
    /// Requirements: each has id, status, body (Typst markup).
    pub requirements: Vec<RequirementInput>,
    /// Scope content as Typst markup (None if no scope file).
    pub scope: Option<String>,
    /// Document sections: name → Typst markup.
    pub sections: BTreeMap<String, String>,
    /// Metadata from config.toml: key → value.
    pub metadata: BTreeMap<String, String>,
    /// Notes: each has id, body (Typst markup).
    pub notes: Vec<NoteInput>,
}

#[derive(Debug, Clone)]
pub struct FindingInput {
    pub id: String,
    pub status: String,
    pub severity: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct RequirementInput {
    pub id: String,
    pub status: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct NoteInput {
    pub id: String,
    pub body: String,
}

/// Convert Markdown to Typst markup.
pub fn markdown_to_typst(markdown: &str) -> Result<String, String> {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);

    let parser = pulldown_cmark::Parser::new_ext(markdown, options);
    let mut converter = Converter::new();
    converter.convert(parser)
}

struct Converter {
    output: String,
    list_stack: Vec<ListType>,
    nesting: usize,
    table_rows: Vec<Vec<String>>,
    table_aligns: Vec<pulldown_cmark::Alignment>,
    in_table: bool,
    current_cell: String,
    in_cell: bool,
}

enum ListType {
    Unordered,
    Ordered,
}

impl Converter {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_stack: Vec::new(),
            nesting: 0,
            table_rows: Vec::new(),
            table_aligns: Vec::new(),
            in_table: false,
            current_cell: String::new(),
            in_cell: false,
        }
    }

    fn convert<'a, I>(&mut self, parser: I) -> Result<String, String>
    where
        I: Iterator<Item = Event<'a>>,
    {
        for event in parser {
            match event {
                Event::Start(tag) => self.handle_start(tag)?,
                Event::End(tag) => self.handle_end(tag),
                Event::Text(text) => {
                    if self.in_cell {
                        self.current_cell.push_str(&escape_markup(&text));
                    } else {
                        self.output.push_str(&escape_markup(&text));
                    }
                }
                Event::Code(code) => {
                    let raw = format!("#raw(\"{}\")", escape_typst_string(&code));
                    if self.in_cell {
                        self.current_cell.push_str(&raw);
                    } else {
                        self.output.push_str(&raw);
                    }
                }
                Event::SoftBreak => {
                    if self.in_cell {
                        self.current_cell.push(' ');
                    } else {
                        self.output.push(' ');
                    }
                }
                Event::HardBreak => {
                    let br = " #linebreak() ".to_string();
                    if self.in_cell {
                        self.current_cell.push_str(&br);
                    } else {
                        self.output.push_str(&br);
                    }
                }
                Event::InlineHtml(html) => {
                    if html.trim().eq_ignore_ascii_case("<u>") {
                        self.output.push_str("#underline[");
                    } else if html.trim().eq_ignore_ascii_case("</u>") {
                        self.output.push(']');
                    }
                }
                _ => {}
            }
        }
        Ok(std::mem::take(&mut self.output))
    }

    fn handle_start(&mut self, tag: Tag) -> Result<(), String> {
        match tag {
            Tag::Heading { level, .. } => {
                let count = level as usize;
                self.output.push('\n');
                for _ in 0..count {
                    self.output.push('=');
                }
                self.output.push(' ');
            }
            Tag::Paragraph => {
                if !self.output.is_empty() && !self.output.ends_with('\n') {
                    self.output.push_str("\n\n");
                }
            }
            Tag::Strong => self.output.push_str("#strong["),
            Tag::Emphasis => self.output.push_str("#emph["),
            Tag::Strikethrough => self.output.push_str("#strike["),
            Tag::Link { dest_url, .. } => {
                self.output
                    .push_str(&format!("#link(\"{}\")[", escape_typst_string(&dest_url)));
            }
            Tag::Image { dest_url, .. } => {
                self.output
                    .push_str(&format!("#image(\"{}\")", escape_typst_string(&dest_url)));
            }
            Tag::BlockQuote(_) => {
                self.nesting += 1;
                if self.nesting > MAX_NESTING_DEPTH {
                    return Err(format!(
                        "Markdown nesting too deep (>{MAX_NESTING_DEPTH} levels). Simplify the content."
                    ));
                }
                self.output.push_str("#quote[\n");
            }
            Tag::CodeBlock(kind) => {
                use pulldown_cmark::CodeBlockKind;
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.output.push_str(&format!(
                    "#raw(block: true, lang: \"{}\", \"",
                    escape_typst_string(&lang)
                ));
            }
            Tag::List(ordered) => {
                self.nesting += 1;
                if self.nesting > MAX_NESTING_DEPTH {
                    return Err(format!(
                        "Markdown nesting too deep (>{MAX_NESTING_DEPTH} levels). Simplify the content."
                    ));
                }
                if ordered.is_some() {
                    self.list_stack.push(ListType::Ordered);
                } else {
                    self.list_stack.push(ListType::Unordered);
                }
            }
            Tag::Item => {
                let marker = match self.list_stack.last() {
                    Some(ListType::Ordered) => "+ ",
                    Some(ListType::Unordered) | None => "- ",
                };
                self.output.push('\n');
                self.output.push_str(marker);
            }
            Tag::Table(aligns) => {
                self.in_table = true;
                self.table_rows.clear();
                self.table_aligns = aligns;
            }
            Tag::TableHead => {}
            Tag::TableRow => {
                self.table_rows.push(Vec::new());
            }
            Tag::TableCell => {
                self.in_cell = true;
                self.current_cell.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.output.push('\n');
            }
            TagEnd::Paragraph => {
                self.output.push_str("\n\n");
            }
            TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough | TagEnd::Link => {
                self.output.push(']');
            }
            TagEnd::BlockQuote(_) => {
                self.nesting = self.nesting.saturating_sub(1);
                self.output.push_str("\n]\n");
            }
            TagEnd::CodeBlock => {
                self.output.push_str("\")\n");
            }
            TagEnd::List(_) => {
                self.nesting = self.nesting.saturating_sub(1);
                self.list_stack.pop();
                self.output.push('\n');
            }
            TagEnd::Item => {
                self.output.push('\n');
            }
            TagEnd::Table => {
                self.emit_table();
                self.in_table = false;
            }
            TagEnd::TableCell => {
                if let Some(row) = self.table_rows.last_mut() {
                    row.push(std::mem::take(&mut self.current_cell));
                }
                self.in_cell = false;
            }
            TagEnd::TableHead | TagEnd::TableRow => {}
            _ => {}
        }
    }

    fn emit_table(&mut self) {
        if self.table_rows.is_empty() {
            return;
        }
        let cols = self.table_rows[0].len();
        let aligns: Vec<&str> = self
            .table_aligns
            .iter()
            .map(|a| match a {
                pulldown_cmark::Alignment::Left => "left",
                pulldown_cmark::Alignment::Center => "center",
                pulldown_cmark::Alignment::Right => "right",
                pulldown_cmark::Alignment::None => "auto",
            })
            .collect();
        let has_non_default = aligns.iter().any(|a| *a != "auto");

        self.output
            .push_str(&format!("#table(\n  columns: {cols},\n"));
        if has_non_default {
            let align_str = aligns.join(", ");
            self.output.push_str(&format!("  align: ({align_str}),\n"));
        }

        if let Some(header) = self.table_rows.first() {
            self.output.push_str("  table.header(");
            for cell in header {
                self.output.push_str(&format!("[{cell}], "));
            }
            self.output.push_str("),\n");
        }

        for row in self.table_rows.iter().skip(1) {
            for cell in row {
                self.output.push_str(&format!("[{cell}], "));
            }
            self.output.push('\n');
        }
        self.output.push_str(")\n");
    }
}

/// Escape text for use in Typst markup context.
fn escape_markup(text: &str) -> String {
    let mut s = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '#' => s.push_str("\\#"),
            '*' => s.push_str("\\*"),
            '_' => s.push_str("\\_"),
            '`' => s.push_str("\\`"),
            '~' => s.push_str("\\~"),
            '$' => s.push_str("\\$"),
            '<' => s.push_str("\\<"),
            '>' => s.push_str("\\>"),
            '@' => s.push_str("\\@"),
            '[' => s.push_str("\\["),
            ']' => s.push_str("\\]"),
            '{' => s.push_str("\\{"),
            '}' => s.push_str("\\}"),
            '/' => s.push_str("\\/"),
            _ => s.push(c),
        }
    }
    s
}

/// Escape a string for use as a Typst string literal.
pub fn escape_typst_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{{{:04x}}}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out
}

/// Compile a Typst template with data to PDF bytes.
///
/// The template defines a function named `template_fn` that receives
/// named arguments matching TypstInputs fields.
pub fn compile_pdf(
    template: &str,
    template_fn: &str,
    inputs: &TypstInputs,
) -> Result<Vec<u8>, String> {
    let source = build_source(template, template_fn, inputs);
    compile_source(&source)
}

/// Compile a raw Typst source string to PDF bytes.
pub fn compile_source(source: &str) -> Result<Vec<u8>, String> {
    use typst_as_lib::TypstEngine;

    let engine = TypstEngine::builder()
        .main_file(source)
        .fonts(typst_assets::fonts())
        .build();

    let warned = engine.compile();
    let doc = warned
        .output
        .map_err(|e| format!("Typst compilation failed: {e}"))?;

    let pdf = typst_pdf::pdf(&doc, &Default::default())
        .map_err(|e| format!("PDF generation failed: {e:?}"))?;

    Ok(pdf)
}

/// Build a complete Typst source file from template + data.
fn build_source(template: &str, template_fn: &str, inputs: &TypstInputs) -> String {
    let mut src = String::new();

    // 1. Template definition
    src.push_str(template);
    src.push_str("\n\n");

    // 2. Data as Typst values
    src.push_str("#let findings = (\n");
    for f in &inputs.findings {
        src.push_str(&format!(
            "  (id: \"{}\", status: \"{}\", severity: \"{}\", body: eval(\"{}\", mode: \"markup\")),\n",
            escape_typst_string(&f.id),
            escape_typst_string(&f.status),
            escape_typst_string(&f.severity),
            escape_typst_string(&f.body),
        ));
    }
    src.push_str(")\n\n");

    src.push_str("#let requirements = (\n");
    for r in &inputs.requirements {
        src.push_str(&format!(
            "  (id: \"{}\", status: \"{}\", body: eval(\"{}\", mode: \"markup\")),\n",
            escape_typst_string(&r.id),
            escape_typst_string(&r.status),
            escape_typst_string(&r.body),
        ));
    }
    src.push_str(")\n\n");

    if let Some(scope) = &inputs.scope {
        src.push_str(&format!(
            "#let scope = eval(\"{}\", mode: \"markup\")\n\n",
            escape_typst_string(scope)
        ));
    } else {
        src.push_str("#let scope = none\n\n");
    }

    if inputs.sections.is_empty() {
        src.push_str("#let sections = (:)\n\n");
    } else {
        src.push_str("#let sections = (\n");
        for (name, content) in &inputs.sections {
            src.push_str(&format!(
                "  {}: eval(\"{}\", mode: \"markup\"),\n",
                name,
                escape_typst_string(content)
            ));
        }
        src.push_str(")\n\n");
    }

    if inputs.metadata.is_empty() {
        src.push_str("#let metadata = (:)\n\n");
    } else {
        src.push_str("#let metadata = (\n");
        for (key, value) in &inputs.metadata {
            src.push_str(&format!("  {}: \"{}\",\n", key, escape_typst_string(value)));
        }
        src.push_str(")\n\n");
    }

    if inputs.notes.is_empty() {
        src.push_str("#let notes = ()\n\n");
    } else {
        src.push_str("#let notes = (\n");
        for n in &inputs.notes {
            src.push_str(&format!(
                "  (id: \"{}\", body: eval(\"{}\", mode: \"markup\")),\n",
                escape_typst_string(&n.id),
                escape_typst_string(&n.body)
            ));
        }
        src.push_str(")\n\n");
    }

    // 3. Apply template
    src.push_str(&format!(
        "#show: {template_fn}.with(\n  findings: findings,\n  requirements: requirements,\n  scope: scope,\n  sections: sections,\n  metadata: metadata,\n  notes: notes,\n)\n"
    ));

    src
}

/// Get a built-in skeleton template by name.
/// Returns None if no built-in template exists for the given name.
pub fn get_builtin_template(name: &str) -> Option<&'static str> {
    match name {
        "report" => Some(include_str!("typst_templates/report.typ")),
        "sow" => Some(include_str!("typst_templates/sow.typ")),
        "proposal" => Some(include_str!("typst_templates/proposal.typ")),
        "finding" => Some(include_str!("typst_templates/finding.typ")),
        "requirement" => Some(include_str!("typst_templates/requirement.typ")),
        "scope" => Some(include_str!("typst_templates/scope.typ")),
        "note" => Some(include_str!("typst_templates/note.typ")),
        "roe" => Some(include_str!("typst_templates/roe.typ")),
        "nda" => Some(include_str!("typst_templates/nda.typ")),
        "custom" => Some(include_str!("typst_templates/custom.typ")),
        _ => None,
    }
}

/// Load a Typst template: workspace `templates/<name>.typ` if it exists,
/// otherwise the built-in default. Returns (template_content, template_fn_name).
pub fn load_template(
    workspace_root: &camino::Utf8Path,
    name: &str,
) -> Result<(String, String), String> {
    let workspace_template = workspace_root.join("templates").join(format!("{name}.typ"));
    if workspace_template.exists() {
        let content = std::fs::read_to_string(&workspace_template)
            .map_err(|e| format!("Cannot read template '{workspace_template}': {e}"))?;
        Ok((content, name.to_string()))
    } else if let Some(builtin) = get_builtin_template(name) {
        Ok((builtin.to_string(), name.to_string()))
    } else {
        Err(format!(
            "No template found for '{name}'. No workspace template and no built-in default."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_heading() {
        let md = "# Hello\n\nWorld";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("= Hello"));
    }

    #[test]
    fn convert_bold() {
        let md = "**bold text**";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("#strong[bold text]"));
    }

    #[test]
    fn convert_italic() {
        let md = "*italic text*";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("#emph[italic text]"));
    }

    #[test]
    fn convert_inline_code() {
        let md = "`code`";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("#raw(\"code\")"));
    }

    #[test]
    fn convert_unordered_list() {
        let md = "- item one\n- item two";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("- item one"));
        assert!(result.contains("- item two"));
    }

    #[test]
    fn convert_ordered_list() {
        let md = "1. first\n2. second";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("+ first"));
        assert!(result.contains("+ second"));
    }

    #[test]
    fn convert_link() {
        let md = "[text](https://example.com)";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("#link(\"https://example.com\")[text]"));
    }

    #[test]
    fn convert_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("#raw(block: true"));
    }

    #[test]
    fn convert_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let result = markdown_to_typst(md).unwrap();
        assert!(result.contains("#table("));
        assert!(result.contains("columns: 2"));
    }

    #[test]
    fn convert_empty() {
        let result = markdown_to_typst("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn escape_string_basic() {
        assert_eq!(escape_typst_string("hello"), "hello");
        assert_eq!(escape_typst_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_typst_string("line\nbreak"), "line\\nbreak");
        assert_eq!(escape_typst_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn compile_simple_pdf() {
        let source =
            "#set page(paper: \"a4\")\n#set text(font: \"DejaVu Sans\", 11pt)\n\nHello, World!\n";
        let pdf = compile_source(source).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn compile_with_template() {
        let template = "#let report(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {\n  set page(paper: \"a4\", margin: 2cm)\n  set text(font: \"DejaVu Sans\", 11pt)\n  doc\n}\n";
        let inputs = TypstInputs {
            findings: vec![FindingInput {
                id: "ACME-WEB-001".to_string(),
                status: "open".to_string(),
                severity: "high".to_string(),
                body: "= Stored XSS\n\nVulnerable to XSS".to_string(),
            }],
            ..Default::default()
        };
        let pdf = compile_pdf(template, "report", &inputs).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn compile_with_metadata() {
        let template = "#let report(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {\n  set page(paper: \"a4\")\n  set text(font: \"DejaVu Sans\", 11pt)\n  metadata.at(\"client_name\", default: \"Unknown\")\n  parbreak()\n  doc\n}\n";
        let mut inputs = TypstInputs::default();
        inputs
            .metadata
            .insert("client_name".to_string(), "Acme Corp".to_string());
        let pdf = compile_pdf(template, "report", &inputs).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn compile_builtin_report_template() {
        let template = get_builtin_template("report").unwrap();
        let inputs = TypstInputs {
            findings: vec![FindingInput {
                id: "ACME-WEB-001".to_string(),
                status: "open".to_string(),
                severity: "high".to_string(),
                body: "= Stored XSS\n\nThe app is vulnerable".to_string(),
            }],
            scope: Some("= Scope\n\nWeb application".to_string()),
            sections: {
                let mut s = BTreeMap::new();
                s.insert(
                    "methodology".to_string(),
                    "= Methodology\n\nOWASP".to_string(),
                );
                s
            },
            metadata: {
                let mut m = BTreeMap::new();
                m.insert("client_name".to_string(), "Acme Corp".to_string());
                m.insert("project_name".to_string(), "Web App".to_string());
                m.insert("engagement_name".to_string(), "Initial".to_string());
                m
            },
            ..Default::default()
        };
        let pdf = compile_pdf(template, "report", &inputs).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }
}
