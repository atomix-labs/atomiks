#!/usr/bin/env bash
# The gate for one crate's docs: format check, rustdoc (private items too), doctests, clippy,
# then the mechanical cut-list pass. Every step must be clean; the first failure stops the run.
#
# Usage: doc-audit.sh <crate-dir> [--lint-only] [--advisory]
#   --lint-only   skip the cargo steps (fast reread loop)
#   --advisory    also print widows and wrapped summaries
set -euo pipefail

usage() { sed -n '2,7p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
case "${1:-}" in
    -h|--help) usage; exit 0 ;;
    "") usage >&2; exit 2 ;;
esac
[ -f "$1/Cargo.toml" ] || { echo "doc-audit: $1 has no Cargo.toml" >&2; exit 2; }
crate=$(cd "$1" && pwd)
shift
lint_only=false; advisory=()
for arg in "$@"; do
    case "$arg" in
        --lint-only) lint_only=true ;;
        --advisory) advisory=(--advisory) ;;
        *) echo "doc-audit: unknown argument $arg" >&2; exit 2 ;;
    esac
done
name=$(sed -n 's/^name *= *"\([^"]*\)".*/\1/p' "$crate/Cargo.toml" | head -1)
root=$(cd "$crate" && cargo locate-project --workspace --message-format plain | xargs dirname)
cd "$root"
# The host as the target, as the `just` recipes build, so the audit shares their cache and writes
# the docs where `just check-rust-doc` does: `target/<host>/doc/`.
export CARGO_BUILD_TARGET="${CARGO_BUILD_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"

if ! $lint_only; then
    features=()
    grep -q '^\[features\]' "$crate/Cargo.toml" && features=(--all-features)
    step() { echo "== $*"; "$@"; }
    step cargo fmt -p "$name" -- --check
    # As `just check-rust-doc` builds it: private items documented, and any warning fatal.
    RUSTDOCFLAGS="-D warnings" step cargo doc -p "$name" --no-deps ${features[@]+"${features[@]}"} \
        --document-private-items
    step cargo test -p "$name" --doc ${features[@]+"${features[@]}"}
    # Any warning fatal, a dead `#[expect]` included where the workspace only warns of one.
    step cargo clippy -p "$name" --all-targets ${features[@]+"${features[@]}"} -- -D warnings
fi
echo "== doc-lint"
python3 -B "$(git rev-parse --show-toplevel)/.just/rust-doc.py" "$crate" ${advisory[@]+"${advisory[@]}"}
