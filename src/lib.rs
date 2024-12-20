use docx_rs::read_docx;
use docx_rs::Docx;
use file_format::FileFormat;
use heck::{AsPascalCase, AsSnakeCase};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::collections::HashMap;
use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const MODULE_PRELUDE: &str = "
pub mod templates {
    use docxside_templates::Filename;
    use std::collections::HashMap;
";

const TYPE_TEMPLATE: &str = "
pub struct {name} {
    {fields}
}

impl Filename for {name} {
        fn get_filename(&self) -> String{
            \"{file_name}\".to_owned()
        }

        fn get_fields(&self) -> HashMap<&str, &String>{
            {get_fields_body}
        }

    }
";

#[macro_export]
macro_rules! include_templates {
    () => {
        include!(concat!(env!("OUT_DIR"), "/templates.rs"));
    };
}

fn remove_extension(filename: &str) -> String {
    match filename.rfind('.') {
        Some(index) => filename[..index].to_owned(),
        None => filename.to_owned(),
    }
}

fn variable_to_field_name(variable: &String) -> String {
    let mut field_name = variable.replace(" ", "_");
    //TODO: handle all illegal characters
    field_name = field_name.replace(":", "_");
    field_name = format!("{}", AsSnakeCase(field_name));
    field_name
}

pub trait Filename {
    fn get_filename(&self) -> String;
    fn get_fields(&self) -> HashMap<&str, &String>;
}

pub trait Save {
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>;
}

impl<T: Filename> Save for T {
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        println!("Saving template {}", self.get_filename());
        //TODO: actually do save
        for (key, value) in self.get_fields() {
            println!("{}, will be replaced with {}", key, value);
        }

        // algoritme:
        // slett custom props
        // find-replace strengen, MEN: kun der den faktisk skal byttast ut. Korleis sjekke det? (sjekk opp antagelse om at strengen alltid er i doc'et?). Kva gjer ein visst ikkje? :S

        let mut file = File::create(path)?;
        file.write_all("blabla".as_bytes())?;
        Ok(())
    }
}

fn derive_type_name_from_filename(filename: OsString) -> Result<String, String> {
    if let Ok(file_name_string) = filename.into_string() {
        let mut type_name = remove_extension(file_name_string.as_str());
        type_name = type_name
            .trim_start_matches(|c: char| c.is_numeric())
            .to_string();
        type_name = format!("{}", AsPascalCase(type_name));
        return Ok(type_name);
    }
    Err("Could not convert filename to string".to_owned())
}

fn get_template_variables_map(doc: &Docx) -> HashMap<String, String> {
    doc.doc_props.custom.properties.clone()
}

fn get_template_variable_keys(map: &HashMap<String, String>) -> Vec<String> {
    map.keys().cloned().collect()
}

fn build_struct_fields(fields: &Vec<String>) -> String {
    let mut fields_string = String::new();

    for field in fields {
        let field_name = variable_to_field_name(field);
        fields_string.push_str(&format!("pub {}: String,\n", field_name));
    }

    fields_string
}

fn build_get_fields_body(map: HashMap<String, String>) -> String {
    let mut result: String = String::from("let mut map = HashMap::new();\n");

    for variable in map {
        let field_name = variable_to_field_name(&variable.0);
        let row = "map.insert(\"{value}\", &self.{replacement});\n";
        let mut modified_row = row.replace("{value}", variable.1.as_str());
        modified_row = modified_row.replace("{replacement}", &field_name.as_str());

        result.push_str(&modified_row);
    }

    result.push_str("map");
    result
}

pub fn generate_types(template_path: &str) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("templates.rs");
    let template_dir = Path::new(template_path);

    if let Ok(files) = fs::read_dir(template_dir) {
        let structs: Vec<String> = files
            .filter_map(Result::ok)
            .par_bridge()
            .filter_map(|dir_entry| {
                let file_path = dir_entry.path();
                let fmt = FileFormat::from_file(&file_path).ok()?;

                // if not docx, skip file
                if fmt.extension() != "docx" {
                    return None;
                }

                let mut file = File::open(&file_path).ok()?;
                let mut buf = vec![];
                let _ = file.read_to_end(&mut buf);
                let doc = read_docx(&buf).ok()?;

                let type_name = derive_type_name_from_filename(dir_entry.file_name()).ok()?;
                let template_variables = get_template_variables_map(&doc);
                let template_keys = get_template_variable_keys(&template_variables);
                let fields_string = build_struct_fields(&template_keys);
                let get_fields_body = build_get_fields_body(template_variables);

                let mut formatted_string = TYPE_TEMPLATE.replace("{name}", type_name.as_str());

                formatted_string = formatted_string.replace("{fields}", fields_string.as_str());
                formatted_string =
                    formatted_string.replace("{file_name}", file_path.as_path().to_str().unwrap());
                formatted_string =
                    formatted_string.replace("{get_fields_body}", get_fields_body.as_str());

                Some(formatted_string)
            })
            .collect();

        let mut generated_types_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_path)
            .unwrap();

        generated_types_file
            .write_all(MODULE_PRELUDE.as_bytes())
            .unwrap();

        for s in structs {
            generated_types_file.write_all(s.as_bytes()).unwrap()
        }

        generated_types_file.write_all("\n}".as_bytes()).unwrap();
    }
}
