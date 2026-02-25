use regex::Regex;
use syn::LitStr;

use crate::naming::placeholder_to_field_name;

pub(crate) struct StructContent {
    pub(crate) fields: Vec<proc_macro2::Ident>,
    /// All placeholder/field pairs for replacements (may have multiple
    /// placeholder strings mapping to the same field, e.g. `{name}` and `{ name }`).
    pub(crate) replacement_placeholders: Vec<LitStr>,
    pub(crate) replacement_fields: Vec<proc_macro2::Ident>,
}

pub(crate) fn generate_struct_content(corpus: Vec<String>) -> StructContent {
    let re = Regex::new(r"(\{\s*[^}]+\s*\})").unwrap();
    let mut seen_fields = std::collections::HashSet::new();
    let mut seen_placeholders = std::collections::HashSet::new();
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
            if seen_placeholders.insert(placeholder.clone()) {
                replacement_placeholders.push(syn::LitStr::new(&placeholder, span));
                replacement_fields.push(ident);
            }
        }
    }

    StructContent {
        fields,
        replacement_placeholders,
        replacement_fields,
    }
}
