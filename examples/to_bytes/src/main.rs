use docxide_template::generate_templates;

generate_templates!("examples/to_bytes/templates");

fn main() {
    let doc = HelloWorld::new("Alice", "docxide");

    match doc.to_bytes() {
        Ok(bytes) => println!("Generated {}-byte docx in memory", bytes.len()),
        Err(e) => eprintln!("to_bytes failed: {e}"),
    }
}
