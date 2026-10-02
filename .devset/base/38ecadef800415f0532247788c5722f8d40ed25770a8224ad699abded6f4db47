---
name: writing-rust
description: Use when writing, changing or reviewing Rust code, whether a crate, module, type, trait, function or error type; when choosing between an error and a panic, a newtype and an alias, a spec struct and a builder, a borrow and a clone; when a rustc or clippy lint fires or an `#[expect]` needs a reason. Covers errors, crate and module layout, naming, API design, ownership and lints. Not for unsafe code or atomics, which writing-unsafe-rust covers.
---

# Writing Rust

How Rust is written in this workspace: errors that are values, one type for each
question a verb can be asked; a crate whose surface is its root; names that say
what a call costs and how it refuses; signatures that make a wrong call fail to
compile; and lints answered where they fire. The rules below are the whole of
it, each with its reason. The references hold each rule's why, a bad and a good
example that compile under the workspace's lints, and what holds the rule.
Unsafe code, raw pointers and atomics are `writing-unsafe-rust`'s.

The examples leave their docs out to stay short, and compile with the lints that
ask for docs off; real code writes them: `# Errors` on a public function that
returns a `Result`, and `# Panics` on one that can panic.

Under `strict`, real code also documents every item.

## Rules

### Errors

1. **Return an error for anything a caller can cause, and panic only for a
   broken invariant**, since a library cannot know whether its caller can
   recover. A function that panics carries `#[track_caller]`, a `# Panics`
   section and an `expect` whose message states the invariant; a `debug_assert!`
   checks what the type's own code keeps, or an `unsafe fn`'s contract, never a
   safe function's input.
2. **One private `errors.rs` a crate, its types re-exported flat from
   `lib.rs`**, so one file says every way the crate refuses. A public module
   with refusals of its own has its own.
3. **One error type for each question a verb can be asked, `<Question>Error`**,
   so a signature says which refusals are possible: a unit struct for one cause,
   `Copy` and `Eq` where the payload allows, and no crate-wide `Error`.
4. **A message is `"<type words> error: <lowercase fragment>"`, its fields
   inline**, so a line read alone names its source. Context is a type, not a
   string, and fields read `want` and `held`, `need` and `have`, `at` and `len`.
5. **An error renders its cause or exposes it, never both**, since a reporter
   prints the whole chain: `#[error(transparent)]` with `#[from]` across a
   boundary, and `#[from]` only where the lower error means one thing, else a
   prefixed variant and `map_err`.
6. **A refused value goes back inside the error, and an impossible arm is
   `Infallible`**, so a refusal loses nothing and no caller handles what cannot
   happen. Whether a retry could succeed is in the type: `TryError`, or a
   variant documented as transient.
7. **Alias an error type, never `Result`**, so each signature names its error. A
   binary fails with a `BoxError`, and a one-off failure is
   `io::Error::other("…")`; nothing uses anyhow. `main` returns `Result<(),
   BoxError>` where only an operator reads a failure; a command a person runs
   prints the message and each cause and returns `ExitCode`, since an `Err` from
   `main` prints its `Debug`.
8. **A dropped error is named, `|_gone|`**, so a reader sees the drop was
   chosen.

### Layout

1. **`lib.rs` holds docs, attributes, `mod` lines, then `pub use`, and no items;
   modules are private, and the root re-exports each item by name**, so a type
   moves without breaking a caller. `pub mod` is for a namespace callers are
   meant to write.
2. **Inside a private module, what the crate does not export is `pub(crate)` or
   `pub(super)`**, so an item says how far it reaches.
3. **A module with children is `mod.rs`, and every module is a singular noun for
   what it holds**: `errors.rs`, `testing.rs` and `consts.rs` are fixed, what
   integration tests share is `tests/testing/mod.rs`, and none is `utils`,
   `helpers` or `common`.
4. **What a macro calls is `#[doc(hidden)] pub`, named with `__`**, so no caller
   takes it for the API.

### Names

1. **Names follow `references/naming.md`**: no `get_`; `as_`, `to_` and `into_`
   by cost; `try_` refuses, `_with` takes a closure, `_in` an allocator; `new`
   builds, `create` lays, `open` binds, `open_or_create` does either; `*Spec`
   for one call's parameters, `*Config` for a program's settings, `*Guard` and
   `*Error` by role, never `*Options` or `*Params`; an acronym is one word; a
   type parameter is one capital letter.
2. **A type's one value is an associated constant, `Pos::ORIGIN`, and a
   yes-or-no method starts `is_` or `has_`**, with `is_empty` beside every
   `len`.
