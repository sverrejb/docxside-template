extern crate proc_macro;
mod templates;

use docx_rs::{read_docx, DocumentChild::Paragraph};
use file_format::FileFormat;
use proc_macro::TokenStream;
use proc_macro2;
use quote::quote;
use regex::Regex;
use std::{
    collections::HashMap,
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
    let mut seen_type_names: HashMap<String, PathBuf> = HashMap::new();

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
            other => {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if stem.starts_with(|c: char| c.is_ascii_digit()) {
                    let attempted = other.unwrap_or_default();
                    print_docxside_message(
                        &format!(
                            "Filename starts with a digit, which produces an invalid Rust type name `{}`. Skipping.",
                            if attempted.is_empty() { stem.to_string() } else { attempted }
                        ),
                        &path,
                    );
                } else {
                    print_docxside_message(
                        "Unable to derive a valid Rust type name from file name. Skipping.",
                        &path,
                    );
                }
                continue;
            }
        };

        if let Some(existing_path) = seen_type_names.get(&type_name) {
            panic!(
                "\n\n[Docxside-template] Type name collision: both {:?} and {:?} produce the struct name `{}`.\n\
                Rename one of the files to avoid this conflict.\n",
                existing_path, path, type_name
            );
        }
        seen_type_names.insert(type_name.clone(), path.clone());

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

                pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
                    docxside_template::save_docx(self, path.as_ref().with_extension("docx"))
                }
            }

            impl<'a> docxside_template::DocxTemplate for #type_ident<'a> {
                fn template_path(&self) -> &std::path::Path {
                    std::path::Path::new(#path_str)
                }

                fn replacements(&self) -> Vec<(&str, &str)> {
                    vec![#( (#placeholders, self.#fields), )*]
                }
            }
        }
    } else {
        quote! {
            #[derive(Debug)]
            pub struct #type_ident;

            impl #type_ident {
                pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
                    docxside_template::save_docx(self, path.as_ref().with_extension("docx"))
                }
            }

            impl docxside_template::DocxTemplate for #type_ident {
                fn template_path(&self) -> &std::path::Path {
                    std::path::Path::new(#path_str)
                }

                fn replacements(&self) -> Vec<(&str, &str)> {
                    vec![]
                }
            }
        }
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
