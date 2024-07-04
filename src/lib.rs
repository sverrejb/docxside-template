use docx_rs::read_docx;
use docx_rs::Docx;
use file_format::FileFormat;
use heck::AsPascalCase;
use heck::AsSnakeCase;
use rayon::prelude::*;
use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const TYPE_TEMPLATE: &str = "
pub struct {name} {
    [props]
}

impl Template for {name} {
        fn save(&self) {
            println!(\"{}\", \"Saved!\")
        }
    }
";

const TYPE_TRAIT: &str = "
pub trait Template {
    fn save(&self);
}
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
pub trait DocTemplate {
    fn save(&self);
}

fn generate_type_name(filename: OsString) -> Result<String, String> {
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

                let mut file = File::open(file_path).ok()?;
                let mut buf = vec![];
                let _ = file.read_to_end(&mut buf);
                let doc = read_docx(&buf).ok()?;

                let type_name = generate_type_name(dir_entry.file_name()).ok()?;
                let mut formatted_string = TYPE_TEMPLATE.replace("{name}", type_name.as_str());
                formatted_string = formatted_string.replace("[props]", get_props(&doc).as_str());

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
            .write_all(TYPE_TRAIT.as_bytes())
            .unwrap();

        for s in structs {
            generated_types_file.write_all(s.as_bytes()).unwrap()
        }
    }
}
