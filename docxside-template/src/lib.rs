pub use docxside_template_derive::generate_templates;

use std::io::{Cursor, Read, Write};
use std::path::Path;

pub trait DocxTemplate {
    fn template_path(&self) -> &Path;
    fn replacements(&self) -> Vec<(&str, &str)>;
}

pub fn save_docx<T: DocxTemplate, P: AsRef<Path>>(
    template: &T,
    output_path: P,
) -> Result<(), Box<dyn std::error::Error>> {
    save_docx_from_file(template.template_path(), output_path.as_ref(), &template.replacements())
}

pub fn save_docx_from_file(
    template_path: &Path,
    output_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let template_bytes = std::fs::read(template_path)?;
    save_docx_bytes(&template_bytes, output_path, replacements)
}

pub fn build_docx_bytes(
    template_bytes: &[u8],
    replacements: &[(&str, &str)],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(template_bytes);
    let mut archive = zip::read::ZipArchive::new(cursor)?;

    let mut output_buf = Cursor::new(Vec::new());
    let mut zip_writer = zip::write::ZipWriter::new(&mut output_buf);
    let options = zip::write::SimpleFileOptions::default();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name: String = file.name().to_string();

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        if file_name.ends_with(".xml") || file_name.ends_with(".rels") {
            let xml = String::from_utf8(contents)?;
            let replaced = replace_placeholders_in_xml(&xml, replacements);
            contents = replaced.into_bytes();
        }

        zip_writer.start_file(&file_name, options)?;
        zip_writer.write_all(&contents)?;
    }

    zip_writer.finish()?;
    Ok(output_buf.into_inner())
}

pub fn save_docx_bytes(
    template_bytes: &[u8],
    output_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = build_docx_bytes(template_bytes, replacements)?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, bytes)?;
    Ok(())
}

pub fn replace_placeholders_in_xml(xml: &str, replacements: &[(&str, &str)]) -> String {
    let mut text_spans: Vec<(usize, usize, String)> = Vec::new();
    let mut search_start = 0;
    while let Some(tag_start) = xml[search_start..].find("<w:t") {
        let tag_start = search_start + tag_start;
        let content_start = match xml[tag_start..].find('>') {
            Some(pos) => tag_start + pos + 1,
            None => break,
        };
        let content_end = match xml[content_start..].find("</w:t>") {
            Some(pos) => content_start + pos,
            None => break,
        };
        let text = xml[content_start..content_end].to_string();
        text_spans.push((content_start, content_end, text));
        search_start = content_end + 6;
    }

    if text_spans.is_empty() {
        return xml.to_string();
    }

    let concatenated: String = text_spans.iter().map(|(_, _, t)| t.as_str()).collect();

    let mut offset_map: Vec<(usize, usize)> = Vec::new();
    for (span_idx, (_, _, text)) in text_spans.iter().enumerate() {
        for char_offset in 0..text.len() {
            offset_map.push((span_idx, char_offset));
        }
    }

    let mut span_replacements: Vec<Vec<(usize, usize, String)>> = vec![Vec::new(); text_spans.len()];
    for &(placeholder, value) in replacements {
        let mut start = 0;
        while let Some(found) = concatenated[start..].find(placeholder) {
            let match_start = start + found;
            let match_end = match_start + placeholder.len();
            if match_start >= offset_map.len() || match_end > offset_map.len() {
                break;
            }

            let (start_span, start_off) = offset_map[match_start];
            let (end_span, _) = offset_map[match_end - 1];
            let end_off_exclusive = offset_map[match_end - 1].1 + 1;

            if start_span == end_span {
                span_replacements[start_span].push((start_off, end_off_exclusive, value.to_string()));
            } else {
                let first_span_text = &text_spans[start_span].2;
                span_replacements[start_span].push((start_off, first_span_text.len(), value.to_string()));
                for mid in (start_span + 1)..end_span {
                    let mid_len = text_spans[mid].2.len();
                    span_replacements[mid].push((0, mid_len, String::new()));
                }
                span_replacements[end_span].push((0, end_off_exclusive, String::new()));
            }
            start = match_end;
        }
    }

    let mut result = xml.to_string();
    for (span_idx, (content_start, content_end, _)) in text_spans.iter().enumerate().rev() {
        let mut span_text = result[*content_start..*content_end].to_string();
        let mut reps = span_replacements[span_idx].clone();
        reps.sort_by(|a, b| b.0.cmp(&a.0));
        for (from, to, replacement) in reps {
            let safe_to = to.min(span_text.len());
            span_text = format!("{}{}{}", &span_text[..from], replacement, &span_text[safe_to..]);
        }
        result = format!("{}{}{}", &result[..*content_start], span_text, &result[*content_end..]);
    }

    result
}
