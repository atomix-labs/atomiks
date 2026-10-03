# Tooling: Gates, Lints, Rustdoc Mechanics

Read this before running the checks, when a lint fires on a doc or a comment,
and before a link rustdoc cannot resolve, a doctest attribute or a `cfg` a doc
depends on.

Contents: 1 Commands · 2 Workspace lints that shape docs · 3 Features and cfg
axes · 4 Links rustdoc cannot resolve · 5 Rustdoc and doctest attributes · 6
Rendered check · 7 Docs.rs

## 1 Commands

From the workspace root. `.claude/skills/writing-rustdoc/scripts/doc-audit.sh
<crate-dir>` runs the whole ladder and then the heuristic lint; the raw steps,
in order, each clean before the next:

```sh
# The formatters the workspace has: each rewraps, never tightens.
just fix-rust-fmt
just fix-dprint
just fix-toml
just fix-markdown
export CARGO_BUILD_TARGET=$(rustc -vV | sed -n 's/^host: //p')   # the host, as the recipes build
cargo fmt -p <crate> -- --check
RUSTDOCFLAGS="-D warnings" cargo doc -p <crate> --no-deps [--all-features] --document-private-items
cargo test -p <crate> --doc [--all-features]
cargo clippy -p <crate> --all-targets [--all-features] -- -D warnings
mise exec -- python3 .just/rust-doc.py <crate-dir>        # the cut list, mechanically
```

`--all-features` where the manifest has a `[features]` table. `-D warnings`
fails the build on any warning, as `just check-rust-doc` does. The recipes name
the host as the target, `CARGO_BUILD_TARGET`, so building the same way shares
their cache and writes the docs where they do, `target/<host>/doc/`.

