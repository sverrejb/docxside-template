use docxside_template::generate_templates;

generate_templates!("examples/save_to_file/templates");

fn main() {
    let doc = HelloWorld {
        first_name: "Alice",
        product_name: "docxside",
    };

    match doc.save("examples/save_to_file/output/greeting") {
        Ok(()) => println!("Saved to examples/save_to_file/output/greeting.docx"),
        Err(e) => eprintln!("Save failed: {e}"),
    }
}
