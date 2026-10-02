# Checks advisories, licences, bans and sources.
[metadata("rust")]
check-cargo-deny:
    cargo deny check

# Checks for advisories published since the last change, which no commit brings.
[metadata("rust")]
nightly-cargo-deny:
    cargo deny check advisories
