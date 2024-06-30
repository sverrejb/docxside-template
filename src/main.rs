use std::env;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

fn main() {
    println!("Hello, world!");

    let out_dir = env::var_os("OUT_DIR").unwrap();

    let template = DocxTemplate {
        v1: "Goodbye".to_string(),
        v2: "World".to_string(),
    };
    
}
