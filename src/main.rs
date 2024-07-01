use std::env;

// include!(concat!(env!("OUT_DIR"), "/templates.rs"));

fn main() {
    println!("Hello, world!");

    let out_dir = env::var_os("OUT_DIR").unwrap();

    let template = Foo {
        v1: "Goodbye".to_string(),
        v2: "World".to_string(),
        v3: "Hello".to_string(),
    };
}
