mod proc;

generate_templates!("test");

fn main() {
    // Use the generated type
    let instance = Test1 {
        bar: "BAH".into(),
        foo: String::from("example"),
    };

    let instance1 = Test2 {
        bar: "Bah".into(),
        foo: "lla".into(),
    };

    println!("Type has been generated and instantiated! {:?}", instance);
    println!("Type has been generated and instantiated! {:?}", instance1);
}
