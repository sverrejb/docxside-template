use docx_rs::read_docx;
use docx_rs::Docx;
use file_format::FileFormat;
use heck::AsPascalCase;
use heck::AsSnakeCase;
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

    println!("OUT_DIR: {:?}", out_dir);

    let template_dir = Path::new(template_path);

    if let Ok(files) = fs::read_dir(template_dir) {
        let mut structs = vec![];
        for dir_entry in files {
            let file_path = dir_entry.as_ref().unwrap().path();
            let fmt = FileFormat::from_file(&file_path).unwrap();
            // ignore and skip non-docx files
            if fmt.extension() != "docx" {
                continue;
            }

            match File::open(file_path) {
                Ok(mut file) => {
                    let mut buf = vec![];
                    let _ = file.read_to_end(&mut buf);
                    let doc = read_docx(&buf).unwrap();

                    if let Ok(type_name) = generate_type_name(dir_entry.unwrap().file_name()) {
                        let mut formatted_string =
                            TYPE_TEMPLATE.replace("{name}", type_name.as_str());
                        formatted_string =
                            formatted_string.replace("[props]", get_props(&doc).as_str());

                        structs.push(formatted_string);
                    }
                }

                Err(e) => {
                    println!("Error opening file: {}", e);
                }
            }
        }
        let mut generated_types_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_path)
            .unwrap();

        for s in structs {
            generated_types_file.write_all(s.as_bytes()).unwrap()
        }
    }
}
