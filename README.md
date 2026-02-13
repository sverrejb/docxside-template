# **Docx**side Templates - Type safe MS Word templates

> [!WARNING]
> Work in progress. Subject to change.

`docxside-templates` is a Rust proc-macro crate for working with .docx / MS Word templates. It reads `.docx` template files, finds `{placeholder}` patterns in document text, and generates type-safe Rust structs with those placeholders as fields. The generated structs include a `save()` method that produces a new `.docx` with placeholders replaced by field values.

## Usage

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
docxside-templates = "0.1"
zip = "2"
```

Place your `.docx` templates in a folder (e.g. `templates/`), using `{PlaceholderName}` for variables. Then invoke the macro:

```rust
use docxside_templates::generate_templates;

generate_templates!("templates");

fn main() {
    // If templates/HelloWorld.docx contains {FirstName} and {Company}:
    let doc = HelloWorld::new("Alice", "Acme Corp");

    doc.save("output/greeting").unwrap();
    // Writes output/greeting.docx with placeholders replaced
}
```

## How it works

1. The proc macro scans the given directory for `.docx` files at compile time
2. Each file becomes a struct named after the filename (PascalCase)
3. `{placeholder}` patterns become struct fields (snake_case)
4. `save()` opens the original template, replaces all placeholders in the XML, and writes a new `.docx`

## License

MIT
