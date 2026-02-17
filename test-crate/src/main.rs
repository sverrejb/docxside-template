use std::io::{self, Write};

use docxide_template::generate_templates;

generate_templates!("test-crate/templates");

fn main() {
    print!("What is your name? ");
    io::stdout().flush().unwrap();
    let mut name = String::new();
    io::stdin().read_line(&mut name).unwrap();
    let name = name.trim();

    print!("Filename? ");
    io::stdout().flush().unwrap();
    let mut filename = String::new();
    io::stdin().read_line(&mut filename).unwrap();
    let filename = filename.trim();

    let hw = HelloWorld::new(name, "docxide");

    let path = format!("output/{filename}");
    match hw.save(&path) {
        Ok(_) => println!("Saved to {path}.docx"),
        Err(e) => println!("Save failed: {e}"),
    }

    match hw.to_bytes() {
        Ok(bytes) => println!("to_bytes() returned {} bytes", bytes.len()),
        Err(e) => println!("to_bytes() failed: {e}"),
    }
}

