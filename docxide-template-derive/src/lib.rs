extern crate proc_macro;
mod templates;

use docx_rs::{read_docx, DocumentChild::Paragraph};
use file_format::FileFormat;
use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use syn::{parse_str, LitStr};
use templates::{derive_type_name_from_filename, placeholder_to_field_name};

/// Scans a directory for `.docx` template files and generates a typed struct for each one.
///
/// Template paths are resolved as absolute paths at compile time, so binaries work
/// regardless of working directory.
///
/// With the `embed` feature enabled, template bytes are baked into the binary via
/// `include_bytes!`, making it fully self-contained with no runtime file dependencies.
///
/// # Usage
///
/// ```rust,ignore
/// use docxide_template::generate_templates;
///
/// generate_templates!("path/to/templates");
/// ```
///
/// For each `.docx` file, this generates a struct with:
/// - A field for each `{placeholder}` found in the document text (converted to snake_case)
/// - `new()` constructor taking all field values as `impl Into<String>`
/// - `save(path)` to write a filled-in `.docx` to disk
/// - `to_bytes()` to get the filled-in `.docx` as `Vec<u8>`
#[proc_macro]
pub fn generate_templates(input: TokenStream) -> TokenStream {
    let embed = cfg!(feature = "embed");

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
            print_docxide_message("Invalid template file, skipping.", &path);
            continue;
        }

        let type_name = match derive_type_name_from_filename(&path) {
            Ok(name) if parse_str::<syn::Ident>(&name).is_ok() => name,
            other => {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if stem.starts_with(|c: char| c.is_ascii_digit()) {
                    let attempted = other.unwrap_or_default();
                    print_docxide_message(
                        &format!(
                            "Filename starts with a digit, which produces an invalid Rust type name `{}`. Skipping.",
                            if attempted.is_empty() { stem.to_string() } else { attempted }
                        ),
                        &path,
                    );
                } else {
                    print_docxide_message(
                        "Unable to derive a valid Rust type name from file name. Skipping.",
                        &path,
                    );
                }
                continue;
            }
        };

        if let Some(existing_path) = seen_type_names.get(&type_name) {
            panic!(
                "\n\n[Docxide-template] Type name collision: both {:?} and {:?} produce the struct name `{}`.\n\
                Rename one of the files to avoid this conflict.\n",
                existing_path, path, type_name
            );
        }
        seen_type_names.insert(type_name.clone(), path.clone());

        let type_ident = syn::Ident::new(type_name.as_str(), proc_macro::Span::call_site().into());

        let buf = match fs::read(&path) {
            Ok(buf) => buf,
            Err(_) => {
                print_docxide_message("Unable to read file content. Skipping.", &path);
                continue;
            }
        };

        let doc = match read_docx(&buf) {
            Ok(doc) => doc,
            Err(_) => {
                print_docxide_message("Unable to read docx content. Skipping.", &path);
                continue;
            }
        };

        let corpus: Vec<String> = doc
            .document
            .children
            .into_iter()
            .filter_map(|child| match child {
                Paragraph(p) => Some(p.raw_text()),
                _ => None,
            })
            .collect();

        let content = generate_struct_content(corpus);

        let abs_path = path.canonicalize().expect("Failed to canonicalize template path");
        let abs_path_str = abs_path.to_str().expect("Failed to convert path to string");

        let template_struct = generate_struct(
            type_ident,
            abs_path_str,
            &content.fields,
            &content.replacement_placeholders,
            &content.replacement_fields,
            embed,
        );

        structs.push(template_struct)
    }

    let combined = quote! {
        #(#structs)*
    };

    combined.into()
}

