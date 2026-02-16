# **Docx**side Templates - Type safe MS Word templates

> [!WARNING]
> Work in progress. Subject to change. Do not assume this is ready for production.

`docxside-template` is a Rust crate for working with .docx / MS Word templates. It reads `.docx` template files, finds `{placeholder}` patterns in document text, and generates type-safe Rust structs with those placeholders as fields. The generated structs include a `save()` method that produces a new `.docx` with placeholders replaced by field values.

## Usage

```bash
cargo add docxside-template
```

Place your `.docx` templates in a folder (e.g. `templates/`), using `{PlaceholderName}` for variables. Placeholders are converted to snake_case struct fields automatically:

| Placeholder in template | Struct field |
|------------------------|-------------|
| `{FirstName}` | `first_name` |
| `{first_name}` | `first_name` |
| `{firstName}` | `first_name` |
| `{first-name}` | `first_name` |
| `{FIRST_NAME}` | `first_name` |
| `{first name}` | `first_name` |

Then invoke the macro:

```rust
use docxside_template::generate_templates;

generate_templates!("templates");

fn main() {
    // If templates/HelloWorld.docx contains {FirstName} and {Company}:
    let doc = HelloWorld::new("Alice", "Acme Corp");

    // Writes output/greeting.docx with placeholders replaced
    doc.save("output/greeting").unwrap();

    // Or outputs the filled template as bytes:
    doc.to_bytes()
}
```

## Examples

**Save to file** — fill a template and write it to disk:
```bash
cargo run -p save-to-file
```

**To bytes** — fill a template and get the `.docx` as `Vec<u8>` in memory, useful for piping into other processing steps:
```bash
cargo run -p to-bytes
```

See the [`examples/`](examples/) directory for source code.

## How it works

1. The proc macro scans the given directory for `.docx` files at compile time
2. Each file becomes a struct named after the filename (PascalCase)
3. `{placeholder}` patterns become struct fields (snake_case)
4. `save()` opens the original template, replaces all placeholders in the XML, and writes a new `.docx`

## License

MIT
