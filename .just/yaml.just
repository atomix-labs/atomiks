# A file .gitattributes marks `linguist-vendored` or `linguist-generated` is someone else's.

# Checks every YAML file the repository owns, tracked or new, for what no formatter fixes.
check-yaml:
    #!/usr/bin/env bash
    set -euo pipefail
    files=()
    while IFS= read -r -d '' file; do [[ -f $file ]] && files+=("$file"); done \
        < <(git ls-files -z --cached --others --exclude-standard -- '*.yml' '*.yaml' \
            ':(exclude,attr:linguist-vendored)' ':(exclude,attr:linguist-vendored=true)' \
            ':(exclude,attr:linguist-generated)' ':(exclude,attr:linguist-generated=true)')
    (( ${#files[@]} )) || exit 0
    yamllint --strict "${files[@]}"
