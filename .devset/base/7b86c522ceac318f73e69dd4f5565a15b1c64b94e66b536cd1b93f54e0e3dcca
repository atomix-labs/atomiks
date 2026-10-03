#!/usr/bin/env bash
# Installs rustup where it is missing, with no toolchain of its own, then the toolchain
# rust-toolchain.toml names. mise runs it before it installs any tool, as a preinstall hook, and
# `just setup` runs it too.
set -euo pipefail

bin=${CARGO_HOME:-$HOME/.cargo}/bin
if ! command -v rustup > /dev/null && [[ ! -x $bin/rustup ]]; then
    curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain none --profile minimal --no-modify-path
fi
[[ -f rust-toolchain.toml ]] || exit 0
"$(command -v rustup || echo "$bin/rustup")" toolchain install
