# By each tool the profile's features turn on.

# Fails on a dependency no crate uses, and names each tool that found one.
[metadata("rust")]
check-cargo-unused:
    #!/usr/bin/env bash
    set -euo pipefail
    failed=()
    cargo machete || failed+=(cargo-machete)
    cargo shear || failed+=(cargo-shear)
    (( ${#failed[@]} == 0 )) || { echo "failed: ${failed[*]}" >&2; exit 1; }

# By each tool the profile's features turn on; where cargo-manifest is applied, it puts back the
# group markers a removal takes with it.

# Removes each dependency no crate uses.
fix-cargo-unused:
    cargo machete --fix
    cargo shear --fix
    just fix-cargo-manifest
