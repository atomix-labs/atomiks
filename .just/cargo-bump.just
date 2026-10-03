# Moves every Cargo requirement and git revision past the cooldown, one at a time, and settles the locks.
bump-cargo-bump:
    mise exec -- python3 .just/cargo-bump.py "${BUMP_REPORT_DIR:+$BUMP_REPORT_DIR/cargo.md}"

# Sets every crate's version to $RELEASE_VERSION, and the lock with them.
release-cargo-bump:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${RELEASE_VERSION:?set it: RELEASE_VERSION=x.y.z just release}"
    cargo set-version --workspace "$RELEASE_VERSION"
    cargo update --workspace
