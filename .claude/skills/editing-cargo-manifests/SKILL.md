---
name: editing-cargo-manifests
description: Use when creating a crate, or writing, reviewing or cleaning any `Cargo.toml` in the workspace, the root's included; when adding, removing or re-pinning a dependency; when adding or reshaping a `[features]` table, or choosing between a feature, an optional dependency and a separate crate; when adding a proc-macro crate or declaring a `[[bench]]`, `[[bin]]`, `[[example]]` or `[lib]` target; when `std` reaches a `no_std` crate through a default; when the manifest check or the unused-dependency check reports a finding. Covers the manifest's shape, the root, dependencies, features, defaults and targets. Not for build profiles, which tuning-rust-performance covers. Not for the lint table, which writing-rust covers.
---

# Editing Cargo Manifests

How a crate's `Cargo.toml` is written in this workspace. A manifest is a
declaration: what the crate is, what it may reach, and what a consumer may
switch on; explanation lives in the crate's docs. taplo settles the layout, and
`.just/cargo-manifest.py` the text around the values; neither decides what
belongs in the crate, which is most of this skill. The rules below are the whole
of it, each with its reason; `references/sources.md` holds the Cargo
documentation behind each.

## Rules

### The Shape

1. **Tables in one order, each written only when it has something to say, but
   `[package]` and `[lints]`**, so a reader finds each where it always is:
   `[package]`, any `[package.metadata.*]`, `[features]`, `[lints]`, the
   targets, then `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`
   and each `[target.'cfg(…)'.dependencies]`.
2. **`[package]` reads `name`, `description`, then the keys the workspace sets,
   each `<key>.workspace = true`: `version`, `edition`, `rust-version`,
   `license`, `authors`, then `publish` where the workspace keeps its crates off
   crates.io, or `repository` where it publishes them**, so a crate says only
   its name and pitch, and one edit to the workspace moves every crate.
3. **Every crate has `[lints] workspace = true`, and nothing else under
   `[lints]`**, so no crate leaves the shared lints without anyone deciding it
   should.
4. **Every `[features]` table opens with `default`, even `default = []`**, the
   manifest's word that nothing else is on by default.
5. **No comment in a member but the group markers, `# external` then `#
   internal`, each once and bare, inside a dependency table**; a reason worth
   keeping goes in the crate's docs, a feature's in its `# Crate features` table
   and a target's in its file's `//!`.
6. **Nothing Cargo finds or defaults to by itself**: no `[lib]` or `[[bin]]`
   that restates `src/lib.rs` or `src/main.rs`, no `build = "build.rs"`, no
   `autobins` or its kin, and no `documentation`, since crates.io links docs.rs
   without it.

```toml
[package]
name                   = "tiles-paint"
description            = "brushes that paint tiles onto a board, one square at a time."
version.workspace      = true
edition.workspace      = true
rust-version.workspace = true
license.workspace      = true
authors.workspace      = true

[features]
default = []

serde = ["dep:serde", "tiles-geometry/serde"]

[lints]
workspace = true

[[bench]]
name    = "strokes"
harness = false

[dependencies]
# external
serde     = { workspace = true, optional = true }
thiserror = { workspace = true }
# internal
tiles-geometry = { workspace = true }

[dev-dependencies]
# external
divan = { workspace = true }
```

### Dependencies

1. **Every version lives in `[workspace.dependencies]`, external and internal
   alike, with its `default-features` and the features every member needs; a
   member writes `name = { workspace = true }`, adding at most `features` and
   `optional`, in that order**, so one edit moves every crate: those two are all
   Cargo documents for an inherited dependency, and the workspace's form uses no
   other. Cargo ignores a `version` there, with a warning.
2. **An internal crate is listed in `[workspace.dependencies]` with its `path`
   and `version`, under `# internal`, and every member inherits it**; the
   manifest check refuses a `path` to a listed crate, and one that leaves the
   member's own directory, but for a crate in a `macros/` directory, which may
   reach its siblings there.
3. **A proc-macro crate sits in its parent's `macros/`, beside a plain library
   holding its token logic, and each reaches the next by `path`**, since a
   `proc-macro = true` crate exports its macros alone and its logic is tested
   through the library: the parent reaches `macros/macro`, and the macro crate
   its sibling, `../macro-impl`, neither of them listed in
   `[workspace.dependencies]`.