The build documents private items, as `just check-rust-doc` does: it is where a
private doc's ``[`SLOTS`]`` link is checked, and it surfaces link hygiene a
public build never sees (a private `mod init` making ``[`init!`](crate::init)``
ambiguous, or an explicit target on a private field's link being redundant).

`just fix-rust-fmt fix-toml` format the whole tree; when only one crate changed,
`cargo fmt -p <crate>` and `taplo fmt <crate>/Cargo.toml` are the same two
formatters scoped down.

What `rustfmt` does to docs where `rustfmt.toml` sets `wrap_comments`,
`comment_width` and `format_code_in_doc_comments`, as it does on nightly: it
wraps a line over the width and pushes the tail down, which is where widows come
from; it never joins a short line to the next, so a paragraph rebalanced by hand
stays as written while every line is within the width; and it reformats the Rust
inside a doc fence, so write examples as `rustfmt` would.

For a crate the repository also builds under loom or Miri, its docs must also
compile on that axis, since a `cfg`-gated item a doc links may vanish there:

```sh
cargo clippy -p <crate> --lib --tests --config 'target."cfg(all())".rustflags=["--cfg","loom"]'
```

Snapshots a doc change can move: `TRYBUILD=overwrite cargo nextest run -p
<crate> --test trybuild` regenerates `tests/compile_fail/*.stderr`; read each
before it is committed, and never hand-edit one.

`just check-rust-clippy` and `just check-rust-doc`, and `just check` over
everything, are the whole-tree gates; run them before a PR, not per edit.

## 2 Workspace Lints That Shape Docs

Denied by the workspace's lint table, in the root `Cargo.toml`; every crate
inherits them with `[lints] workspace = true`. Do not add lint attributes to a
crate to satisfy this skill.

| lint                                                                   | fires on                                               | so                                                      |
| ---------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------------------- |
| `clippy::missing_errors_doc` / `missing_panics_doc` (pedantic)         | a public `Result` or `panic!` path without its section | the section standard                                    |
| `clippy::missing_safety_doc` (style)                                   | a public `unsafe fn` without `# Safety`                | the section standard                                    |
| `clippy::doc_markdown` (pedantic)                                      | an identifier or `CamelCase` word without backticks    | backticks on every identifier                           |
| `clippy::doc_lazy_continuation`, `doc_overindented_list_items` (style) | a list item's continuation line mis-indented           | indent continuation lines to the item's text            |
| `clippy::empty_line_after_doc_comments` (suspicious)                   | a blank line between a doc block and its item          | keep the doc attached                                   |
| `rustdoc::broken_intra_doc_links`                                      | ``[`Name`]`` that does not resolve; an ambiguous `Foo` | fix the path; disambiguate with `fn@`/`struct@`/`Foo()` |
| `rustdoc::private_intra_doc_links`                                     | a public doc linking a private item                    | link the public front door                              |
| `rustdoc::redundant_explicit_links`                                    | ``[`Foo`](Foo)``                                       | ``[`Foo`]``                                             |
| `rustdoc::bare_urls`                                                   | a URL not in `<…>` or a link                           | a reference definition at the bottom                    |
| `rustdoc::unescaped_backticks`, `invalid_html_tags`                    | a stray `` ` ``; `<T>` read as HTML                    | write `` `<T>` `` in backticks                          |
| `rustdoc::invalid_rust_codeblocks`, `invalid_codeblock_attributes`     | a fence that does not parse; a misspelled attribute    | fix the example; `text` for non-Rust                    |
| `unfulfilled_lint_expectations`                                        | an `#[expect]` whose lint no longer fires              | delete the `#[expect]`, or move it to where it fires    |

Under `strict`, the table denies these too:

| lint                                                                        | fires on                                                                                                                | so                                                                                                            |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `missing_docs`                                                              | an undocumented `pub` item, or a crate root with no `//!`, each file of `tests/` included                               | every public item has a summary line, and every crate root its `//!`                                          |
| `clippy::missing_docs_in_private_items` (restriction)                       | an undocumented private item, field or const of the crate's code; not one inside a `#[cfg(test)]` module                | private items too, each line carrying information; a test's helpers and fixtures by review (`comments.md` §7) |
| `clippy::unnecessary_safety_doc` (restriction)                              | `# Safety` on a safe fn                                                                                                 | delete it                                                                                                     |
| `clippy::undocumented_unsafe_blocks` (restriction)                          | `unsafe {}` or `unsafe impl` without `// SAFETY:` above it (above the statement or above its attributes, both accepted) | `comments.md` §1: above the attributes                                                                        |
| `clippy::unnecessary_safety_comment` (restriction)                          | `// SAFETY:` on safe code                                                                                               | delete it                                                                                                     |
| `clippy::multiple_unsafe_ops_per_block` (restriction)                       | two `unsafe` ops in one block                                                                                           | split; one comment per operation                                                                              |
| `clippy::too_long_first_doc_paragraph` (nursery)                            | a first paragraph over 200 rendered characters on an exported item; impl members and private items are exempt           | one sentence, blank `///`, then the body; the house asks this everywhere, the lint only gates the surface     |
| `clippy::allow_attributes`, `allow_attributes_without_reason` (restriction) | `#[allow]`, or an `#[expect]` without a reason                                                                          | `#[expect(lint, reason = "…")]` only                                                                          |
| `clippy::missing_assert_message` (restriction)                              | `assert!` without a message                                                                                             | a fragment naming the property (`comments.md` §6)                                                             |

## 3 Features and Cfg Axes

Features are explained once, in the crate docs' `# Crate Features` table; never
in item prose (the item is either there or not for the reader's build).

`cargo hack --each-feature clippy` (`just nightly-cargo-hack`) is the nightly
per-feature pass.

An item behind `#[cfg(target_os = …)]` is named in plain backticks on shared
surfaces; an intra-doc link to it breaks on the other target. An item behind
`cfg(loom)`/`cfg(miri)` likewise.

A published crate's docs are built by docs.rs, with its default features unless
the manifest says otherwise: §7.

## 4 Links Rustdoc Cannot Resolve

| target                                       | write                                                                                                                 |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| an item of this crate or a normal dependency | ``[`Name`]``, ``[`m`](Self::m)``, ``[`Layout`](core::alloc::Layout)``, by the crate's path                            |
| a path used several times in one block       | a reference definition: ``[`Shared`]: crate::Shared``                                                                 |
| an unstable / impl-restricted std item       | URL: ``[`AtomicPrimitive`]: https://doc.rust-lang.org/std/sync/atomic/trait.AtomicPrimitive.html``                    |
| a third-party crate or one of its items      | URL: `[loom]: https://docs.rs/loom`, ``[`UnsafeCell`]: https://docs.rs/loom/latest/loom/cell/struct.UnsafeCell.html`` |
| an issue, RFC, paper                         | URL with a stable short label: `[rust#125632]: https://github.com/rust-lang/rust/issues/125632`                       |
| a `cfg`-gated variant on a shared surface    | plain `` `Variant` ``, no link                                                                                        |

A ``[`Foo`]`` with a matching reference definition uses the URL and skips
intra-doc resolution, so it links cleanly and stays code-formatted. Never demote
an unresolvable link to a bare code span when a page for it exists.

A crate of the workspace that is not a normal dependency is not linked, and not
named: a relative HTML path to its pages breaks on docs.rs and wherever the item
is inlined, and rustdoc cannot check it. A published crate outside the workspace
may be linked by its docs.rs URL, as a third-party crate is.

Under `strict`, the doc lint refuses a crate of the workspace named where it is
neither this crate, its family nor a dependency.

A name shared by a private module and a public macro or fn (`mod init` and
`init!`) is ambiguous only when private items are documented, which `just
check-rust-doc` and the audit both do: write the disambiguator from the start,
``[`init!`](crate::init!)``, ``[`foo`](fn@crate::foo)``,
``[`init`](mod@crate::init)``.

## 5 Rustdoc and Doctest Attributes

| attribute / fence           | use here                                                                                                                                             |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `#[doc(hidden)]`            | plumbing a macro needs `pub` (`pub use ::pin_init as __pin_init`), a test-only module behind a feature; no doc on it, the `__` name says "not yours" |
| `pub(crate)` / `pub(super)` | the first tool for keeping an item off the surface; `#[doc(hidden)]` only for what must be `pub`                                                     |
| `#[doc(inline)]`            | a `pub use` of the crate's own item so it lists with its siblings; never on a third-party re-export                                                  |
| `#[doc(alias = "…")]`       | a name readers search for that the item is not called (`alias = "memset"`)                                                                           |
| ` ``` `                     | compiled and run: the default for every example                                                                                                      |
| `no_run`                    | compiled, not run: I/O, a process, a mapping                                                                                                         |
| `should_panic`              | documents a `# Panics` condition                                                                                                                     |
| `compile_fail`              | a type-level guarantee shown in docs; prefer a `trybuild` fixture for the error text                                                                 |
| `text`                      | not Rust: diagrams, listings, invocations                                                                                                            |
| `ignore`                    | never                                                                                                                                                |
| `# ` line prefix            | compiled, hidden: `# Ok::<(), E>(())`, `# install(…)?;`, `# let _ = boxed;`                                                                          |
| `#![feature(…)]` first line | when the example calls an unstable API (`allocator_api`); never for a lint                                                                           |

## 6 Rendered Check

The audit's build and `just check-rust-doc`'s land at
`target/<host>/doc/<crate_snake>/index.html`, the path `cargo doc` prints after
`Generated`; the recipe's holds every crate of the workspace. Read the crate
page top to bottom in the HTML (without a browser, `sed 's/<[^>]*>//g'` over the
file) and check:

1. The crate page reads as an introduction; the sidebar shows the task headings.
2. Every ``[`Name`]`` became a link; no literal `` [` `` survives (``grep -c
   '\[`' index.html``). A link into a sibling workspace crate renders as a link
   whose target is the bare path (`href="tiles_geometry::span"`), dead until
   that crate is documented beside it (`cargo doc --workspace` documents them
   all); that is not a defect of the doc, and no lint fires for it.
3. Diagrams and listings sit in `text` fences, aligned; table pipes align in the
   source.
4. Each summary is complete in the module listing: one sentence, no trailing
   fragment.
5. No heading is followed by an empty paragraph; no `# Example` singular.

## 7 Docs.rs

A crate published to crates.io has its docs built by docs.rs, on a recent
nightly, for its default features alone, with a `docsrs` cfg set on the crate
being documented and on none of its dependencies. So a published crate:

- **Builds every item it documents**: where a feature gates an item, the
  manifest says `all-features = true` under `[package.metadata.docs.rs]`, or
  `features = […]` naming those docs.rs can build.
- **Marks what a feature gates**: `#![cfg_attr(docsrs, feature(doc_cfg))]` in
  `lib.rs` shows, on the nightly the profiles pin, an "Available on crate
  feature … only" badge on each gated item; an item whose badge must say more
  than its `cfg` adds `#[cfg_attr(docsrs, doc(cfg(feature = "…")))]`. Both build
  on stable, where nothing sets `docsrs`; a `doc(cfg)` outside `cfg_attr(docsrs,
  …)` does not, since it is unstable. `just check-rust-doc` sets no `docsrs`, so
  the badges are read by hand, on nightly: `RUSTDOCFLAGS="--cfg docsrs" cargo
  doc -p <crate> --all-features --no-deps`.
- **Links only what resolves there**: items of this crate and its normal
  dependencies, which docs.rs links to each dependency's own docs, and pages by
  URL (§4); never a relative HTML path.
- **Has its manifest's words**: a `description`, which crates.io refuses a crate
  without and shows beside it (`comments.md` §9), and a `repository`, the source
  link crates.io and docs.rs show.

The `[package.metadata.docs.rs]` table is written as `editing-cargo-manifests`
says.
