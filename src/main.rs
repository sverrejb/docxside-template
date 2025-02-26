use docxside_templates::generate_templates;

generate_templates!("test");

fn main() {
    let instance = HelloWorld {
        name: "Sverre".into(),
        test: "hehe".into(),
        big_test: "lol".into(),
        subject: "Beefs",
        fat_bat: "snaxk",
    };

    println!("{:?}", instance);
    println!("{:?}", instance.get_file_path());

    match instance.save("./foo/testfest.docx") {
        Ok(_) => println!("SAVED"),
        Err(err) => println!("FAILED: {}", err),
    }

}
