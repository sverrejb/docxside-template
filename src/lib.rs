use docx_rs::read_docx;
use docx_rs::Docx;
use file_format::FileFormat;
use heck::AsPascalCase;
use heck::AsSnakeCase;
use rayon::prelude::*;
use std::collections::HashMap;
use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const TYPE_TEMPLATE: &str = "
pub struct {name} {
    {fields}
}

impl Filename for {name} {
        fn get_filename(&self) -> String{
            \"{file_name}\".to_owned()
        }

        fn get_fields(&self) -> HashMap<&str, &String>{
            {fields_vector}
        }

    }
";

const TRAIT_IMPORT: &str = "use docxside_templates::Filename;
use std::collections::HashMap;
";

#[macro_export]
macro_rules! include_templates {
    () => {
        include!(concat!(env!("OUT_DIR"), "/templates.rs"));
    };
}

fn remove_extension(filename: &str) -> String {
    match filename.rfind('.') {
        Some(index) => filename[..index].to_owned(),
        None => filename.to_owned(),
    }
}
pub trait Filename {
    fn get_filename(&self) -> String;
    fn get_fields(&self) -> HashMap<&str, &String>;
}

pub trait Save {
    fn save(&self);
}

impl<T: Filename> Save for T {
    fn save(&self) {
        println!("Saved {}", self.get_filename());
        for (key, value) in self.get_fields() {
            println!("{}, will be replaced with {}", key, value);
        }
    }
}

fn derive_type_name_from_filename(filename: OsString) -> Result<String, String> {
    if let Ok(file_name_string) = filename.into_string() {
        let mut type_name = remove_extension(file_name_string.as_str());
        type_name = type_name
            .trim_start_matches(|c: char| c.is_numeric())
            .to_string();
        type_name = format!("{}", AsPascalCase(type_name));
        return Ok(type_name);
    }
    Err("Could not convert filename to string".to_owned())
}

fn get_template_variables(doc: &Docx) -> Vec<String> {
    let props = &doc.doc_props.custom.properties;
    props.clone().into_keys().collect()
}

fn get_props(doc: &Docx) -> String {
    let props = &doc.doc_props.custom.properties;
    let mut fields_str = String::new();
    for (key, _) in props {
        let rust_type = "String";
        //TODO: handle all illegal characters
        let mut field_name = key.replace(" ", "_");
        field_name = field_name.replace(":", "_");
        field_name = format!("{}", AsSnakeCase(field_name));
        fields_str.push_str(&format!("{}: {},\n", field_name, rust_type));
    }
    fields_str
}

fn build_struct_fields(fields: &Vec<String>) -> String {
    let mut fields_string = String::new();

    for field in fields {
        let rust_type = "String";
        //TODO: handle all illegal characters
        let mut field_name = field.replace(" ", "_");
        field_name = field_name.replace(":", "_");
        field_name = format!("{}", AsSnakeCase(field_name));
        fields_string.push_str(&format!("{}: {},\n", field_name, rust_type));

        //field_string.push_str(field.as_str())
    }

    fields_string
}

fn variable_to_field_name(variable: &String) -> String {
    let mut field_name = variable.replace(" ", "_");
    field_name = field_name.replace(":", "_");
    field_name = format!("{}", AsSnakeCase(field_name));
    field_name
}

//TODO: rename
fn build_variable_to_field_map(variables: &Vec<String>) -> String {
    let mut result = String::from("let mut map = HashMap::new();\n");

    for variable in variables {
        let field_name = variable_to_field_name(variable);
        let row = "map.insert(\"{key}\", &self.{value});\n";
        let mut modified_row = row.replace("{key}", variable.as_str());
        modified_row = modified_row.replace("{value}", &field_name.as_str());

        result.push_str(&modified_row);
    }

    //let formatted_elements: Vec<String> = fields.iter().map(|s| format!("\"{}\"", s)).collect();
    //result.push_str(&formatted_elements.join(", "));
    result.push_str("map");
    result
}

pub fn generate_types(template_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("templates.rs");

    let template_dir = Path::new(template_path);

    if let Ok(files) = fs::read_dir(template_dir) {
        let structs: Vec<String> = files
            .filter_map(Result::ok)
            .par_bridge()
            .filter_map(|dir_entry| {
                let file_path = dir_entry.path();
                let fmt = FileFormat::from_file(&file_path).ok()?;

                // if not docx, skip file
                if fmt.extension() != "docx" {
                    return None;
                }

                let mut file = File::open(&file_path).ok()?;
                let mut buf = vec![];
                let _ = file.read_to_end(&mut buf);
                let doc = read_docx(&buf).ok()?;

                let type_name = derive_type_name_from_filename(dir_entry.file_name()).ok()?;
                let template_variables = get_template_variables(&doc);
                let fields_string = build_struct_fields(&template_variables);
                let fields_map = build_variable_to_field_map(&template_variables);

                let mut formatted_string = TYPE_TEMPLATE.replace("{name}", type_name.as_str());

                formatted_string = formatted_string.replace("{fields}", fields_string.as_str());
                formatted_string =
                    formatted_string.replace("{file_name}", file_path.as_path().to_str().unwrap());
                formatted_string = formatted_string.replace("{fields_vector}", fields_map.as_str());

                Some(formatted_string)
            })
            .collect();

        let mut generated_types_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_path)
            .unwrap();

        generated_types_file
            .write_all(TRAIT_IMPORT.as_bytes())
            .unwrap();

        for s in structs {
            generated_types_file.write_all(s.as_bytes()).unwrap()
        }
    }
}
