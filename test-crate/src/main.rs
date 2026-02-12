use docxside_templates::generate_templates;

generate_templates!("test-crate/templates");

fn main() {
    let hw = HelloWorld::new("Sverre", "Docxside");
    println!("{:?}", hw);

    match hw.save("output/hello") {
        Ok(_) => println!("Saved to output/hello.docx"),
        Err(e) => println!("Save failed: {}", e),
    }
}

