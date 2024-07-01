use std::{env, ffi::OsString, fs, path::Path};

extern crate heck;
use heck::AsKebabCase;

const TYPE_TEMPLATE: &str = "pub struct {name} {
            v1: String,
            v2: String,
            v3: String,
        }";

//TODO: Function to include types in consumer.

fn generate_type_name(filename: OsString) -> Result<String, String> {
    if let Ok(file_name_string) = filename.into_string() {
        //TODO update to docx
        //TODO handle multiple things that makes a non-valid rust type name
        let mut type_name = file_name_string.replace(".txt", "");
        type_name = file_name_string.replace(".", "_");
        return Ok(type_name);
    }
    Err("Could not convert filename to string".to_owned())
}

pub fn generate_types(template_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("templates.rs");

    println!("OUT_DIR: {:?}", out_dir);

    let template_dir = Path::new(template_path);

    //TODO: only handle docx-files, igonore others
    if let Ok(files) = fs::read_dir(template_dir) {
        for file in files {
            let type_name = file.unwrap().file_name();
            let formatted_string = TYPE_TEMPLATE.replace("{name}", type_name.to_str().unwrap());
            fs::write(&dest_path, formatted_string).unwrap();
        }
    }
}
