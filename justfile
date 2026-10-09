# The repository's own recipes go here, above the block the just profile writes.

# Under loom: `--cfg loom` through a `--config` entry, which joins the CPU floor, and a target
# directory of its own; each command names the features it builds.
loom := '''--config 'target."cfg(all())".rustflags=["--cfg","loom"]' --target-dir target/loom'''

# Miri with strict provenance, isolation on, and proptest's cases cut to what Miri runs in time.
miri := "MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-isolation-error=warn-nobacktrace -Zmiri-env-forward=PROPTEST_CASES' PROPTEST_CASES=16"

# The lints run on aarch64, and on x86_64 with the floor and with x86-64, which goes through
# `RUSTFLAGS`, replacing the floor. Only the models run: a doctest fails under loom, whose atomics
# exist only inside a model, with no `const` `new` and no `from_ptr`.

# Lints every crate under loom, with every feature on; runs the models, core's and the facade's.
[metadata("rust")]
check-loom:
    cargo clippy --workspace --all-targets --all-features {{ loom }} --target aarch64-unknown-linux-gnu -- -D warnings
    cargo clippy --workspace --all-targets --all-features {{ loom }} --target x86_64-unknown-linux-gnu -- -D warnings
    RUSTFLAGS='--cfg loom -C target-cpu=x86-64' cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu --target-dir target/loom-x86-64 -- -D warnings
    cargo test -p atomix-core --features loom {{ loom }} --test model
    cargo test -p atomix-rs --features derive,loom {{ loom }} --test model

# macOS's aarch64 floor has LSE2, which Linux's lacks. x86_64's floor has AVX; x86-64-v2 has
# `cmpxchg16b` but no AVX; x86-64 has neither, so no 128-bit atomics. The last two go through
# `RUSTFLAGS`, which replaces the floor. docs.rs builds the docs at x86-64, where an item of 128 bits
# or two words does not exist, so they build there too, each link resolving.

# Lints every crate for aarch64 and x86_64 on Linux, macOS and Windows, for arm64ec, and for x86_64
# with x86-64-v2 and x86-64; builds the docs for x86-64.
[metadata("rust")]
check-targets:
    cargo clippy --workspace --all-targets --all-features --target aarch64-unknown-linux-gnu -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target aarch64-apple-darwin -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target aarch64-pc-windows-msvc -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target arm64ec-pc-windows-msvc -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target x86_64-apple-darwin -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings
    RUSTFLAGS='-C target-cpu=x86-64-v2' cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu --target-dir target/x86-64-v2 -- -D warnings
    RUSTFLAGS='-C target-cpu=x86-64' cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu --target-dir target/x86-64 -- -D warnings
    RUSTFLAGS='-C target-cpu=x86-64' RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --document-private-items --target x86_64-unknown-linux-gnu --target-dir target/x86-64

# Checks the codegen fixtures' formatting, which `cargo fmt --all` misses: each is its own workspace.
[metadata("rust")]
check-codegen-fmt:
    rustfmt --check crates/atomix-core/tests/codegen/src/lib.rs crates/atomix/tests/codegen/src/lib.rs

# Formats the codegen fixtures.
fix-codegen-fmt:
    rustfmt crates/atomix-core/tests/codegen/src/lib.rs crates/atomix/tests/codegen/src/lib.rs

# Every target rustup ships at tier 1 or 2 that the build script admits, but those with no OS: each
# `aarch64` and `x86_64` one with 64-bit pointers, and `arm64ec`.
hosted_targets := "aarch64-apple-darwin aarch64-apple-ios aarch64-apple-ios-macabi aarch64-apple-ios-sim aarch64-apple-tvos aarch64-apple-tvos-sim aarch64-apple-visionos aarch64-apple-visionos-sim aarch64-apple-watchos aarch64-apple-watchos-sim aarch64-linux-android aarch64-pc-windows-gnullvm aarch64-pc-windows-msvc aarch64-unknown-freebsd aarch64-unknown-fuchsia aarch64-unknown-linux-gnu aarch64-unknown-linux-musl aarch64-unknown-linux-ohos aarch64-unknown-uefi arm64ec-pc-windows-msvc x86_64-apple-darwin x86_64-apple-ios x86_64-apple-ios-macabi x86_64-fortanix-unknown-sgx x86_64-linux-android x86_64-pc-solaris x86_64-pc-windows-gnu x86_64-pc-windows-gnullvm x86_64-pc-windows-msvc x86_64-unknown-freebsd x86_64-unknown-fuchsia x86_64-unknown-illumos x86_64-unknown-linux-gnu x86_64-unknown-linux-gnuasan x86_64-unknown-linux-gnumsan x86_64-unknown-linux-gnutsan x86_64-unknown-linux-musl x86_64-unknown-linux-ohos x86_64-unknown-netbsd x86_64-unknown-redox x86_64-unknown-uefi"

# And those with no OS, which have no `std` for `arbitrary`.
bare_metal_targets := "aarch64-unknown-none aarch64-unknown-none-softfloat x86_64-unknown-none"

# Each target builds at its own CPU, as a dependent builds it: an empty `RUSTFLAGS` replaces the
# floor. What differs between targets is their defaults, which change with the toolchain, not with
# atomix's code. `rust-toolchain.toml` cannot list every target for every runner, so the recipe
# adds them.

