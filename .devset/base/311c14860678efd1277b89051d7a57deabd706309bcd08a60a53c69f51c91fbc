# With `strict`, then the house's doc lint, over every crate. Under a named target, cargo passes a
# proc-macro crate no rustdocflags, so the recipe names none, and its docs land in target/doc.
# `-D warnings` reaches rustdoc through a `--config` entry every target matches, which cargo joins
# to the rustdocflags the workspace's configuration sets, its CPU floor among them; RUSTDOCFLAGS
# would replace those. A RUSTDOCFLAGS the caller sets, even empty, still replaces them, the entry
# included, so the flag joins it instead.

# Builds every crate's documentation, private items included; a warning, broken links too, fails.
[metadata("rust")]
check-rust-doc:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n ${RUSTDOCFLAGS+set} ]]; then export RUSTDOCFLAGS="$RUSTDOCFLAGS -D warnings"; fi
    unset CARGO_BUILD_TARGET
    cargo --config "target.'cfg(all())'.rustdocflags = ['-D', 'warnings']" \
        doc --workspace --all-features --no-deps --document-private-items
    mise exec -- python3 -B .just/rust-doc.py --workspace
