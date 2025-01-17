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
    let _ = instance.save("path.txt");
}
