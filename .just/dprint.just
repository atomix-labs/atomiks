# Checks that every Markdown, JSON and YAML file is formatted.
check-dprint:
    dprint check

# Formats every Markdown, JSON and YAML file.
fix-dprint:
    dprint fmt
