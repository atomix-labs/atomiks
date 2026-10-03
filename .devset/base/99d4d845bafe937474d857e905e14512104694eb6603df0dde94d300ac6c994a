# Lints every crate, target and feature, reporting every crate that fails; a warning fails.
[metadata("rust")]
check-rust-clippy:
    cargo clippy --workspace --all-targets --all-features --keep-going -- -D warnings

# Applies clippy's suggestions.
fix-rust-clippy:
    cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged
