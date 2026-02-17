# Docxide Template - Type safe MS Word templates for Rust.

> [!WARNING]
> Work in progress. Subject to change. Do not assume this is ready for production.

`docxide-template` is a Rust crate for working with .docx / MS Word templates. It reads `.docx` template files, finds `{placeholder}` patterns in document text, and generates type-safe Rust structs with those placeholders as fields. The generated structs include a `save()` method that produces a new `.docx` with placeholders replaced by field values and a `to_bytes()` for outputting the raw bytes.

## Usage

```bash
cargo add docxide-template
```

Place your `.docx` templates in a folder (e.g. `templates/`), using `{PlaceholderName}` for variables.

Then invoke the macro:

```rust
use docxide_template::generate_templates;

generate_templates!("templates");

fn main() {
    // If templates/HelloWorld.docx contains {FirstName} and {Company}:
    let doc = HelloWorld {
        first_name: "Alice",
        company: "Acme Corp",
    };

    // Writes output/greeting.docx with placeholders replaced
    doc.save("output/greeting").unwrap();

    // Or outputs the filled template as bytes:
    doc.to_bytes()
}
```


Placeholders are converted to snake_case struct fields automatically:

| Placeholder in template | Struct field |
|------------------------|-------------|
| `{FirstName}` | `first_name` |
| `{last_name}` | `last_name` |
| `{middle-name}` | `middle_name` |
| `{companyName}` | `company_name` |
| `{USER_COUNTRY}` | `customer_country` |
| `{first name}` | `first_name` |
| `{ ZipCode }` | `zip_code` |
| `{ZIPCODE}` | `zipcode` |

> Note: all upper- or lower-caps without a separator (like `ZIPCODE`) can't be split into words — use `ZIP_CODE` or another format if you want it to become `zip_code`.



## Embedded templates

By default, `generate_templates!` reads template files from disk at runtime. If you want a fully self-contained binary with no runtime file dependencies, enable the `embed` feature:

```toml
[dependencies]
docxide-template = { version = "0.1.0", features = ["embed"] }
```

With `embed` enabled, template bytes are baked into the binary at compile time via `include_bytes!`. The same `generate_templates!` macro is used — no code changes needed.

## Examples

**Save to file** — fill a template and write it to disk:
```bash
cargo run -p save-to-file
```

**To bytes** — fill a template and get the `.docx` as `Vec<u8>` in memory, useful for piping into other processing steps:
```bash
cargo run -p to-bytes
```

**Embedded** — template bytes baked into the binary, no runtime file access needed:
```bash
cargo run -p embedded
```

See the [`examples/`](examples/) directory for source code.

## How it works

1. The proc macro scans the given directory for `.docx` files at compile time
2. Each file becomes a struct named after the filename (PascalCase)
3. `{placeholder}` patterns become struct fields (snake_case)
4. `save()` opens the original template, replaces all placeholders in the XML, and writes a new `.docx`

## License

MIT
