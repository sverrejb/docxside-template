use std::{env, fs, path::Path};

//TODO: Function to include types in consumer.

pub fn generate_types(template_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("templates.rs");
    fs::write(
        &dest_path,
        "pub struct DocxTemplate {
            v1: String,
            v2: String
        }"
    ).unwrap();
}

