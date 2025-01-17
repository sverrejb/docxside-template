extern crate proc_macro;
mod templates;

use docx_rs::{read_docx, DocumentChild::Paragraph};
use file_format::FileFormat;
use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use syn::parse_str;
use templates::{derive_type_name_from_filename, placeholder_to_field_name};

fn print_message(message: &str, path: &PathBuf) {
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

#[proc_macro]
pub fn generate_templates(input: TokenStream) -> TokenStream {
    let input_string = input.to_string();
    let folder_path = input_string.trim_matches('"');

    let paths = fs::read_dir(folder_path).expect("Failed to read the folder");

    let mut structs = Vec::new();

    for path in paths {
        //todo: maybe recursive traversal?
        let path = path.expect("Failed to read path").path();

        if !is_valid_docx_file(&path) {
            print_message("Invalid template file, skipping.", &path);
            continue;
        }

        let type_name = match derive_type_name_from_filename(&path) {
            Ok(name) if parse_str::<syn::Ident>(&name).is_ok() => name,
            _ => {
                print_message(
                    "Unable to derive type name from file name. skipping.",
                    &path,
                );

                continue;
            }
        };

        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => {
                continue;
            }
        };

        let mut buf = vec![];

        if let Err(_) = file.read_to_end(&mut buf) {
            print_message("Unable to read file content. Skipping.", &path);
            continue;
        }

        let doc = match read_docx(&buf) {
            Ok(doc) => doc,
            Err(_) => {
                print_message("Unable to read docx content. Skipping.", &path);
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

        let re = Regex::new(r"(\{\s*[^}]+\s*\})").unwrap();
        let mut fields = Vec::new();
        let mut field_names = Vec::new();
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
                    let x = syn::LitStr::new(&field_name, proc_macro::Span::call_site().into());
                    let y = syn::LitStr::new(&placeholder, proc_macro::Span::call_site().into());
                    field_names.push(x);
                    placeholders.push(y);
                } else {
                    println!(
                        "\x1b[34m[Docxside-template]\x1b[0m Invalid placeholder name in file: {}",
                        placeholder
                    );
                }
            }
        }

        let type_ident = syn::Ident::new(type_name.as_str(), proc_macro::Span::call_site().into());
        let path_str = path.to_str().expect("Failed to convert path to string");

        let expanded = quote! {
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
                    use std::io::Write;
                    let file_path = path.as_ref();
                    let mut file = std::fs::File::create(file_path)?;

                    let mut contents = std::fs::read_to_string("test.txt").expect("Should have been able to read the file");

                    #(
                        println!("Value: {}, Placeholder: {}", self.#fields, #placeholders);
                        contents = contents.replace(#placeholders, self.#fields);
                    )*


                    file.write_all(contents.as_bytes())?;
                    Ok(())
                }
            }

        };

        structs.push(expanded)
    }

    let combined = quote! {
        #(#structs)*
    };

    combined.into()
}
