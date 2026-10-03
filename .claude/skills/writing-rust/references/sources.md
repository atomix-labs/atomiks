# Sources

Read this before a rule of this skill needs backing, before citing a source in a
doc or a review, and before adapting a rule from somewhere else. It says what
each source holds, which rule it backs, where this skill departs from it, and
the notice owed for the text it adapts. Cite one of these, or an equally primary
source; open the page before linking it.

## Canon

| source                                                                                                               | says                                                                                                                                                                                                                                                            | backs                                                           |
| -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| [Rust API Guidelines, Naming](https://rust-lang.github.io/api-guidelines/naming.html)                                | C-CASE (RFC 430; "acronyms and contractions of compound words count as one word"); C-CONV (`as_` free, `to_` expensive, `into_` variable); C-GETTER (no `get_`, and `get` only "when there is a single and obvious thing"); C-ITER, C-ITER-TY                   | `naming.md`                                                     |
| [Rust API Guidelines, Interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html)            | C-COMMON-TRAITS; C-CONV-TRAITS (`From` and `TryFrom` are implemented, `Into` and `TryInto` "should never be implemented"); C-GOOD-ERR (messages "lowercase without trailing punctuation, and typically concise")                                                | `api-design.md`, `errors.md`                                    |
| [Rust API Guidelines, Predictability](https://rust-lang.github.io/api-guidelines/predictability.html)                | C-CTOR (`new` for the primary constructor, `_with_` for secondary ones, `from_` for conversions)                                                                                                                                                                | `naming.md`                                                     |
| [Rust API Guidelines, Flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)                      | C-CALLER-CONTROL ("If a function requires ownership of an argument, it should take ownership of the argument rather than borrowing and cloning"); C-GENERIC                                                                                                     | `ownership.md`, `api-design.md`                                 |
| [Rust API Guidelines, Type safety](https://rust-lang.github.io/api-guidelines/type-safety.html)                      | C-NEWTYPE; C-CUSTOM-TYPE; C-BUILDER                                                                                                                                                                                                                             | `api-design.md`                                                 |
| [Rust API Guidelines, Future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)              | C-SEALED; C-STRUCT-PRIVATE; C-STRUCT-BOUNDS ("Adding a trait bound to a data structure is a breaking change")                                                                                                                                                   | `api-design.md`                                                 |
| [`core::error::Error`](https://doc.rust-lang.org/core/error/trait.Error.html)                                        | messages are "concise lowercase sentences without trailing punctuation"; a wrapped error "should be either returned by the outer error's `Error::source()`, or rendered by the outer error's `Display` implementation, but not both"                            | `errors.md`: the message form, and rendering a cause once       |
| [thiserror](https://docs.rs/thiserror)                                                                               | `#[error(transparent)]`; `#[from]` implies `#[source]`; a field named `source` is exposed as the source                                                                                                                                                         | `errors.md`                                                     |
| [`core::time::Duration`](https://doc.rust-lang.org/core/time/struct.Duration.html)                                   | a value built and read in its unit: `from_nanos` and `as_nanos`, `from_secs` and `as_secs`                                                                                                                                                                      | `api-design.md`, `naming.md`: a newtype's unit                  |
| [derive_more's `Display`](https://docs.rs/derive_more/latest/derive_more/derive.Display.html)                        | where the format "can be trivially substituted with a transparent delegation call to the inner type, then additional formatting parameters will work too"                                                                                                       | `api-design.md`: deriving, and its width                        |
| [arrayvec](https://docs.rs/arrayvec)                                                                                 | `ArrayString` and `ArrayVec`, a string and a vector of fixed capacity, held inline                                                                                                                                                                              | `api-design.md`: search first                                   |
| [powerfmt's `FormatterExt`](https://docs.rs/powerfmt/latest/powerfmt/ext/trait.FormatterExt.html)                    | `pad_with_width` pads "with the given width"; its source reads the formatter's fill and alignment, and no precision                                                                                                                                             | `api-design.md`: search first                                   |
| [itoa](https://docs.rs/itoa)                                                                                         | `Buffer::format` writes an integer's decimal digits and hands them back as a `&str`                                                                                                                                                                             | `api-design.md`: search first                                   |
| [cfg_aliases](https://docs.rs/cfg_aliases)                                                                           | `cfg_aliases!` in `build.rs` declares each alias; its source prints each alias's `cargo:rustc-check-cfg`                                                                                                                                                        | `layout.md`: one alias for a `cfg`                              |
| [The Rust Reference, Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)        | `pub(crate)`, `pub(super)` (the parent module, and so what it holds), `pub(in path)`                                                                                                                                                                            | `layout.md`                                                     |
| [The Rust Reference, Lint check attributes](https://doc.rust-lang.org/reference/attributes/diagnostics.html)         | `#[expect]` is fulfilled where `#[warn]` would emit, and `unfulfilled_lint_expectations` fires otherwise; every lint attribute takes a `reason`                                                                                                                 | `lints.md`                                                      |
| [Edition guide, RPIT lifetime capture](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html) | in Rust 2024 "in-scope generic lifetime parameters are unconditionally captured"; `use<..>` names fewer                                                                                                                                                         | `ownership.md`                                                  |
| [clippy's lint list](https://rust-lang.github.io/rust-clippy/master/index.html)                                      | each lint, its group and its reason                                                                                                                                                                                                                             | `lints.md`                                                      |
| [clippy's configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)                                   | `clippy.toml` sets options, not levels; `avoid-breaking-exported-api`, on by default, spares an exported item from `enum_variant_names`, `trivially_copy_pass_by_ref`, `large_types_passed_by_value`, `upper_case_acronyms`, `wrong_self_convention` and others | `lints.md`, and each rule held "by review" for an exported item |
| [rustc's lint listing](https://doc.rust-lang.org/rustc/lints/listing/index.html)                                     | the allowed-by-default lints the table turns on                                                                                                                                                                                                                 | `lints.md`                                                      |
| [rustdoc's lints](https://doc.rust-lang.org/rustdoc/lints.html)                                                      | the rustdoc lints the table denies                                                                                                                                                                                                                              | `lints.md`                                                      |
| [Cargo, the `[lints]` section](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section)            | `priority`: lower numbers are overridden by higher                                                                                                                                                                                                              | `lints.md`                                                      |

## Where the Workspace Departs

| a source says                                                                     | here                                                                                                                                                      |
| --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C-BUILDER: a builder for a complex value                                          | a `*Spec` with public fields, built as a literal, for a call that creates or opens something; a builder only for construction that is a sequence of steps |
| C-CTOR: `new` for the primary constructor; C-GETTER: `get`                        | a value of one number is built and read in its unit, `from_squares` and `as_squares`; `new` for parts no unit names, `get` for a value with no unit       |
| rust-skills `api-non-exhaustive`: `#[non_exhaustive]` on public enums and structs | exhaustive by default; `#[non_exhaustive]` only on a published crate's type meant to grow, judged type by type                                            |
| rust-skills `err-anyhow-app`: anyhow in an application                            | no anyhow anywhere: a binary fails with `Box<dyn Error + Send + Sync>`, and a one-off failure is `io::Error::other`                                       |
| rust-skills `err-context-chain`: `.context("…")` strings                          | context is a type with a `Display`                                                                                                                        |
| rust-skills `err-custom-type`: errors grouped by domain, a crate-wide `Error`     | one type for each question a verb can be asked; no type named `Error`                                                                                     |
| rust-skills `err-source-chain`: a `source` field beside a message                 | a cause is rendered or exposed, never both                                                                                                                |
| rust-skills `err-expect-bugs-only`: `expect` messages that start "BUG:"           | the message states the precondition, or why the call cannot fail                                                                                          |
| rust-skills `proj-prelude-module`: a prelude                                      | no prelude; a caller imports the names it uses                                                                                                            |
| a `Result` alias per crate, as `io::Result`                                       | an alias of an error type, never of `Result`                                                                                                              |

## Corrected Here

Claims of rust-skills that fail on the pinned toolchain or against the sources
above, and what this skill says instead:

- `mod_module_files` and `self_named_module_files` are swapped: it is
  `self_named_module_files` that asks for `mod.rs`.
- A crate cannot set its own lint levels beside `[lints] workspace = true`:
  Cargo refuses the manifest.
- A group set beside a lint of its own at one priority fails; the group takes
  `priority = -1`. `clippy.toml` cannot set a level at all.
- `pub(super)` reaches the parent module and everything inside it.
- `#[must_use]` on a function that returns a `Result` fails
  `clippy::double_must_use`; an inherent `to_string` fails
  `clippy::inherent_to_string`.
- A newtype over a `String` cannot derive `Copy`, and `&mut T` is not `Copy`.
- No attribute sets a field's `Default`; `#[default]` marks an enum's variant.
- A sealed trait is sealed whole: no caller implements any part of it.
- A `&Path` parameter takes a `&PathBuf`, not a `&str`.

## Adapted Text

Rules of this skill adapt rules of
[leonardomso/rust-skills](https://github.com/leonardomso/rust-skills), at
v1.0.0, the copy sockudo vendors, and at v1.5.1 for `TryFrom` over `as`,
explicit overflow and exhaustive matches, rewritten for edition 2024 and this
workspace, and checked by compiling: the lowercase message, thiserror in a
library, a `Result` over a panic, `expect` for a broken invariant, an error
dropped by name, `From` over `Into`, `#[must_use]` on builders and results,
newtypes, checking at the boundary, typestate, sealed traits, the common traits,
`Default`, generic bounds, the size assertion, `TryFrom` over `as`, explicit
overflow, exhaustive matches, slices over owned parameters, borrowing over
cloning, `Copy` by value, `Arc` and `Rc`, `Cow`, lifetime elision, `mod.rs`,
`pub(crate)` and `pub(super)`, flat re-exports, the naming rules and workspace
lints.

Both versions carry the same notice:

```text
MIT License

Copyright (c) 2025 Leonardo Maldonado

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
