# Lints every crate with each feature alone; a warning fails.
[metadata("rust")]
nightly-cargo-hack:
    cargo hack --each-feature --workspace clippy --all-targets -- -D warnings
