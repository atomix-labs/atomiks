# Lints every Markdown file.
check-markdown:
    rumdl check .

# Fixes what rumdl can fix.
fix-markdown:
    rumdl check --fix .
