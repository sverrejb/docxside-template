extern crate proc_macro;
use std::fs;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro]
pub fn generate_templates(input: TokenStream) -> TokenStream {
    let input_string = input.to_string();
    let folder_path = input_string.trim_matches('"');

    let paths = fs::read_dir(folder_path).expect("Failed to read the folder");

    let mut structs = Vec::new();

    for path in paths {
        let path = path.expect("Failed to read path").path();
        if !path.is_file() {
            continue;
        }
        match path.extension() {
            Some(ext) if ext == "txt" => {}
            None => continue,
            _ => continue,
        }

        let content = fs::read_to_string(&path).expect("Failed to read the file");
        let mut lines = content.lines();
        let type_name = lines.next().expect("File is empty").trim();

        // Validate that the type name is a valid Rust identifier
        if !syn::parse_str::<syn::Ident>(type_name).is_ok() {
            panic!("Invalid type name in file: {}", type_name);
        }

        // The remaining lines are the field names
        let fields: Vec<syn::Ident> = lines
            .map(|line| {
                let field_name = line.trim();
                if !syn::parse_str::<syn::Ident>(field_name).is_ok() {
                    panic!("Invalid field name in file: {}", field_name);
                }
                syn::Ident::new(field_name, proc_macro::Span::call_site().into())
            })
            .collect();

        // Generate a struct with the name and fields from the file, and derive Debug
        let type_ident = syn::Ident::new(type_name, proc_macro::Span::call_site().into());
        let expanded = quote! {
            #[derive(Debug)]
            pub struct #type_ident {
                #(pub #fields: String,)*
            }
        };

        structs.push(expanded);
    }

    // Combine all generated structs into a single TokenStream
    let combined = quote! {
        #(#structs)*
    };

    // Return the generated code as a TokenStream
    combined.into()
}
