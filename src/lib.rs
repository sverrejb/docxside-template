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

fn generate_struct(
    type_ident: syn::Ident,
    path_str: &str,
    fields: &[syn::Ident],
    placeholders: &[syn::LitStr],
) -> proc_macro2::TokenStream {
    let tokens = quote! {
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
                    let template_path = self.get_file_path();
                    let template_copy_path = path.as_ref().with_extension("docx");

                    // Open the template .docx file as a zip archive
                    let template_file = std::fs::File::open(template_path)?;
                    let mut archive = zip::read::ZipArchive::new(template_file)?;

                    // Create a new zip archive for the output .docx file
                    let output_file = std::fs::File::create(template_copy_path)?;
                    let mut zip_writer = zip::write::ZipWriter::new(output_file);

                     // Iterate through the files in the template archive

                    for i in 0..archive.len() {
                        let mut file = archive.by_index(i)?;
                        let file_name: String = file.name().to_string();

                        // Read the file contents
                        let mut contents = Vec::new();
                        std::io::Read::read_to_end(&mut file, &mut contents)?;

                        // If the file is an XML file, replace placeholders
                        if file_name.ends_with(".xml") {
                            let mut contents_str = String::from_utf8(contents)?;
                            #(
                                contents_str = contents_str.replace(#placeholders, self.#fields);
                            )*
                            contents = contents_str.into_bytes();
                        }

                        // Write the file to the output archive
                        //let file_options: zip::write::FileOptions = zip::write::ExtendedFileOptions::default().into();
                        //zip_writer.start_file(file_name, file_options)?;
                        //zip_writer.write_all(&contents)?;
                    }


                    //FOR DEBUG PURPOSES
                    #(
                        println!("Value: {}, Placeholder: {}", self.#fields, #placeholders);
                    )*

                    // Copy the template file
                    //std::fs::copy(template_path, template_copy_path)?;


                    Ok(())
                }
            }
    };

    tokens
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

        let template_struct = generate_struct(type_ident, path_str, &fields, &placeholders);

        structs.push(template_struct)
    }

    let combined = quote! {
        #(#structs)*
    };

    combined.into()
}
