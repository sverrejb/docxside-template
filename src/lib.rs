extern crate proc_macro;
mod templates;

use docx_rs::{read_docx, DocumentChild::Paragraph};
use file_format::FileFormat;
use proc_macro::TokenStream;
use proc_macro2;
use quote::quote;
use regex::Regex;
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use syn::{parse_str, LitStr};
use templates::{derive_type_name_from_filename, placeholder_to_field_name};

#[proc_macro]
pub fn generate_templates(input: TokenStream) -> TokenStream {
    let input_string = input.to_string();
    let folder_path = input_string.trim_matches('"');

    let paths = fs::read_dir(folder_path).expect("Failed to read the folder");
    let mut structs = Vec::new();

    for path in paths {
        //todo: maybe recursive traversal?
        let path = path.expect("Failed to read path").path();

        // TOOD: Move all validation into function
        if !is_valid_docx_file(&path) {
            print_docxside_message("Invalid template file, skipping.", &path);
            continue;
        }

        let type_name = match derive_type_name_from_filename(&path) {
            Ok(name) if parse_str::<syn::Ident>(&name).is_ok() => name,
            _ => {
                print_docxside_message(
                    "Unable to derive type name from file name. skipping.",
                    &path,
                );
                continue;
            }
        };
        let type_ident = syn::Ident::new(type_name.as_str(), proc_macro::Span::call_site().into());

        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => {
                continue;
            }
        };

        let mut buf = vec![];

        if let Err(_) = file.read_to_end(&mut buf) {
            print_docxside_message("Unable to read file content. Skipping.", &path);
            continue;
        }

        let doc = match read_docx(&buf) {
            Ok(doc) => doc,
            Err(_) => {
                print_docxside_message("Unable to read docx content. Skipping.", &path);
                continue;
            }
        };

        let content = doc.document.children;
        let mut corpus: Vec<String> = vec![];

        for child in content {
            match child {
                Paragraph(paragraph) => corpus.push(paragraph.raw_text()),
                _ => {}
            }
        }

        let struct_content = generate_struct_content(corpus);
        let path_str = path.to_str().expect("Failed to convert path to string");

        let fields = struct_content.fields;
        let placeholders = struct_content.placeholders;

        let template_struct = generate_struct(type_ident, path_str, &fields, &placeholders);

        structs.push(template_struct)
    }

    let combined = quote! {
        #(#structs)*

        fn replace_placeholders_in_xml(xml: &str, replacements: &[(&str, &str)]) -> String {
            // Collect all <w:t ...>...</w:t> spans with their byte positions
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

            // Concatenate all text spans to find placeholders across run boundaries
            let concatenated: String = text_spans.iter().map(|(_, _, t)| t.as_str()).collect();

            // Build a mapping from concatenated-text offset to (span_index, offset_within_span)
            let mut offset_map: Vec<(usize, usize)> = Vec::new();
            for (span_idx, (_, _, text)) in text_spans.iter().enumerate() {
                for char_offset in 0..text.len() {
                    offset_map.push((span_idx, char_offset));
                }
            }

            // Find all placeholder occurrences and build replacement instructions
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
                        // Replacement spans multiple <w:t> elements
                        // Put the replacement value in the first span, clear the rest
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

            // Apply replacements to each span (in reverse order to preserve offsets)
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
    };

    combined.into()
}

fn generate_struct(
    type_ident: syn::Ident,
    path_str: &str,
    fields: &[syn::Ident],
    placeholders: &[syn::LitStr],
) -> proc_macro2::TokenStream {
    let has_fields = !fields.is_empty();

    let save_body = generate_save_body(placeholders, fields);

    if has_fields {
        quote! {
            #[derive(Debug)]
            pub struct #type_ident<'a> {
                #(pub #fields: &'a str,)*
            }

            impl<'a> #type_ident<'a> {
                pub fn new(#(#fields: &'a str),*) -> Self {
                    Self {
                        #(#fields),*
                    }
                }

                fn get_file_path(&self) -> &'static std::path::Path {
                    std::path::Path::new(#path_str)
                }

                pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
                    #save_body
                }
            }
        }
    } else {
        quote! {
            #[derive(Debug)]
            pub struct #type_ident;

            impl #type_ident {
                fn get_file_path(&self) -> &'static std::path::Path {
                    std::path::Path::new(#path_str)
                }

                pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
                    #save_body
                }
            }
        }
    }
}

fn generate_save_body(
    placeholders: &[syn::LitStr],
    fields: &[syn::Ident],
) -> proc_macro2::TokenStream {
    quote! {
        use std::io::Write;
        let template_path = self.get_file_path();
        let output_path = path.as_ref().with_extension("docx");

        let replacements: Vec<(&str, &str)> = vec![
            #( (#placeholders, self.#fields), )*
        ];

        let template_file = std::fs::File::open(template_path)?;
        let mut archive = zip::read::ZipArchive::new(template_file)?;

        let output_file = std::fs::File::create(&output_path)?;
        let mut zip_writer = zip::write::ZipWriter::new(output_file);
        let options = zip::write::SimpleFileOptions::default();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name: String = file.name().to_string();

            let mut contents = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut contents)?;

            if file_name.ends_with(".xml") || file_name.ends_with(".rels") {
                let xml = String::from_utf8(contents)?;
                let replaced = replace_placeholders_in_xml(&xml, &replacements);
                contents = replaced.into_bytes();
            }

            zip_writer.start_file(&file_name, options)?;
            zip_writer.write_all(&contents)?;
        }

        zip_writer.finish()?;
        Ok(())
    }
}

struct StructContent {
    fields: Vec<proc_macro2::Ident>,
    placeholders: Vec<LitStr>,
}

fn generate_struct_content(corpus: Vec<String>) -> StructContent {
    let re = Regex::new(r"(\{\s*[^}]+\s*\})").unwrap();
    let mut fields = Vec::new();
    let mut placeholders = Vec::new();

    for text in corpus {
        for cap in re.captures_iter(&text) {
            let placeholder = cap[1].to_string();
            let cleaned_placeholder: &str =
                placeholder.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace());
            let field_name = placeholder_to_field_name(&cleaned_placeholder.to_string());
            if syn::parse_str::<syn::Ident>(&field_name).is_ok() {
                fields.push(syn::Ident::new(
                    &field_name,
                    proc_macro::Span::call_site().into(),
                ));
                let y = syn::LitStr::new(&placeholder, proc_macro::Span::call_site().into());
                placeholders.push(y);
            } else {
                println!(
                    "\x1b[34m[Docxside-template]\x1b[0m Invalid placeholder name in file: {}",
                    placeholder
                );
            }
        }
    }

    StructContent {
        fields,
        placeholders,
    }
}

fn print_docxside_message(message: &str, path: &PathBuf) {
    println!("\x1b[34m[Docxside-template]\x1b[0m {} {:?}", message, path);
}

fn is_valid_docx_file(path: &PathBuf) -> bool {
    if !path.is_file() {
        return false;
    }

    match FileFormat::from_file(&path) {
        Ok(fmt) if fmt.extension() == "docx" => return true,
        Ok(_) => return false,
        Err(_) => return false,
    }
}
