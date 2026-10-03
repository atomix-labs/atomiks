# The repository's own recipes go here, above the block the just profile writes.

# Under loom: the feature, and `--cfg loom` through a `--config` entry, which joins the CPU floor.
loom := '''--features loom --config 'target."cfg(all())".rustflags=["--cfg","loom"]' --target-dir target/loom'''

# Miri with strict provenance, isolation on, and proptest's cases cut to what Miri runs in time.
miri := "MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-isolation-error=warn-nobacktrace -Zmiri-env-forward=PROPTEST_CASES' PROPTEST_CASES=16"

# x86-64 goes through `RUSTFLAGS`, which replaces the floor. Only the models run: a doctest fails
# under loom, whose atomics exist only inside a model, with no `const` `new` and no `from_ptr`.

# Lints every crate under loom on aarch64, and on x86_64 with the floor and x86-64; runs the models.
[metadata("rust")]
check-loom:
    cargo clippy --workspace --all-targets {{ loom }} --target aarch64-unknown-linux-gnu -- -D warnings
    cargo clippy --workspace --all-targets {{ loom }} --target x86_64-unknown-linux-gnu -- -D warnings
    RUSTFLAGS='--cfg loom -C target-cpu=x86-64' cargo clippy --workspace --all-targets --features loom --target x86_64-unknown-linux-gnu --target-dir target/loom-x86-64 -- -D warnings
    cargo test -p atomiks-core {{ loom }} --test model

# x86_64's floor has AVX; x86-64-v2 has `cmpxchg16b` but no AVX; x86-64 has neither, so no 128-bit
# atomics. The last two go through `RUSTFLAGS`, which replaces the floor.

# Lints every crate for aarch64, and for x86_64 with the floor, x86-64-v2 and x86-64.
[metadata("rust")]
check-targets:
    cargo clippy --workspace --all-targets --all-features --target aarch64-unknown-linux-gnu -- -D warnings
    cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu -- -D warnings
    RUSTFLAGS='-C target-cpu=x86-64-v2' cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu --target-dir target/x86-64-v2 -- -D warnings
    RUSTFLAGS='-C target-cpu=x86-64' cargo clippy --workspace --all-targets --all-features --target x86_64-unknown-linux-gnu --target-dir target/x86-64 -- -D warnings

# Checks the codegen fixture's formatting, which `cargo fmt --all` misses: it is its own workspace.
[metadata("rust")]
check-codegen-fmt:
    rustfmt --check crates/atomiks-core/tests/codegen/src/lib.rs

# Formats the codegen fixture.
fix-codegen-fmt:
    rustfmt crates/atomiks-core/tests/codegen/src/lib.rs

# x86_64's floor has AVX; x86-64-v2's 128-bit load is a compare-exchange.

# Runs the tests under Miri on aarch64, and on x86_64 with the floor and x86-64-v2.
[metadata("rust")]
nightly-miri:
    rustup component add miri
    {{ miri }} cargo miri test --workspace --target aarch64-unknown-linux-gnu
    {{ miri }} cargo miri test --workspace --target x86_64-unknown-linux-gnu
    {{ miri }} RUSTFLAGS='-C target-cpu=x86-64-v2' cargo miri test --workspace --target x86_64-unknown-linux-gnu --target-dir target/miri-x86-64-v2

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
