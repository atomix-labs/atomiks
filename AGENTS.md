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
the cell and the loom seam) and the ranged integers, each on a pattern-type
field; the facade `atomiks` re-exports both. `atomiks-core`'s `src/interop/`
holds the integrations with other crates, a file per crate, each behind its
feature; zerocopy's are derives on the types themselves.
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
  `tests/codegen.rs`. Each takes core's name and meaning, `fetch_add` to
  `try_update`, though a field's returns its container before, not the field's
  value. A form that discards what it returns exists only where the `fetch_`
  form is not one instruction on every target: `and`, `or`, `xor` and `not`, and
  a `bool` field's `set`, `clear` and `toggle`; the forms that return it are
  `#[must_use]`, naming the short one, and no other is, as core's are not. So
  there is no `add`, `sub`, `max` or `min`: a dropped `fetch_add` is `lock add`
  already.
- Every packed struct the derive takes gets a projection, `QuoteFields<'a, P>`
  beside `Quote`, of its visibility: one `&'a AtomicField` per field, of the
  field's visibility and with its docs, and a tuple struct's a tuple struct.
  Beside it go each field's hidden `HasPackedField` and the `ProjectFields`
  impl, and nothing else names a field: no constant per field, no `impl Quote`.
  The projection is the one way to a field's place, since a private field may
  carry an invariant its module's unsafe code relies on: a path is a type alone,
  and the hidden `project_field` that builds each place is `unsafe`.
- A pointer word, a struct of pointers beside tags, and a pointer enum, an enum
  some of whose variants hold a pointer, keep their tags in the low bits a
  pointee's alignment leaves clear, or, a pointer word of two words, in an
  integer word beside its pointer. In one word, they keep the pointer's
  provenance strictly: an address changes only through `wrapping_byte_add`,
  `wrapping_byte_sub`, `map_addr`, `mask`, or `AtomicPtr`'s `fetch_or`,
  `fetch_and` and `fetch_xor`; one that is no pointer's, a unit's or a value's,
  is `without_provenance`; and no integer is cast to a pointer, nor a provenance
  exposed. Two words, a `DoubleWord`, are the one place atomiks exposes a
  provenance, in its cell alone, since no Rust operation keeps one through a
  16-byte atomic: each pointer stored is exposed, and each loaded takes an
  exposed provenance back. Its cell lends no place, having no `RawAccess`, so
  its atomic operations alone reach it. Under Miri the cell is a lock around
  plain copies, which keeps each pointer's own provenance, so strict-provenance
  Miri checks every pointer those operations read back, though not their
  orderings, which the lock makes stronger. A pointer word projects as a packed
  struct does, its pointer a place whose `load` reads it through the word.
  Neither's `Validity`, `REPRS` or `TAG_WIDTH`, nor the layout they come from,
  one word or two, reads a pointee's alignment, since a type's layout may read
  them: only code and the checks read `POINTEE_ALIGNMENT`, so a node can hold an
  atomic of the word that points to it.
- A function that can be `const` is, and a trait whose impls can be is a `const
  trait`.
- Each `unsafe` block sits under an `#[expect(unsafe_code, reason = "…")]` with
  a `// SAFETY:` comment that proves what it requires; a field a proof relies on
  states its `// INVARIANT:` and names every writer.
- An integration is off under loom only where it reads an atomic's memory as
  bytes, as zerocopy's derives and bytemuck's `Zeroable` for an atomic do.
- An integration's feature names its dependency's version where that dependency
  is a 0.x crate, `zerocopy-08` and `deranged-05` as t2t's `chrono-04`, so the
  next 0.y can sit beside it; a 1.x or later dependency's feature is its name:
  `serde`, `bytemuck`, `arbitrary`, `arbitrary-int`. `loom`, the model-checking
  seam rather than an integration, keeps its name.
- A nightly feature is taken where it makes the API right, as `ptr_metadata`
  lets one `Atom` impl of a pointer choose its repr by its pointee's metadata,
  never to reach core's internals beyond four: the 128-bit intrinsics; a ranged
  integer's pattern-type field, which `transmute_neo` alone converts and a plain
  integer replaces should a nightly break it; `DynMetadata`'s layout, its vtable
  pointer, which `transmute` reads, as core's own `vtable_ptr` does, and writes
  back, since the compiler hard-codes it: `transmute` refuses a change of its
  size, and `crates/atomiks-core/tests/double_words.rs` calls through a trait
  object's pointer read back; and `const_eval_select`, whose four callers read
  in a constant no address of a pointer with provenance, three null's alone and
  `exposed_address` that of a pointer made of an integer, a tagged null, by a
  `transmute` a constant refuses of any other, and at run time do what no
  constant can: `address` reads a pointer's address with `addr`;
  `exposed_address`, a double word's, exposes its provenance with
  `expose_provenance`; `clear_tags`, a word's decode, clears its tag bits with
  `ptr_mask`'s `mask`, which tells LLVM they are clear; and `subtract_tags`, a
  pointer enum's, offsets the pointer back by its tag, which a match's arm
  knows, and tells LLVM through `assert_unchecked` that the bits are clear.
  `generic_const_exprs`, `specialization` and `unsafe_fields` stay out.

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

- A change to unsafe code, a primitive or a cell runs `just miri`, Miri on
  aarch64 Linux and macOS, x86_64 and x86-64-v2, which CI runs only each night.
- A change to an ordering, a fence or a cell adds or updates its model in
  `crates/atomiks-core/tests/model.rs`, which `just check-loom` runs.
- What the types refuse has a fixture in
  `crates/atomiks-core/tests/compile_fail/`, what aarch64 Linux's floor alone
  refuses one under its `aarch64_without_lse2/`, and what the derive refuses one
  in `crates/atomiks/tests/compile_fail/`, under its `aarch64/` where `x86_64`
  adds notes. What x86_64 without `cmpxchg16b` refuses, which trybuild's floor
  cannot build, the codegen fixtures' `x86-64-refused` probes pin. A new
  toolchain may reword a message; `TRYBUILD=overwrite cargo test -p atomiks-core
  --test trybuild`, and `TRYBUILD=overwrite cargo test -p atomiks --features
  derive --test compiled trybuild` for the derive's, write it again, to be read
  before it is committed.
- The checks of a value's repr, validity and decodes, the `Atom` laws, and the
  reading of a codegen fixture's assembly live once in
  `crates/atomiks-core/tests/testing/`, which the facade's tests reach by path:
  `crates/atomiks/tests/derive_laws.rs` holds each derived shape to the laws a
  built-in keeps.

<!-- >>> devset: cargo-deny >>> -->

## Dependencies

`just check-cargo-deny` holds every dependency, with every feature on, to
`deny.toml`: its advisories, its licence, its source, and the bans. A failure
names a choice for the maintainer, between a newer version, another crate, and
an exception with its reason: ask before adding an exception or allowing another
licence. Each goes in a key devset leaves to the repository: `skip` in `[bans]`,
`ignore` in `[advisories]`, and `exceptions` in `[licenses]`, which allows a
licence for one crate.

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