3. **A collection's iterators are `iter` and `iter_mut`, each with its
   `IntoIterator` impl**, so `for` works on it as on a slice.
4. **One word for one thing**, in names, docs, messages and tests, so a second
   word never reads as a second thing.

### API

1. **A call that creates or opens something, with three parameters or more, or
   one a caller may leave at its usual value, takes a `*Spec` of public fields,
   built as a literal, not a builder**, so every field is written where it is
   used; one or two plain values stay arguments.
2. **A newtype has a private field and `const fn new` and `get`**, so two
   meanings of one primitive cannot be swapped; an alias only where the name is
   worth having and a second type is not.
3. **Check input once, at the boundary, with `FromStr` or `TryFrom` and a typed
   error**, so holding the type is the proof; `From` only for what cannot fail,
   and never `Into`.
4. **`Display` and `FromStr` agree on one spelling**, pinned by a round-trip
   test, so what a program prints a user can type.
5. **State lives in the type, as a marker parameter or a value spent by value**,
   so a wrong call does not compile; a trait callers must not implement is
   sealed.
6. **`#[must_use]` on a pure function, with a reason on a guard, one in an
   `Option` too, and never on a `Result`**, which has it already.
7. **Derive `Debug` always, and each common trait the type's meaning supports**;
   `Default` where one value is obvious, with `new` delegating to it; bounds on
   the `impl`, not the type.
8. **A public signature names its generics, and dispatch is static**, `dyn` only
   at an edge, a boxed error or a `&mut dyn fmt::Write`, so a caller can name
   each type and pays for no call it did not need.
9. **Enums and structs are exhaustive; `#[non_exhaustive]` goes only on a
   published crate's type that is meant to grow, judged type by type**, since it
   costs every caller its exhaustive match.

### Ownership

1. **Take a value only where it is kept, and pass a small `Copy` value by
   value**, a spec included, so a caller never gives up what the function only
   reads, and nothing that costs nothing to copy is read through a reference.
2. **Clone only what must be owned twice, and an `Arc` as
   `Arc::clone(&board)`**; `Arc` across threads, `Rc` within one; `Cow` where
   the input usually comes back unchanged.
3. **Write `'_` in a path, and `use<>` on a returned `impl Trait` that borrows
   nothing**, so a signature shows what it borrows.

### Lints

1. **Answer a lint by fixing the code; where it misreads, write `#[expect(lint,
   reason = "…")]` at the narrowest scope, never `#[allow]`**, since an
   `#[expect]` fails once its cause is gone, in every build it must hold in. The
   reason is one lowercase clause saying why the lint is wrong here.
2. **A crate inherits the lint table whole, `[lints] workspace = true`**: the
   table is devset's to manage, and never edited by hand.

How a managed key is changed is `using-devset`'s.

Under `strict`, the lints hold more:

- **No panics in library code**: `unwrap`, `expect`, `panic!`, indexing and
  their kin are refused, and a function that panics on a broken invariant names
  its reason in an `#[expect]`.
- **No `as`**: `From` for a widening, `TryFrom` for the rest.
- **Arithmetic says how it overflows**: `checked_`, `saturating_` or
  `wrapping_`, whichever it means.
- **A library is `no_std` first**, reaching for `core`, then `alloc`, then
  `std`, with `std` a feature that only adds.
- **Every item is documented**, private ones and fields included.
- **`print_stdout` only in a binary, with a reason**, and `dbg!` never.
- **A match names every variant**, never a wildcard on an enum the crate owns.

## Steps

Read each reference a step names, whole, before writing the code.

1. **Changing existing code**: read the module and its neighbours first, and
   match their names, error types and shape; a rule here that they break is
   named in the change's description, not fixed in passing.
2. **A new crate**: `references/layout.md` for `lib.rs` and its modules, and
   `references/errors.md` for its `errors.rs`. Its manifest has `[lints]
   workspace = true` and nothing else under `[lints]`.
3. **A new module**: `references/layout.md` and `references/naming.md`: private,
   `mod.rs` if it will have children, its items `pub(crate)` unless the root
   re-exports them by name.
4. **A new error type**: `references/errors.md`, whole. Name the question it
   answers; put it in the crate's `errors.rs` and re-export it; derive `Debug,
   Error, Clone, Copy, PartialEq, Eq` where the payload allows; write the
   message; choose `#[from]` or a prefix; hand a refused value back; pin
   impossible arms to `Infallible`; add a test that pins how it renders.
5. **A new public type or function**: `references/api-design.md`,
   `references/naming.md` and `references/ownership.md`: a spec, a newtype or a
   typestate; its derives; constructors and conversions by name; `#[must_use]`;
   borrowed parameters; named generics.