fn generate_struct(
    type_ident: syn::Ident,
    abs_path: &str,
    fields: &[syn::Ident],
    replacement_placeholders: &[syn::LitStr],
    replacement_fields: &[syn::Ident],
    embed: bool,
) -> proc_macro2::TokenStream {
    let has_fields = !fields.is_empty();
    let abs_path_lit = syn::LitStr::new(abs_path, proc_macro::Span::call_site().into());

    let save_and_bytes = if embed {
        quote! {
            const TEMPLATE_BYTES: &'static [u8] = include_bytes!(#abs_path_lit);

            pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), docxide_template::TemplateError> {
                use docxide_template::DocxTemplate;
                docxide_template::save_docx_bytes(
                    Self::TEMPLATE_BYTES,
                    path.as_ref().with_extension("docx").as_path(),
                    &self.replacements(),
                )
            }

            pub fn to_bytes(&self) -> Result<Vec<u8>, docxide_template::TemplateError> {
                use docxide_template::DocxTemplate;
                docxide_template::build_docx_bytes(Self::TEMPLATE_BYTES, &self.replacements())
            }
        }
    } else {
        quote! {
            pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), docxide_template::TemplateError> {
                docxide_template::save_docx(self, path.as_ref().with_extension("docx"))
            }

            pub fn to_bytes(&self) -> Result<Vec<u8>, docxide_template::TemplateError> {
                use docxide_template::DocxTemplate;
                let template_bytes = std::fs::read(self.template_path())?;
                docxide_template::build_docx_bytes(&template_bytes, &self.replacements())
            }
        }
    };

    if has_fields {
        quote! {
            #[derive(Debug)]
            pub struct #type_ident {
                #(pub #fields: String,)*
            }

            impl #type_ident {
                pub fn new(#(#fields: impl Into<String>),*) -> Self {
                    Self {
                        #(#fields: #fields.into()),*
                    }
                }

                #save_and_bytes
            }

            impl docxide_template::DocxTemplate for #type_ident {
                fn template_path(&self) -> &std::path::Path {
                    std::path::Path::new(#abs_path_lit)
                }

                fn replacements(&self) -> Vec<(&str, &str)> {
                    vec![#( (#replacement_placeholders, self.#replacement_fields.as_str()), )*]
                }
            }
        }
    } else {
        quote! {
            #[derive(Debug)]
            pub struct #type_ident;

            impl #type_ident {
                #save_and_bytes
            }

            impl docxide_template::DocxTemplate for #type_ident {
                fn template_path(&self) -> &std::path::Path {
                    std::path::Path::new(#abs_path_lit)
                }

                fn replacements(&self) -> Vec<(&str, &str)> {
                    vec![]
                }
            }
        }
    }
}

struct StructContent {
    /// Unique fields for the struct definition and constructor.
    fields: Vec<proc_macro2::Ident>,
    /// All placeholder/field pairs for replacements (may have multiple
    /// placeholder strings mapping to the same field, e.g. `{name}` and `{ name }`).
    replacement_placeholders: Vec<LitStr>,
    replacement_fields: Vec<proc_macro2::Ident>,
}

fn generate_struct_content(corpus: Vec<String>) -> StructContent {
    let re = Regex::new(r"(\{\s*[^}]+\s*\})").unwrap();
    let mut seen_fields = std::collections::HashSet::new();
    let mut fields = Vec::new();
    let mut replacement_placeholders = Vec::new();
    let mut replacement_fields = Vec::new();
    let span = proc_macro::Span::call_site().into();

    for text in &corpus {
        for cap in re.captures_iter(text) {
            let placeholder = cap[1].to_string();
            let cleaned =
                placeholder.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace());
            let field_name = placeholder_to_field_name(cleaned);

            if syn::parse_str::<syn::Ident>(&field_name).is_err() {
                println!(
                    "\x1b[34m[Docxide-template]\x1b[0m Invalid placeholder name in file: {}",
                    placeholder
                );
                continue;
            }

            let ident = syn::Ident::new(&field_name, span);
            if seen_fields.insert(field_name) {
                fields.push(ident.clone());
            }
            replacement_placeholders.push(syn::LitStr::new(&placeholder, span));
            replacement_fields.push(ident);
        }
    }

    StructContent {
        fields,
        replacement_placeholders,
        replacement_fields,
    }
}

fn print_docxide_message(message: &str, path: &Path) {
    println!("\x1b[34m[Docxide-template]\x1b[0m {} {:?}", message, path);
}

fn is_valid_docx_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    matches!(FileFormat::from_file(path), Ok(fmt) if fmt.extension() == "docx")
}