# Builds the facade, with every feature it can, and each codegen fixture, for every target.
[metadata("rust")]
nightly-targets:
    #!/usr/bin/env bash
    set -uo pipefail
    rustup target add {{ hosted_targets }} {{ bare_metal_targets }} || exit
    export RUSTFLAGS=
    # Each build runs whether one before it failed, and `--keep-going` builds every target of one.
    status=0
    cargo build -p atomix-rs --lib --all-features --locked --keep-going --target-dir target/targets {{ prepend("--target ", hosted_targets) }} || status=1
    cargo build -p atomix-rs --lib --features arbitrary-int,bytemuck,deranged-05,derive,serde,zerocopy-08 --locked --keep-going --target-dir target/targets {{ prepend("--target ", bare_metal_targets) }} || status=1
    for fixture in atomix-core atomix; do
        cargo build --release --keep-going --manifest-path "crates/$fixture/tests/codegen/Cargo.toml" --config "resolver.lockfile-path='target/targets/$fixture-codegen/Cargo.lock'" --target-dir target/targets {{ prepend("--target ", hosted_targets + " " + bare_metal_targets) }} || status=1
    done
    exit "$status"

# macOS's aarch64 floor has LSE2, whose 128-bit load is `ldp`; Linux's floor reads with a
# compare-exchange. x86_64's floor has AVX; x86-64-v2's 128-bit load is a compare-exchange. x86_64
# macOS builds as x86_64 Linux does. Windows does too, but runs through Miri's shims of Windows,
# which no other run reaches. Each target is a `nightly-*` recipe of its own, so the nightly runs
# them side by side.

# Runs every feature's tests under Miri on each target a `nightly-miri-*` recipe names.
miri: nightly-miri-aarch64-linux nightly-miri-aarch64-macos nightly-miri-x86-64 nightly-miri-x86-64-v2 nightly-miri-x86-64-windows

[metadata("rust")]
nightly-miri-aarch64-linux: (_miri "aarch64-unknown-linux-gnu")

[metadata("rust")]
nightly-miri-aarch64-macos: (_miri "aarch64-apple-darwin")

[metadata("rust")]
nightly-miri-x86-64: (_miri "x86_64-unknown-linux-gnu")

[metadata("rust")]
nightly-miri-x86-64-v2: (_miri "x86_64-unknown-linux-gnu" "x86-64-v2")

[metadata("rust")]
nightly-miri-x86-64-windows: (_miri "x86_64-pc-windows-msvc")

# Runs every feature's tests under Miri on `target`, built for `cpu` where one is named, in a target
# directory of its own.
_miri target cpu="":
    rustup component add miri
    {{ miri }}{{ if cpu == "" { "" } else { " RUSTFLAGS='-C target-cpu=" + cpu + "'" } }} cargo miri test --workspace --all-features --target {{ target }}{{ if cpu == "" { "" } else { " --target-dir target/miri-" + cpu } }}

# >>> devset: just >>>
# Each active profile's recipes.
import? '.just/agents.just'
import? '.just/cargo-bump.just'
import? '.just/cargo-deny.just'
import? '.just/cargo-hack.just'
import? '.just/cargo-manifest.just'
import? '.just/cargo-nextest.just'
import? '.just/cargo-profiles.just'
import? '.just/cargo-unused.just'
import? '.just/cargo-workspace.just'
import? '.just/devset.just'
import? '.just/dprint.just'
import? '.just/editorconfig.just'
import? '.just/git-attributes.just'
import? '.just/git-changelog.just'
import? '.just/git-commits.just'
import? '.just/git-ignore.just'
import? '.just/github-automation.just'
import? '.just/github-bump.just'
import? '.just/github-ci.just'
import? '.just/github-dependabot.just'
import? '.just/github-labels.just'
import? '.just/github-nightly.just'
import? '.just/github-templates.just'
import? '.just/github-watch.just'
import? '.just/github-workflow-lint.just'
import? '.just/just.just'
import? '.just/lychee.just'
import? '.just/markdown.just'
import? '.just/mdbook.just'
import? '.just/mdbook-tool.just'
import? '.just/mise.just'
import? '.just/project.just'
import? '.just/rust.just'
import? '.just/rust-clippy.just'
import? '.just/rust-doc.just'
import? '.just/rust-fmt.just'
import? '.just/rust-lints.just'
import? '.just/rust-toolchain.just'
import? '.just/setup.just'
import? '.just/shell.just'
import? '.just/spelling.just'
import? '.just/toml.just'
import? '.just/vscode.just'
import? '.just/yaml.just'

# Runs every `check-*` recipe, as CI does, and names each that fails.
check: (_each "check")

# Runs every `fix-*` recipe.
fix: (_each "fix")

# Runs every `bump-*` recipe: each moves what its profile pins, and reports to $BUMP_REPORT_DIR.
bump: (_each "bump")

# Runs every `nightly-*` recipe: the checks too slow for every change.
nightly: (_each "nightly")

# Runs every `test-*` recipe: the suites too slow for `just check`, which CI runs beside it.
test: (_each "test")

# Runs every `setup-*` recipe: what a checkout needs before it builds. `mise bootstrap` runs it.
setup: (_each "setup")

# Runs every `host-*` recipe: the machine's own setup, whose steps may ask for sudo.
host: (_each "host")

# Runs every `release-*` recipe for $RELEASE_VERSION: each writes what a release needs.
release: (_each "release")

# Runs every `package-*` recipe: what a release ships, built for this machine into dist/.
package: (_each "package")

# Runs every `publish-*` recipe: what a release puts in a registry, once the release is out.
publish: (_each "publish")

# Runs every recipe named `<verb>-*`, and names each that fails.
_each verb:
    #!/usr/bin/env bash
    set -uo pipefail
    failed=()
    for recipe in $(just --justfile '{{ justfile() }}' --summary); do
        [[ $recipe == {{ verb }}-* ]] || continue
        just --justfile '{{ justfile() }}' "$recipe" || failed+=("$recipe")
    done
    if (( ${#failed[@]} )); then
        echo "failed: ${failed[*]}" >&2
        exit 1
    fi

# <<< devset: just <<<
