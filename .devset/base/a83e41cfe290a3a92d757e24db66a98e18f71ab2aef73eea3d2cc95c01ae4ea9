# Their features come from -Zcrate-attr, so no source needs a #![feature], and every crate still
# builds on stable; a crate that enables them itself is no fault. The flags reach rustc through a
# `--config` entry every target matches, which cargo joins to the rustflags the workspace's
# configuration sets, its CPU floor among them; RUSTFLAGS would replace those. A RUSTFLAGS the
# caller sets, even empty, still replaces them, the entry included, so the flags join it instead.

# Checks the lints only nightly has, reporting every crate that fails.
[metadata("rust")]
check-rust-lints:
    #!/usr/bin/env bash
    set -euo pipefail
    flags=(
        '-Zcrate-attr=feature(non_exhaustive_omitted_patterns_lint)'
        '-Zcrate-attr=feature(strict_provenance_lints)'
        -Aduplicate_features
        -Dnon_exhaustive_omitted_patterns
        -Dimplicit_provenance_casts
    )
    if [[ -n ${RUSTFLAGS+set} ]]; then export RUSTFLAGS="$RUSTFLAGS ${flags[*]}"; fi
    listed=$(printf '"%s", ' "${flags[@]}")
    cargo --config "target.'cfg(all())'.rustflags = [${listed%, }]" \
        check --workspace --all-targets --all-features --locked --keep-going