6. **A binary**: `references/errors.md`, for its `BoxError` and
   `io::Error::other`, and a `main` that returns `Result<(), BoxError>` where
   only an operator reads a failure, or, in a command a person runs, prints the
   message and each cause and returns `ExitCode`.
7. **A lint that fires**: find it in `references/lints.md` and write what it
   asks. Only where the lint misreads the code, `#[expect]` it at the narrowest
   scope with a reason; never edit the lint table or give a crate its own
   `[lints.clippy]`.
8. **Unsafe code, a raw pointer, an `unsafe impl` or an atomic**:
   `writing-unsafe-rust`, before the code is written.
9. **Before finishing**: the checks below, until they pass.

Every doc comment, `// SAFETY:` comment and `#[expect]` reason also follows
`writing-rustdoc`.

Every manifest follows `editing-cargo-manifests`.

Every test, its fixtures and its messages follow `writing-rust-tests`.

Code made faster, a benchmark, and a build profile follow
`tuning-rust-performance`.

## Checks

- `just check`: every check, as CI runs them, after `just fix`.
- `just check-rust-clippy`: clippy on every crate, target and feature, with
  warnings denied; tests may unwrap, expect, panic, print and index.
- `just fix-rust-clippy`: applies clippy's suggestions, each read after.
- `just check-rust-lints`: the two lints only nightly rustc has, on every crate,
  naming each that fails.

## What Not to Do

| Thought                                                  | Instead                                                                        |
| -------------------------------------------------------- | ------------------------------------------------------------------------------ |
| "`#[allow]`, just this once"                             | `#[expect(lint, reason = "…")]` at the site: it fails once the cause is gone.  |
| "A `String` error is enough here"                        | A type for the question, its facts as fields; in a binary, `io::Error::other`. |
| "`fn main() -> Result` reports the error"                | In a command a person runs, `ExitCode`, printing the message and each cause.   |
| "`anyhow` keeps the library simple"                      | thiserror types a caller can match; `anyhow` erases them.                      |
| "One `Error` enum for the whole crate"                   | One type for each question a verb can be asked.                                |
| "`#[error("load error: {0}")]` on a `#[from]`"           | `#[error(transparent)]`, or render the cause and expose nothing.               |
| "Clamp the index to the last square"                     | Refuse it with the index and the length; clamping hides the caller's mistake.  |
| "`get_cols()` reads clearly"                             | `cols()`.                                                                      |
| "A builder for these three fields"                       | A `*Spec` literal.                                                             |
| "`impl Trait` in this public argument"                   | A named generic, `fn fill_with<F: FnMut() -> u8>`.                             |
| "`pub` is simpler than `pub(crate)`"                     | `pub(crate)` inside a private module.                                          |
| "`.clone()` to quiet the borrow checker"                 | Shape the borrow: a shorter scope, a reference kept, a move.                   |
| "`#[non_exhaustive]`, for the future"                    | Exhaustive; only a published type meant to grow, judged one by one.            |
| "`[lints.clippy]` here, or a level changed in the table" | Cargo refuses the first, devset reports the second; `#[expect]` at the site.   |

Under `strict`, also:

| Thought                                 | Instead                                                                |
| --------------------------------------- | ---------------------------------------------------------------------- |
| "`unwrap()`, it cannot fail"            | `?` or `ok_or`; for a broken invariant, `expect` under an `#[expect]`. |
| "`as u16` is fine, it fits"             | `u16::try_from`, or `u32::from` for a widening.                        |
| "`row + 1` cannot overflow here"        | `checked_add`, `saturating_add` or `wrapping_add`, as it means.        |
| "`squares[at]`, the index is in bounds" | `squares.get(at)`, or an `#[expect]` whose reason names the bound.     |
| "A `println!` to show progress"         | Return it to the binary, which prints under an `#[expect]`.            |

## References

Read every reference a task touches before writing code, and read them again
after compaction: this body is the summary, and the examples are there.

- `references/errors.md`: before an error type, a public `Result`, a `?` that
  crosses a module, a panic, an `expect`, or a binary's `main`.
- `references/layout.md`: before a crate, a module, a file or a re-export.
- `references/naming.md`: before naming or renaming anything a caller sees.
- `references/api-design.md`: before a public type, trait, constructor,
  conversion or signature.
- `references/ownership.md`: before choosing how a value is taken or returned,
  and before a clone, an `Arc`, a `Cow` or a named lifetime.
- `references/lints.md`: when a lint fires, before an `#[expect]`, and before
  touching `[lints]` or a `clippy.toml`.
- `references/sources.md`: before citing a source for a rule, or adapting one.
