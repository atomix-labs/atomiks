# In its layout, with `format`; valid, and true to its schema where taplo.toml names one, with
# `lint`.

# Checks every TOML file.
check-toml:
    RUST_LOG=warn taplo fmt --check
    RUST_LOG=warn taplo lint

# Formats every TOML file.
fix-toml:
    RUST_LOG=warn taplo fmt
