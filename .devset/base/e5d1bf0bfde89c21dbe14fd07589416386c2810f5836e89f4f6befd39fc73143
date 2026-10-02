# With `strict`, then the house's doc lint, over every crate.

# Builds every crate's documentation, private items included; a warning, broken links too, fails.
[metadata("rust")]
check-rust-doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --document-private-items
    mise exec -- python3 -B .just/rust-doc.py --workspace
