# **Docx**side Templates - Type safe MS Word templates

> [!WARNING]
> Work in progress. Subject to change.

`docxside-templates` is a Rust crate for working with .docx / MS Word templates. This crate allows you to generate Rust types based on your docx template files, making it simple to fill out and save templated documents programmatically.

## Templates?
.docx-files [using custom properties as variables](https://dradis.com/support/guides/word_reports/custom_properties.html) can be used as templates. Further down the road, this crate might support other methods of templating as well. Feel free to make an issue or a PR!

## Usage
This crate is centered around code generation. Add a `build.rs` file to the root of your project like so:

```rust
fn main() {
    docxside_templates::generate_types("./path/to/your/templates");
}

```

and include the generated types in your code:

```rust
use docxside_templates::{include_templates, Save};

include_templates!();
```