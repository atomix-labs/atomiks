# The lint takes every page in SUMMARY.md, every include and anchor there, and every recipe the
# prose names. The workspace's API lands at /api, under a page
# grouping its crates. Then lychee checks every link
# of the built book, offline.

# Lints the book, builds it, and runs its Rust examples.
[metadata("rust")]
check-mdbook:
    #!/usr/bin/env bash
    set -euo pipefail
    mise exec -- python3 -B .just/mdbook.py lint docs
    # rustdoc's index page is nightly's, and the API lands in target/doc, not target/<triple>/doc,
    # where the book's preprocessor, building it again with these flags, looks for it.
    export RUSTDOCFLAGS="-Zunstable-options --enable-index-page"
    unset CARGO_BUILD_TARGET
    cargo doc --workspace --no-deps
    mdbook build docs
    # mdbook runs rustdoc from a temporary directory, where rust-toolchain.toml no longer applies.
    RUSTUP_TOOLCHAIN="$(rustup show active-toolchain | cut -d' ' -f1)" mdbook test docs
    # The preprocessor links the API into the book only where a page links into it.
    [[ -e docs/book/api ]] || cp -R target/doc docs/book/api
    find docs/book/api -maxdepth 1 -name '.*' -delete
    mise exec -- python3 -B .just/mdbook.py api docs/book/api
    # The pages alone, as a stylesheet names fallbacks, such as KaTeX's fonts, a browser never
    # fetches; rustdoc's own pages aside, and the 404 page, whose links resolve against the site's
    # URL, which only a server has.
    book="$PWD/docs/book"
    lychee --offline --no-progress --include-fragments --root-dir "$book" \
        --exclude-path "$book/api" --exclude-path "$book/404.html" "$book/**/*.html"
