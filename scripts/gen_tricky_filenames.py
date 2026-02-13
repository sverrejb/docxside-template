"""
Generate .docx files with filenames that stress the filename-to-type-name conversion.

Tests derive_type_name_from_filename() and heck's to_pascal_case():
- Filenames with hyphens, underscores, spaces
- Filenames starting with digits (invalid Rust ident)
- Filenames with special characters
- Very long filenames
- Single-character filenames

Some of these should be silently skipped by the library.
"""

from docx import Document

cases = [
    # (filename, description, should_work)
    ("my-template.docx", "hyphens → MyTemplate", True),
    ("my_template.docx", "underscores → MyTemplate (COLLIDES with my-template)", True),
    # ("MY TEMPLATE.docx", "spaces → MyTemplate (COLLIDES)", True),
    ("123_starts_with_digit.docx", "leading digit → invalid ident, should be skipped", False),
    ("_leading_underscore.docx", "leading underscore", True),
    ("a.docx", "single char → A", True),
    ("ALLCAPS.docx", "all caps → Allcaps", True),
    ("already-PascalCase.docx", "mixed case → AlreadyPascalCase", True),
]

for filename, desc, _ in cases:
    doc = Document()
    doc.add_paragraph(f"This is the {desc} test. Placeholder: {{TestField}}")
    path = f"test-crate/templates/{filename}"
    doc.save(path)
    print(f"Saved {path} — {desc}")
