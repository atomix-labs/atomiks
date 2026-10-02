# A script under a vendor/ directory, or that .gitattributes marks `linguist-vendored` or
# `linguist-generated`, is someone else's.

# Lints every shell script the repository owns, following what each sources.
check-shell:
    #!/usr/bin/env bash
    set -euo pipefail
    files=()
    while IFS= read -r -d '' file; do [[ -f $file ]] && files+=("$file"); done \
        < <(git ls-files -z --cached --others --exclude-standard -- '*.sh' '*.bash' \
            ':(exclude,glob)**/vendor/**' \
            ':(exclude,attr:linguist-vendored)' ':(exclude,attr:linguist-vendored=true)' \
            ':(exclude,attr:linguist-generated)' ':(exclude,attr:linguist-generated=true)')
    (( ${#files[@]} )) || exit 0
    shellcheck -x "${files[@]}"
