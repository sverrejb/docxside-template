use docxide_template::generate_templates;

generate_templates!("examples/embedded_templates/templates");

fn main() {
    let doc = HelloWorld {
        first_name: "Alice",
        product_name: "docxide",
    };

    match doc.save("examples/embedded/output/greeting") {
        Ok(()) => println!("Saved to examples/embedded/output/greeting.docx"),
        Err(e) => eprintln!("Save failed: {e}"),
    }

    match doc.to_bytes() {
        Ok(bytes) => println!("Generated {}-byte docx in memory", bytes.len()),
        Err(e) => eprintln!("to_bytes failed: {e}"),
    }
}
