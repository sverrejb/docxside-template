# Embedded

Generates a `.docx` from a template with the template bytes baked into the binary at compile time via the `embed` feature. The binary is fully self-contained and does not need the template `.docx` on disk at runtime.

## Run

```bash
cargo run -p embedded
```

Output is written to `output/greeting.docx`.
