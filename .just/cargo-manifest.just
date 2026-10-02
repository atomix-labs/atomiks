# The shape is one taplo, reading values, cannot see.

# Holds every crate manifest to one shape, and fails where a crate does not inherit the lints.
[metadata("rust")]
check-cargo-manifest:
    mise exec -- python3 .just/cargo-manifest.py
    cargo workspace-lints

# Puts every dependency under its group, `# external` or `# internal`, the workspace's included.
fix-cargo-manifest:
    mise exec -- python3 .just/cargo-manifest.py --fix
