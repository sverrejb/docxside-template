use docx_rs::{
    DocumentChild, FooterChild, HeaderChild, StructuredDataTagChild, Table, TableCellContent,
    TableChild, TableRowChild,
};
use file_format::FileFormat;
use std::path::Path;

pub(crate) fn collect_text_from_document_children(children: Vec<DocumentChild>) -> Vec<String> {
    let mut texts = Vec::new();
    for child in children {
        match child {
            DocumentChild::Paragraph(p) => texts.push(p.raw_text()),
            DocumentChild::Table(t) => texts.extend(collect_text_from_table(&t)),
            DocumentChild::StructuredDataTag(sdt) => {
                texts.extend(collect_text_from_sdt_children(&sdt.children));
            }
            _ => {}
        }
    }
    texts
}

pub(crate) fn collect_text_from_table(table: &Table) -> Vec<String> {
    let mut texts = Vec::new();
    for row in &table.rows {
        let TableChild::TableRow(ref row) = row;
        for cell in &row.cells {
            let TableRowChild::TableCell(ref cell) = cell;
            for content in &cell.children {
                match content {
                    TableCellContent::Paragraph(p) => texts.push(p.raw_text()),
                    TableCellContent::Table(t) => texts.extend(collect_text_from_table(t)),
                    _ => {}
                }
            }
        }
    }
    texts
}

fn collect_text_from_sdt_children(children: &[StructuredDataTagChild]) -> Vec<String> {
    let mut texts = Vec::new();
    for child in children {
        match child {
            StructuredDataTagChild::Paragraph(p) => texts.push(p.raw_text()),
            StructuredDataTagChild::Table(t) => texts.extend(collect_text_from_table(t)),
            StructuredDataTagChild::StructuredDataTag(sdt) => {
                texts.extend(collect_text_from_sdt_children(&sdt.children));
            }
            _ => {}
        }
    }
    texts
}

pub(crate) fn collect_text_from_header_children(children: &[HeaderChild]) -> Vec<String> {
    let mut texts = Vec::new();
    for child in children {
        match child {
            HeaderChild::Paragraph(p) => texts.push(p.raw_text()),
            HeaderChild::Table(t) => texts.extend(collect_text_from_table(t)),
            HeaderChild::StructuredDataTag(sdt) => {
                texts.extend(collect_text_from_sdt_children(&sdt.children));
            }
        }
    }
    texts
}

pub(crate) fn collect_text_from_footer_children(children: &[FooterChild]) -> Vec<String> {
    let mut texts = Vec::new();
    for child in children {
        match child {
            FooterChild::Paragraph(p) => texts.push(p.raw_text()),
            FooterChild::Table(t) => texts.extend(collect_text_from_table(t)),
            FooterChild::StructuredDataTag(sdt) => {
                texts.extend(collect_text_from_sdt_children(&sdt.children));
            }
        }
    }
    texts
}

pub(crate) fn print_docxide_message(message: &str, path: &Path) {
    println!("\x1b[34m[Docxide-template]\x1b[0m {} {:?}", message, path);
}

pub(crate) fn is_valid_docx_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    matches!(FileFormat::from_file(path), Ok(fmt) if fmt.extension() == "docx")
}
