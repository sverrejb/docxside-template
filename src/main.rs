use docxside_templates::generate_templates;

generate_templates!("test");

fn main() {
    let instance = HelloWorld {
        name: "Sverre".into(),
        test: "hehe".into(),
        big_test: "lol".into(),
        subject: "nah",
    };

    println!("{:?}", instance);
}
