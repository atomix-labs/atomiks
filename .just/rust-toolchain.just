# Every cargo command names the host as its target, so RUSTFLAGS reach no host build script: a build
# script built under `target-cpu=native` can die of SIGILL (openssl-sys does). The host is the
# machine's, not rustc's, so a recipe that runs no Rust tool starts no toolchain.
export CARGO_BUILD_TARGET := arch() + if os() == "macos" { "-apple-darwin" } else if os() == "windows" { "-pc-windows-msvc" } else { "-unknown-linux-" + `ldd --version 2>&1 | grep -qi musl && echo musl || echo gnu` }

# Installs rustup where it is missing, then the toolchain rust-toolchain.toml names.
setup-rust-toolchain:
    bash .just/rust-toolchain.sh
