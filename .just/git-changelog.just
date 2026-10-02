# Checks the configuration renders the notes of what is not yet released.
check-git-changelog:
    #!/usr/bin/env bash
    set -euo pipefail
    git rev-parse -q --verify HEAD > /dev/null || exit 0
    git cliff --unreleased > /dev/null

# Writes CHANGELOG.md for v$RELEASE_VERSION.
release-git-changelog:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${RELEASE_VERSION:?set it: RELEASE_VERSION=x.y.z just release}"
    git cliff --tag "v$RELEASE_VERSION" --output CHANGELOG.md
