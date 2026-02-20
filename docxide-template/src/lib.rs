//! Type-safe `.docx` template engine.
//!
//! Use [`generate_templates!`] to scan a directory of `.docx` files at compile time
//! and generate a struct per template. See the [README](https://github.com/sverrejb/docxide-template)
//! for full usage instructions.

pub use docxide_template_derive::generate_templates;

use std::io::{Cursor, Read, Write};
use std::path::Path;

/// Error type returned by template `save()` and `to_bytes()` methods.
#[derive(Debug)]
pub enum TemplateError {
    /// An I/O error (reading template, writing output, creating directories).
    Io(std::io::Error),
    /// The `.docx` template is malformed (bad zip archive, invalid XML encoding).
    InvalidTemplate(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{}", e),
            Self::InvalidTemplate(msg) => write!(f, "invalid template: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::InvalidTemplate(_) => None,
        }
    }
}

impl From<std::io::Error> for TemplateError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

impl From<zip::result::ZipError> for TemplateError {
    fn from(e: zip::result::ZipError) -> Self {
        match e {
            zip::result::ZipError::Io(io_err) => Self::Io(io_err),
            other => Self::InvalidTemplate(other.to_string()),
        }
    }
}

impl From<std::string::FromUtf8Error> for TemplateError {
    fn from(e: std::string::FromUtf8Error) -> Self { Self::InvalidTemplate(e.to_string()) }
}

#[doc(hidden)]
pub trait DocxTemplate {
    /// Returns the path to the original `.docx` template file.
    fn template_path(&self) -> &Path;
    /// Returns placeholder/value pairs for substitution.
    fn replacements(&self) -> Vec<(&str, &str)>;
}

#[doc(hidden)]
pub fn save_docx<T: DocxTemplate, P: AsRef<Path>>(
    template: &T,
    output_path: P,
) -> Result<(), TemplateError> {
    save_docx_from_file(template.template_path(), output_path.as_ref(), &template.replacements())
}

fn save_docx_from_file(
    template_path: &Path,
    output_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<(), TemplateError> {
    let template_bytes = std::fs::read(template_path)?;
    save_docx_bytes(&template_bytes, output_path, replacements)
}

#[doc(hidden)]
pub fn build_docx_bytes(
    template_bytes: &[u8],
    replacements: &[(&str, &str)],
) -> Result<Vec<u8>, TemplateError> {
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

#[doc(hidden)]
pub fn save_docx_bytes(
    template_bytes: &[u8],
    output_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<(), TemplateError> {
    let bytes = build_docx_bytes(template_bytes, replacements)?;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, bytes)?;
    Ok(())
}

fn replace_placeholders_in_xml(xml: &str, replacements: &[(&str, &str)]) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_single_run_placeholder() {
        let xml = r#"<w:t>{Name}</w:t>"#;
        let result = replace_placeholders_in_xml(xml, &[("{Name}", "Alice")]);
        assert_eq!(result, r#"<w:t>Alice</w:t>"#);
    }

    #[test]
    fn replace_placeholder_split_across_runs() {
        let xml = r#"<w:t>{Na</w:t><w:t>me}</w:t>"#;
        let result = replace_placeholders_in_xml(xml, &[("{Name}", "Alice")]);
        assert_eq!(result, r#"<w:t>Alice</w:t><w:t></w:t>"#);
    }

    #[test]
    fn replace_placeholder_with_inner_whitespace() {
        let xml = r#"<w:t>Hello { Name }!</w:t>"#;
        let result = replace_placeholders_in_xml(xml, &[("{ Name }", "Alice")]);
        assert_eq!(result, r#"<w:t>Hello Alice!</w:t>"#);
    }

    #[test]
    fn replace_both_whitespace_variants() {
        let xml = r#"<w:t>{Name} and { Name }</w:t>"#;
        let result = replace_placeholders_in_xml(
            xml,
            &[("{Name}", "Alice"), ("{ Name }", "Alice")],
        );
        assert_eq!(result, r#"<w:t>Alice and Alice</w:t>"#);
    }

    #[test]
    fn replace_multiple_placeholders() {
        let xml = r#"<w:t>Hello {First} {Last}!</w:t>"#;
        let result = replace_placeholders_in_xml(
            xml,
            &[("{First}", "Alice"), ("{Last}", "Smith")],
        );
        assert_eq!(result, r#"<w:t>Hello Alice Smith!</w:t>"#);
    }

    #[test]
    fn no_placeholders_returns_unchanged() {
        let xml = r#"<w:t>No placeholders here</w:t>"#;
        let result = replace_placeholders_in_xml(xml, &[("{Name}", "Alice")]);
        assert_eq!(result, xml);
    }

    #[test]
    fn no_wt_tags_returns_unchanged() {
        let xml = r#"<w:p>plain paragraph</w:p>"#;
        let result = replace_placeholders_in_xml(xml, &[("{Name}", "Alice")]);
        assert_eq!(result, xml);
    }

    #[test]
    fn empty_replacements_returns_unchanged() {
        let xml = r#"<w:t>{Name}</w:t>"#;
        let result = replace_placeholders_in_xml(xml, &[]);
        assert_eq!(result, xml);
    }

    #[test]
    fn preserves_wt_attributes() {
        let xml = r#"<w:t xml:space="preserve">{Name}</w:t>"#;
        let result = replace_placeholders_in_xml(xml, &[("{Name}", "Alice")]);
        assert_eq!(result, r#"<w:t xml:space="preserve">Alice</w:t>"#);
    }

    #[test]
    fn build_docx_bytes_produces_valid_zip() {
        let template_path = Path::new("../test-crate/templates/HelloWorld.docx");
        if !template_path.exists() {
            return;
        }
        let template_bytes = std::fs::read(template_path).unwrap();
        let result = build_docx_bytes(
            &template_bytes,
            &[("{ firstName }", "Test"), ("{ productName }", "Lib")],
        )
        .unwrap();

        assert!(!result.is_empty());
        let cursor = Cursor::new(&result);
        let archive = zip::ZipArchive::new(cursor).expect("output should be a valid zip");
        assert!(archive.len() > 0);
    }

    #[test]
    fn build_docx_bytes_replaces_content() {
        let template_path = Path::new("../test-crate/templates/HelloWorld.docx");
        if !template_path.exists() {
            return;
        }
        let template_bytes = std::fs::read(template_path).unwrap();
        let result = build_docx_bytes(
            &template_bytes,
            &[("{ firstName }", "Alice"), ("{ productName }", "Docxide")],
        )
        .unwrap();

        let cursor = Cursor::new(&result);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut doc_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc_xml)
            .unwrap();
        assert!(doc_xml.contains("Alice"));
        assert!(doc_xml.contains("Docxide"));
        assert!(!doc_xml.contains("firstName"));
        assert!(!doc_xml.contains("productName"));
    }
}
