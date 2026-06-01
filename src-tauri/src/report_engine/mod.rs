use printpdf::*;
use std::io::BufWriter;

pub struct ReportData {
    pub name: String,
    pub client_name: String,
    pub engagement_name: String,
    pub executive_summary: String,
    pub appendix: String,
    pub finding_titles: Vec<String>,
}

pub fn generate_pdf(report: &ReportData, save_path: &str) -> Result<(), String> {
    let (doc, page1, layer1) = PdfDocument::new(&report.name, Mm(210.0), Mm(297.0), "Cover");

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Font: {}", e))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Font: {}", e))?;

    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    // Cover page
    current_layer.use_text(&report.name, 24.0, Mm(20.0), Mm(270.0), &font_bold);
    current_layer.use_text(
        format!("Client: {}", report.client_name),
        14.0,
        Mm(20.0),
        Mm(255.0),
        &font,
    );
    current_layer.use_text(
        format!("Engagement: {}", report.engagement_name),
        14.0,
        Mm(20.0),
        Mm(245.0),
        &font,
    );

    // Executive summary page
    let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Summary");
    current_layer = doc.get_page(page2).get_layer(layer2);
    current_layer.use_text("Executive Summary", 18.0, Mm(20.0), Mm(270.0), &font_bold);
    let summary_lines = wrap_text(&report.executive_summary, 80);
    let mut y = 255.0;
    for line in &summary_lines {
        current_layer.use_text(line, 11.0, Mm(20.0), Mm(y), &font);
        y -= 6.0;
    }

    // Findings pages
    if !report.finding_titles.is_empty() {
        let (page3, layer3) = doc.add_page(Mm(210.0), Mm(297.0), "Findings");
        current_layer = doc.get_page(page3).get_layer(layer3);
        current_layer.use_text("Findings", 18.0, Mm(20.0), Mm(270.0), &font_bold);
        let mut y = 255.0;
        for title in &report.finding_titles {
            current_layer.use_text(format!("• {}", title), 12.0, Mm(20.0), Mm(y), &font);
            y -= 6.0;
        }
    }

    // Appendix page
    let (page4, layer4) = doc.add_page(Mm(210.0), Mm(297.0), "Appendix");
    current_layer = doc.get_page(page4).get_layer(layer4);
    current_layer.use_text("Appendix", 18.0, Mm(20.0), Mm(270.0), &font_bold);
    let appendix_lines = wrap_text(&report.appendix, 80);
    let mut y = 255.0;
    for line in &appendix_lines {
        current_layer.use_text(line, 11.0, Mm(20.0), Mm(y), &font);
        y -= 6.0;
    }

    doc.save(&mut BufWriter::new(
        std::fs::File::create(save_path).map_err(|e| format!("File: {}", e))?,
    ))
    .map_err(|e| format!("PDF save: {}", e))?;

    Ok(())
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split_whitespace() {
            if current.len() + word.len() + 1 > width {
                lines.push(current.trim().to_string());
                current = word.to_string();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }
        if !current.is_empty() {
            lines.push(current.trim().to_string());
        }
    }
    if lines.is_empty() {
        lines.push("".to_string());
    }
    lines
}
