use docxside_templates::generate_templates;

generate_templates!("test");

fn main() {
    let instance = Type1 {
        baR: "BAH".into(),
        foo: String::from("example"),
    };

    let instance1 = Type2 {
        bar: "Bah".into(),
        foo: "lla".into(),
    };

    println!("Type has been generated and instantiated! {:?}", instance);
    println!("Type has been generated and instantiated! {:?}", instance1);
}
