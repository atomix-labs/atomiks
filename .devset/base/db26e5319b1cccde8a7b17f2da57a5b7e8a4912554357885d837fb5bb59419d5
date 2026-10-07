# The doctests name no target, as the book's build does: under one, cargo builds a proc-macro crate's
# doctests, and the crates they link, for the host with none of the target's flags, so a CPU floor in
# `.cargo/config.toml` reaches neither.

# Runs every test, then the doctests nextest leaves to cargo; under CI, with nextest's `ci` profile.
[metadata("rust")]
check-cargo-nextest:
    cargo nextest run --workspace --all-features --no-tests=pass --profile {{ if env("CI", "") == "" { "default" } else { "ci" } }}
    unset CARGO_BUILD_TARGET && cargo test --workspace --all-features --doc
