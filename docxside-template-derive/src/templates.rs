use heck::{AsSnakeCase, ToPascalCase};
use std::path::Path;


pub fn placeholder_to_field_name(variable: &String) -> String {
    let mut field_name = variable.replace(" ", "_");
    //TODO: handle all illegal characters
    field_name = field_name.replace(":", "_");
    field_name = format!("{}", AsSnakeCase(field_name));
    field_name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_name_formats() {
        let cases = vec![
            ("FirstName", "first_name"),
            ("first_name", "first_name"),
            ("FIRST_NAME", "first_name"),
            ("firstName", "first_name"),
            ("first-name", "first_name"),
            ("first name", "first_name"),
            ("FIRSTNAME", "firstname"),
        ];
        for (input, expected) in cases {
            let result = placeholder_to_field_name(&input.to_string());
            assert_eq!(result, expected, "placeholder_to_field_name({:?})", input);
        }
    }
}

pub fn derive_type_name_from_filename(filename: &Path) -> Result<String, String> {
    // Extract the file stem (filename without extension)
    let file_stem = filename
        .file_stem()
        .ok_or_else(|| "Could not extract file stem".to_owned())?
        .to_str()
        .ok_or_else(|| "Could not convert file stem to string".to_owned())?;

    // Convert to CamelCase to follow Rust's naming conventions for types
    let type_name = file_stem.to_pascal_case();

    // Validate that the type name is a valid Rust identifier
    if syn::parse_str::<syn::Ident>(&type_name).is_err() {
        return Err("Invalid type name derived from filename".to_owned());
    }

    Ok(type_name)
}
