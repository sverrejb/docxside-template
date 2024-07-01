use std::{
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use heck::AsPascalCase;

const TYPE_TEMPLATE: &str = "
pub struct {name} {
    v1: String,
    v2: String,
    v3: String,
}
";

//TODO: Function to include types in consumer.

fn generate_type_name(filename: OsString) -> Result<String, String> {
    if let Ok(file_name_string) = filename.into_string() {
        //TODO update to docx
        //TODO handle multiple things that makes a non-valid rust type name
        let mut type_name = file_name_string.replace(".txt", "");
        type_name = type_name.replace(".", "_");
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
        .truncate(true)
        .open(dest_path)
        .unwrap();

    //TODO: only handle docx-files, igonore others
    if let Ok(files) = fs::read_dir(template_dir) {
        for file in files {
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
