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

    #[test]
    fn whitespace_variants_produce_same_field_name() {
        let variants = vec![
            "{name}",
            "{ name }",
            "{  name  }",
            "{name }",
            "{ name}",
        ];
        for v in &variants {
            let cleaned = v.trim_matches(|c: char| c == '{' || c == '}' || c.is_whitespace());
            let result = placeholder_to_field_name(&cleaned.to_string());
            assert_eq!(result, "name", "placeholder_to_field_name from {:?}", v);
        }
    }

    #[test]
    fn placeholder_with_colons() {
        assert_eq!(
            placeholder_to_field_name(&"date:start".to_string()),
            "date_start"
        );
    }

    #[test]
    fn type_name_from_various_filenames() {
        let cases = vec![
            ("hello_world.docx", Ok("HelloWorld")),
            ("hello-world.docx", Ok("HelloWorld")),
            ("HelloWorld.docx", Ok("HelloWorld")),
            ("ALLCAPS.docx", Ok("Allcaps")),
            ("my template.docx", Ok("MyTemplate")),
        ];
        for (filename, expected) in cases {
            let result = derive_type_name_from_filename(Path::new(filename));
            assert_eq!(
                result.as_deref(),
                expected,
                "derive_type_name_from_filename({:?})",
                filename
            );
        }
    }

    #[test]
    fn type_name_from_digit_prefix_is_invalid() {
        let result = derive_type_name_from_filename(Path::new("123_test.docx"));
        assert!(result.is_err() || syn::parse_str::<syn::Ident>(&result.unwrap()).is_err());
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