4. **A dependency's default is turned off where it would reach a crate that does
   not want it: an external crate a `no_std` member uses has `default-features =
   false` on its workspace entry**, since a member's own cannot hold: Cargo
   1.98, the `rust-version` a new workspace declares, refuses it over a default
   the workspace entry leaves on, and where a newer Cargo takes it, feature
   unification turns the default back on for any build with a member that wants
   it.
5. **No dev-dependency depends back on the crate that names it**, since Cargo
   then builds a second copy of the crate, whose types are not the ones under
   test, and a test that mixes them fails with E0308.

### Features

1. **A feature is justified by exactly one of three things: a dependency not
   every user should pay for, a capability the target may not have, `std` or a
   syscall, or a hook only tests need**; a second way of doing the crate's one
   job is not a feature, and a switch that changes what the crate is makes a
   second crate.
2. **Features only add**, since Cargo unifies them across the whole build: a
   `no-std` or `disable-x` feature that one crate turns on takes something from
   every other, and two features that cannot both be on are a defect, so the
   crate splits. Name the thing, `std`, not the switch, `use-std`.
3. **`dep:` on every optional dependency, `serde = ["dep:serde"]`**, since
   without it Cargo makes the dependency's name a public feature, and swapping
   the crate a breaking change.
4. **A feature forwards a dependency's by name, after its `dep:` entries**, as
   `std = ["tiles-geometry/std"]`, or `serde?/std` where `serde` is optional,
   since a dependency's feature is on only where some crate turns it on.
5. **`default` stays small, `[]` unless the crate is no use without something**,
   since a consumer escapes a default only with `default-features = false`, and
   every crate on the way must remember to.
6. **Each feature is permanent**: removing one, or taking it out of `default`,
   breaks the users who rely on it, and each multiplies the builds a feature
   sweep runs.

### Std Is the Opt-In

Under `strict`, a library is `no_std` first:

1. **A library that can run without `std` writes `#![no_std]` unconditionally,
   never `cfg_attr`-ed on, and `extern crate alloc;` where it allocates**, so a
   use of `std` is gated on purpose and never compiles the day something unifies
   `std` on.
2. **`std` is a feature that only adds, and never in `default`**, forwarded to
   each dependency that has its own; a crate that is `std`-only by nature, a
   binary, a proc-macro crate or one that owns the syscalls, has no `std`
   feature at all.

### Targets

`src/lib.rs`, `src/main.rs`, `tests/*.rs`, `benches/*.rs` and `examples/*.rs`
are found without a table; one is written only for a key that is not its
default:

| written                                                                      | when                                                                                             |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `[lib] proc-macro = true`                                                    | a proc-macro crate                                                                               |
| `[[bench]] name`, `harness = false`                                          | a bench that owns its `main`, as divan's and criterion's do                                      |
| `[lib] bench = false`                                                        | such benches take flags that libtest's harness in the library refuses                            |
| `[[bin]] name`, `path`                                                       | a binary named other than its package, or than its file in `src/bin/`                            |
| `[[bin]] doc = false`                                                        | a binary named like a library of the workspace, whose rustdoc page would overwrite the library's |
| `required-features` on a `[[bin]]`, `[[example]]`, `[[test]]` or `[[bench]]` | a target that needs a feature the crate does not default to; it does nothing on `[lib]`          |

`doc = false` also takes the binary out of rustdoc's build, so no check sees a
broken link in its docs; clippy still reads them, and the doc lint its words.

### The Root

The workspace's own `Cargo.toml` holds what the members share. The rules above
are a member's: the root's comments, the ones the profiles write above their
keys among them, are its own.

1. **`[workspace]` finds its members by glob, `members = ["crates/*"]`, and sets
   `resolver = "3"`**, so a crate under `crates/` is a member without an edit,
   and a virtual workspace, which has no edition to infer it from, still
   resolves as edition 2024 does.
2. **`[workspace.package]` holds the keys members inherit**: `version`,
   `edition`, `rust-version`, `license`, `authors`, and `publish` or
   `repository`. Raising `rust-version` decides who can build the crates, so it
   is a change of its own.
3. **`[patch]` sits in the root alone**, since Cargo ignores one in a member,
   with a warning.
4. **A git dependency is pinned by `rev`**, so what it names moves only when the
   pin does.

`just check-cargo-deny` refuses a git dependency pinned otherwise, or from a
source `deny.toml`'s `allow-git` does not name.

rust-lints writes the lints of `[workspace.lints]`, all but `unexpected_cfgs`,
which is the repository's.

cargo-profiles writes the `[profile.*]` keys.

A key a profile writes changes through the profile: an edit by hand is drift,
which `devset status` reports, and `using-devset` says how it is changed.

