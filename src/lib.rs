use std::{
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use file_format::FileFormat;
use heck::AsPascalCase;

const TYPE_TEMPLATE: &str = "
pub struct {name} {
    v1: String,
    v2: String,
    v3: String,
}
";

//TODO: Function to include types in consumer.

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
        type_name = format!("{}", AsPascalCase(type_name));
        return Ok(type_name);
    }
    Err("Could not convert filename to string".to_owned())
}

pub fn generate_types(template_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("templates.rs");

    println!("OUT_DIR: {:?}", out_dir);

    let template_dir = Path::new(template_path);
    let mut generated_types_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest_path)
        .unwrap();

    if let Ok(files) = fs::read_dir(template_dir) {
        for file in files {
            let f = file.as_ref().unwrap();
            let fmt = FileFormat::from_file(f.path()).unwrap();
            // ignore non-docx files
            if fmt.extension() != "docx" {
                continue;
            }

            //TOOD: handle errors
            if let Ok(type_name) = generate_type_name(file.unwrap().file_name()) {
                let formatted_string = TYPE_TEMPLATE.replace("{name}", type_name.as_str());
                generated_types_file
                    .write_all(formatted_string.as_bytes())
                    .unwrap();
            }
        }
    }
}
