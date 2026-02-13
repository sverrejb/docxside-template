use docxside_templates::generate_templates;

generate_templates!("test-crate/templates");

fn main() {
    let hw = HelloWorld::new("Sverre", "Docxside");
    println!("{:?}", hw);

    match hw.save("output/hello") {
        Ok(_) => println!("Saved to output/hello.docx"),
        Err(e) => println!("Save failed: {}", e),
    }

    // Test: split-runs template — placeholders split across <w:r> boundaries
    let sr = SplitRunsTemplate::new("Alice", "Acme Corp");
    println!("{:?}", sr);

    match sr.save("output/split_runs_output") {
        Ok(_) => println!("Saved split_runs to output/split_runs_output.docx"),
        Err(e) => println!("Split runs save failed: {}", e),
    }
}