## Steps

1. **A new crate**: its directory where the workspace's `members` finds it, its
   `Cargo.toml` in the shape above, and its entry in `[workspace.dependencies]`
   once another crate uses it; its `description` is the crate summary's pitch
   clause, as `writing-rustdoc` words it.
2. **A dependency**: its entry in `[workspace.dependencies]` under its group,
   with its version and `default-features`, then `{ workspace = true }` in the
   member, with `features` or `optional` where the member needs them. `cargo
   add` writes the dotted `name.workspace = true` where the entry exists, which
   Cargo and the check read as the inline form the workspace writes, and a
   pinned version where it does not: write the entry first.
3. **Re-pinning a dependency**: its version in its workspace entry, nowhere
   else; `just bump-cargo-bump` moves every requirement to its newest release
   past a three-day cooldown.
4. **A feature**: which of the three earns it; `dep:` for an optional
   dependency; the features it forwards; its row in the crate docs' `# Crate
   features` table.
5. **A proc-macro crate**: the pair under the parent's `macros/`, each reached
   by `path`, the macro crate's `[lib] proc-macro = true`.
6. **A target**: only a key that is not its default, from the table above.
7. **A finding**: `just fix` puts each dependency under its group; the rest is
   fixed by hand, as the finding names it.
8. **Before finishing**: `just fix`, then the checks below until they pass.

## Checks

- `just check-cargo-manifest`: the `[package]` keys, inherited and in their
  order; `description`; the comments and the two groups; every internal
  dependency inherited, and every `path` in its crate's tree; `default` first;
  and every crate on the workspace's lints. `mise exec -- python3
  .just/cargo-manifest.py <path>` checks one manifest. An external dependency
  pinned in a member, the order of the tables, a key Cargo would find by itself,
  and `documentation` are held by review.
- `just check-toml`: taplo's layout, each dependency group in alphabetical
  order.
- `just check-cargo-unused`: a dependency nothing uses, or one in the wrong
  table. Where one is reached only through a macro's expansion, both scanners
  are told, in `[package.metadata.cargo-machete]` and
  `[package.metadata.cargo-shear]`, `ignored = ["<name>"]`.
- `just check-cargo-deny`: advisories, licences, bans and sources.
- `just nightly-cargo-hack`: every feature builds alone.
- `just check`: all of them.
- By hand: `cargo tree -e features -i <crate>` says who turned a feature on
  across the workspace, and `cargo tree -e features -i <crate> -p <member>`
  within one member: Cargo unifies features across what one command builds, so a
  build of one member can lack a feature a build of all of them has.

## What Not to Do

| Thought                                                       | Instead                                                                |
| ------------------------------------------------------------- | ---------------------------------------------------------------------- |
| "This dependency's purpose is not obvious; a `#` gloss helps" | No comment: the crate docs say it, where its guarantee is used.        |
| "A paragraph above `[[bench]]` says why it has no harness"    | The bench's `//!` says it.                                             |
| "No feature is on by default, so `default` can go"            | `default = []` is the declaration that none is. Write it.              |
| "`default-features = false` here keeps `std` out"             | Cargo 1.98 refuses it, and unification undoes it: the workspace entry. |
| "A `no-std` feature reads clearer"                            | Features only add: name the thing, `std`.                              |
| "Two features that conflict; a `compile_error!` will say so"  | Split the crate: the `compile_error!` is the last resort.              |
| "An optional dependency's implicit feature is fine"           | `dep:`, so the crate's name is not a public feature.                   |
| "One more feature is small"                                   | It is permanent, unifies across the build, and multiplies the sweep.   |
| "Pin the version here; only this crate uses it"               | Until a second does: `[workspace.dependencies]`.                       |
| "`path = "../tiles-geometry"` is simpler"                     | `{ workspace = true }`; a `path` is for a crate inside this one.       |
| "`[lib] path = "src/lib.rs"`, to be explicit"                 | Cargo finds it: write only a key that is not the default.              |
| "Remove the workspace entry the check says is unused"         | Ask first whether a crate is about to use it.                          |
| "`taplo fmt` passed, so the manifest is clean"                | taplo never sees a comment: run the manifest check.                    |

## References

Read the reference before citing a source for a rule, or where Cargo's own
documentation seems to say otherwise, and again after compaction.

- `references/sources.md`: the Cargo documentation and tools behind each rule,
  what measurement on Cargo 1.98 and the pinned nightly shows of them, and what
  this workspace settles that the ecosystem contests.
