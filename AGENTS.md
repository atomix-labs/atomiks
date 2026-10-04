# Working in `atomiks`

What an agent needs to work here: what the repository is, how to check a change,
and the rules a change keeps.

<!-- >>> devset: agents >>> -->

## Before You Commit

Run `just check`: CI runs the same checks, and names each that fails. `just fix`
fixes what a formatter or linter can, and `just --list` shows every recipe.

## Before You Finish

A change beyond a line or two is ready when its passes have run and `just check`
passes:

- `/humanize` on what the change says to a reader: its comments, docs, names and
  messages.
- `/review-rust` on a change to Rust code.
- `/review-names` on a change that adds, renames or changes what a public item
  does.
- `/review-security` on a change that reads input from outside the process.

Each runs in a fresh context and reports: `/humanize` edits, then says what it
changed and what it left, and a review says what it found. Fix what a pass
leaves or finds, or say why a finding does not hold.

## Managed Files

Profiles, applied by devset, manage some of the files here. `devset status`
names each, and whether a local change to it is kept or is drift; `devset
explain <file>` says which profile owns what in it. What a profile owns changes
with the profile, on `devset update`. Never edit `.devset/`.

<!-- <<< devset: agents <<< -->

## The Repository

A Cargo workspace of crates under `crates/`: `atomiks-core` holds the typed
atomic (`Atom`, `Atomic`, the orderings, validity, the primitives, the fences,
the cell and the loom seam), and the facade `atomiks` re-exports it.
`crates/atomiks/macros/derive`, `atomiks-derive`, is the proc macro of
`#[derive(Atom)]` and the capabilities' derives, and
`crates/atomiks/macros/derive-impl`, `atomiks-derive-impl`, its logic; the
derive is `atomiks`' `derive` feature. Each crate inherits its version, edition,
licence and lints from the root `Cargo.toml`, and builds for Linux and macOS, on
aarch64 and x86_64. `atomiks-lock` is yet to be written. The book is under
`docs/`. The toolchain, with the four targets, is the nightly
`rust-toolchain.toml` pins, which the crates need for their nightly features.
The CPU floor is `.cargo/config.toml`'s: x86-64-v3, LSE on aarch64 Linux, and
the M1 on macOS. The tools are the versions `.config/mise/` pins. The justfile's
top section holds the repository's own recipes: the loom models, the lints for
each target and CPU, the codegen fixture's format, and Miri.
`.github/workflows/platforms.yml` is the repository's own: the tests and the
loom models on arm64 Linux and on macOS, arm64 and x86_64, which `check.yml`, on
x86_64 Linux, cannot run.

## Rules

What a change here keeps, beyond what the checks hold it to.

### Code

- No better way is left: before code is written, std, the workspace's own
  helpers and the ecosystem are searched for what does it more neatly, and the
  most concise form that measures as fast is the one taken; a shape that repeats
  is one macro or helper.
- Imports, never paths: neither a body nor an attribute names `core::`,
  `crate::` or another crate's path. A doc link may.
- A cfg that repeats is one alias in `crates/atomiks-core/build.rs`, as `wide`
  is. rustdoc names an alias as it is written, so `lib.rs` hides each alias a
  public item uses from the badges, and a public type writes its condition out
  in a `doc(cfg)`.
- Every name is whole words, never a fragment such as `at`, `by` or `held`.
- The facade has no code of its own: it re-exports each item by name, and an
  item `atomiks-core` makes public is re-exported in the same change.
- An operation exists only where the target runs it without a compare-exchange
  loop, and a loop is `update`, by name; what each lowers to is pinned in
  `tests/codegen.rs`.
- A function that can be `const` is, and a trait whose impls can be is a `const
  trait`.
- Each `unsafe` block sits under an `#[expect(unsafe_code, reason = "…")]` with
  a `// SAFETY:` comment that proves what it requires; a field a proof relies on
  states its `// INVARIANT:` and names every writer.
- A nightly feature is taken where it makes the API right, never to reach core's
  internals beyond the 128-bit intrinsics; `generic_const_exprs`,
  `specialization` and `unsafe_fields` stay out.

### Docs

- Headings are in Title Case, `# Crate Features`, and a crate page's example
  sits under `# Examples`, as an item's does.
- An item's example in `atomiks-core` names `atomiks`, the crate a user depends
  on, through a hidden `# extern crate atomiks_core as atomiks;`: the facade's
  pages show it as it is written. A derive's example in `atomiks-derive` does
  too, and imports the derive with a hidden `# use atomiks_derive::Atom;`, so
  its doctests run every example the facade shows. Only `atomiks-core`'s own
  page names `atomiks_core`.
- Siblings are documented alike: every alias, validity and ordering has the same
  sections, and each feature's row reads the same on every page that lists it.

### Checks Beyond `just check`

- A change to unsafe code, a primitive or a cell runs `just nightly-miri`, Miri
  on aarch64 Linux and macOS, x86_64 and x86-64-v2, which CI runs only each
  night.
- A change to an ordering, a fence or a cell adds or updates its model in
  `crates/atomiks-core/tests/model.rs`, which `just check-loom` runs.
- What the types refuse has a fixture in
  `crates/atomiks-core/tests/compile_fail/`, what aarch64 Linux's floor alone
  refuses one under its `aarch64_without_lse2/`, and what the derive refuses one
  in `crates/atomiks/tests/compile_fail/`. A new toolchain may reword a message;
  `TRYBUILD=overwrite cargo test -p atomiks-core --test trybuild`, and
  `TRYBUILD=overwrite cargo test -p atomiks --features derive --test compiled
  trybuild` for the derive's, write it again, to be read before it is committed.
- The checks of a value's repr, validity and decodes, the `Atom` laws, and the
  reading of a codegen fixture's assembly live once in
  `crates/atomiks-core/tests/testing/`, which the facade's tests reach by path:
  `crates/atomiks/tests/derive_laws.rs` holds each derived shape to the laws a
  built-in keeps.

<!-- >>> devset: cargo-deny >>> -->

## Dependencies

`just check-cargo-deny` holds every dependency to `deny.toml`: its advisories,
its licence, its source, and the bans. A failure names a choice for the
maintainer, between a newer version, another crate, and an exception with its
reason: ask before adding an exception or allowing another licence.

<!-- <<< devset: cargo-deny <<< -->

<!-- >>> devset: git-commits >>> -->

## Commits

A pull request lands squashed, as one commit its title names: the title follows
Conventional Commits, `type(scope): subject`, the subject imperative and lower
case, with no closing period, since it is the line the changelog shows; the
workflow `title` checks it. Each commit on a branch keeps the same rules, which
`just check-git-commits` checks. A breaking change adds `!` after the scope, and
a footer that starts `BREAKING CHANGE:` and says what to do.

<!-- <<< devset: git-commits <<< -->

<!-- >>> devset: mdbook >>> -->

## The Book

`just check-mdbook` lints the book, builds it and runs its examples. Its pages
are Markdown under the `src/` of its directory, each listed in `SUMMARY.md`, and
a preview rebuilds on every save:

```sh
mdbook serve docs
```

<!-- <<< devset: mdbook <<< -->
