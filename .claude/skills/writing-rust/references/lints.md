# Lints

Read this when a rustc, rustdoc or clippy lint fires, before writing an
`#[expect]`, and before touching a crate's `[lints]` or a `clippy.toml`. It says
how a lint is answered, which lints the workspace turns on beyond clippy's
defaults and why, and what to write instead of what each refuses.

The rust-lints profile writes the lint table, `[workspace.lints]`, into the
workspace `Cargo.toml`, and every crate inherits it. Nearly every lint names a
better spelling, and that is the first answer; an `#[expect]` is for the place a
lint misreads.

`just check-rust-clippy` passes `-D warnings`, so a lint at `warn` fails it too.

## Suppress at the Site with `#[expect]` and a Reason, Never `#[allow]`

`#[expect(lint, reason = "…")]` suppresses a lint where it fires, and fails the
build once the lint stops firing there, so a suppression cannot outlive its
cause. `#[allow]` stays silent after its cause is gone, and hides the next
finding at that site too. The one `#[allow]` a workspace has is where an
`#[expect]` cannot hold: a module shared by several test binaries, each of which
uses part of it, which says so once at its top: `#![allow(dead_code, reason =
"each test binary uses part of this module")]`.

```rust,compile_fail
// fails: clippy::allow_attributes
use std::time::Instant;

#[derive(Debug)]
pub struct Session {
    started: Instant,
}

impl Session {
    // Bad: silent for as long as the attribute stays, cause or no cause.
    #[must_use]
    #[allow(clippy::new_without_default, reason = "a session starts its clock when made")]
    pub fn new() -> Self {
        Self { started: Instant::now() }
    }

    #[must_use]
    pub const fn started(&self) -> Instant {
        self.started
    }
}
```

```rust
use std::time::Instant;

#[derive(Debug)]
pub struct Session {
    started: Instant,
}

impl Session {
    #[must_use]
    #[expect(
        clippy::new_without_default,
        reason = "a session starts its clock when made, which `Default::default()` would hide"
    )]
    pub fn new() -> Self {
        Self { started: Instant::now() }
    }

    #[must_use]
    pub const fn started(&self) -> Instant {
        self.started
    }
}
```

Held by `unfulfilled_lint_expectations`, which fails an `#[expect]` whose lint
no longer fires, and by review.

Under `strict`, `clippy::allow_attributes` refuses an outer `#[allow]` and
passes that inner `#![allow]`, and `clippy::allow_attributes_without_reason`
refuses a suppression with no `reason`.

## Delete an `#[expect]` Once Its Lint Stops Firing

A change that removes what a lint saw leaves its `#[expect]` unfulfilled, and
the build fails there: the suppression goes, not the lint's level.

```rust,compile_fail
// fails: unfulfilled_lint_expectations
// Bad: the labels are borrowed now, so nothing is passed by value.
#[must_use]
#[expect(clippy::needless_pass_by_value, reason = "the labels are read, then dropped")]
pub fn longest(labels: &[String]) -> usize {
    labels.iter().map(String::len).max().unwrap_or(0)
}
```

```rust
#[must_use]
pub fn longest(labels: &[String]) -> usize {
    labels.iter().map(String::len).max().unwrap_or(0)
}
```

Held by `unfulfilled_lint_expectations`.

## An `#[expect]` Holds in Every Build

The checks build every target with every feature: the library, its tests, its
examples, its benches. An `#[expect]` fulfilled in one of those builds and not
in another fails the one where its lint is quiet. An `#[expect(dead_code)]` on a
helper only tests call fails the test build, where the helper is used. An item
only tests use goes under `#[cfg(test)]`, in `testing.rs`; one that must exist
in every build carries the expectation only where it holds,
`#[cfg_attr(not(test), expect(dead_code, reason = "…"))]`, or `not(feature =
"…")` for a feature.

```rust,compile_fail
// fails: unfulfilled_lint_expectations
// Bad: dead in the library build, and used in the test build, which fails.
#[expect(dead_code, reason = "only the tests lay a blank grid")]
fn blank(squares: usize) -> Vec<u8> {
    vec![0; squares]
}

#[cfg(test)]
mod tests {
    use super::blank;

    #[test]
    fn a_blank_grid_holds_no_tiles() {
        assert!(blank(9).iter().all(|&tile| tile == 0), "every square empty");
    }
}
```

```rust
// src/testing.rs, under `#[cfg(test)] mod testing;`
#[cfg(test)]
mod testing {
    /// A grid of `squares` squares, every one blank.
    pub(crate) fn blank(squares: usize) -> Vec<u8> {
        vec![0; squares]
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::blank;

    #[test]
    fn a_blank_grid_holds_no_tiles() {
        assert!(blank(9).iter().all(|&tile| tile == 0), "every square empty");
    }
}
```

Held by `unfulfilled_lint_expectations`, in the build where the lint is quiet.

## The Reason Says Why the Lint Is Wrong Here

A reason is read by the next person to meet the suppression, who needs the fact
the lint cannot see: which bound puts an index in range, what a default would
hide. It is one lowercase clause with no final period, naming the concrete
thing, never the lint's name, never "clippy is wrong", never what would happen
without it.

```rust
use std::time::Instant;

#[derive(Debug)]
pub struct Timer {
    started: Instant,
}

impl Timer {
    // Bad: says nothing a reader could check.
    #[must_use]
    #[expect(clippy::new_without_default, reason = "needed")]
    pub fn new() -> Self {
        Self { started: Instant::now() }
    }

    #[must_use]
    pub const fn started(&self) -> Instant {
        self.started
    }
}
```

```rust
use std::time::Instant;

#[derive(Debug)]
pub struct Timer {
    started: Instant,
}

impl Timer {
    #[must_use]
    #[expect(
        clippy::new_without_default,
        reason = "a timer reads the clock when made, which a default would not say"
    )]
    pub fn new() -> Self {
        Self { started: Instant::now() }
    }

    #[must_use]
    pub const fn started(&self) -> Instant {
        self.started
    }
}
```

Held by review.

## Suppress at the Narrowest Scope

An `#[expect]` sits on the statement, expression or item the lint misreads, then
on its module, then on the crate: a wider one suppresses the lint for code its
reason does not cover. A whole crate or file is the scope only where the reason
is true of all of it.

```rust
#![expect(clippy::new_without_default, reason = "a session starts its clock when made")]
// Bad: the whole crate is excused, for the one `new` the reason is about.

use std::time::Instant;

#[derive(Debug)]
pub struct Session {
    started: Instant,
}

impl Session {
    #[must_use]
    pub fn new() -> Self {
        Self { started: Instant::now() }
    }

    #[must_use]
    pub const fn started(&self) -> Instant {
        self.started
    }
}
```

```rust
use std::time::Instant;

#[derive(Debug)]
pub struct Session {
    started: Instant,
}

impl Session {
    #[must_use]
    #[expect(clippy::new_without_default, reason = "a session starts its clock when made")]
    pub fn new() -> Self {
        Self { started: Instant::now() }
    }

    #[must_use]
    pub const fn started(&self) -> Instant {
        self.started
    }
}
```

Held by review.

## A File Whose Role Excuses a Lint Suppresses It Once, at Its Top

An example prints its result and dies loudly on a broken assumption. A file
whose role excuses a lint throughout says so once, with an inner `#![expect]` at
its top, rather than on each item.

```rust
//! Lays a 3 by 3 grid and prints it.

// Bad: one `#[expect]` for each function that prints, where the file's role is to print.
#[expect(clippy::print_stdout, reason = "an example's output is what it shows")]
fn show(squares: u32) {
    println!("a 3 by 3 grid holds {squares} squares");
}

#[expect(clippy::print_stdout, reason = "an example's output is what it shows")]
fn main() {
    show(3_u32.saturating_mul(3));
    println!("and every one is blank");
}
```

```rust
//! Lays a 3 by 3 grid and prints it.

#![expect(clippy::print_stdout, reason = "an example's whole output is what it shows")]

fn show(squares: u32) {
    println!("a 3 by 3 grid holds {squares} squares");
}

fn main() {
    show(3_u32.saturating_mul(3));
    println!("and every one is blank");
}
```

Held by review. An integration test needs no such attribute: it holds its tests
in a `#[cfg(test)] mod tests`, as a unit test does, so
`clippy::tests_outside_test_module` never fires there.

How an integration test is laid out is `writing-rust-tests`'s.

A test file never says so for `expect`, `unwrap`, `panic!` or printing either:
the workspace's `clippy.toml` lets a `#[test]` function and a `#[cfg(test)]`
module use them, so a file-wide `#[expect]` for one goes unfulfilled and fails
the build. A shared helper outside both that uses one carries its own
`#[expect]`.

## A Crate Inherits the Table Whole

Every crate's manifest has `[lints] workspace = true` and nothing else under
`[lints]`: Cargo refuses a crate's own lint levels beside it, and a crate that
left the table would leave it without anyone deciding it should. A lint wrong
for one site is answered at that site. The table's keys are the rust-lints
profile's, and devset manages them: they are never edited by hand.
`unexpected_cfgs`, whose `check-cfg` lists the repository's own cfgs, is the
repository's key to set.

```toml
# Bad: Cargo refuses a crate's own levels beside the workspace's.
[lints]
workspace = true

[lints.clippy]
indexing_slicing = "allow"
```

```toml
[lints]
workspace = true
```

Held by Cargo, which refuses the first. In the table, each group is set at
`priority = -1`, so a lint named on its own overrides its group.

How a managed key is changed, for this repository or for every one the profile
serves, is `using-devset`'s.

## `clippy.toml` Configures Lints, and the Nearest One Wins

`clippy.toml` sets how a lint behaves, never its level, which is the table's.
The workspace's lets tests unwrap, expect, panic, print, index and use `dbg!`.
Clippy reads the `clippy.toml` nearest the crate and no other, so a crate that
adds one of its own, to ban a method on a hot path, copies those six keys into
it, or loses them.

```toml
# Bad: crates/tiles-core/clippy.toml, where the workspace's test keys are lost.
disallowed-methods = [{ path = "alloc::vec::Vec::new", reason = "no allocation while drawing" }]
```

```toml
allow-dbg-in-tests              = true
allow-expect-in-tests           = true
allow-indexing-slicing-in-tests = true
allow-panic-in-tests            = true
allow-print-in-tests            = true
allow-unwrap-in-tests           = true
disallowed-methods              = [{ path = "alloc::vec::Vec::new", reason = "no allocation while drawing" }]
```

Held by review.

## Convert with `From` and `TryFrom`, Not `as`

`as` between integers truncates, wraps or changes sign without a word, and
between an integer and a float it rounds. A widening that cannot lose is `From`,
`u32::from(cols)`, and a narrowing is `TryFrom`, whose refusal the code handles;
a width conversion the crate repeats has one home, a function named for what it
converts.

```rust,compile_fail
// fails: clippy::cast_possible_truncation
// Bad: a count past 65,535 comes back as something smaller.
#[must_use]
pub fn cols(squares: u32) -> u16 {
    squares as u16
}
```

```rust
use core::num::TryFromIntError;

pub fn cols(squares: u32) -> Result<u16, TryFromIntError> {
    u16::try_from(squares)
}

#[must_use]
pub fn squares(cols: u16, rows: u16) -> u32 {
    u32::from(cols).saturating_mul(u32::from(rows))
}
```

Held by clippy's `cast_possible_truncation`, `cast_possible_wrap`,
`cast_sign_loss`, `cast_precision_loss` and `cast_lossless`.

Under `strict`, `clippy::as_conversions` refuses every `as`. The rare one that
must stay sits behind an assertion that proves it and an `#[expect]` whose
reason names the proof.

## Arithmetic Says How It Overflows

`+` on integers panics on overflow in a debug build and wraps in a release one,
so the same code does two different things. Each operation names the one it
means: `checked_` where overflow is a refusal the caller handles, `saturating_`
where the bound is the right answer, `wrapping_` where the arithmetic is
modular. A value type whose own operators saturate or check says so in its doc,
and is listed in `clippy.toml`'s `arithmetic-side-effects-allowed`, with a
comment that says how they overflow.

```rust,compile_fail
// fails: clippy::arithmetic_side_effects
#[must_use]
pub const fn next_row(row: u16) -> u16 {
    // Bad: panics at `u16::MAX` in a debug build, wraps to 0 in a release one.
    row + 1
}
```

```rust
#[must_use]
pub const fn next_row(row: u16) -> Option<u16> {
    row.checked_add(1)
}

#[must_use]
pub const fn clamped_row(row: u16) -> u16 {
    row.saturating_add(1)
}

#[must_use]
pub const fn wrapped_col(col: u16, cols: u16) -> u16 {
    match col.wrapping_add(1).checked_rem(cols) {
        Some(col) => col,
        None => 0,
    }
}
```

Held by `clippy::arithmetic_side_effects`.

## Reach an Element with `get`, Not an Index

`squares[at]` panics when `at` is out of range, and the range is usually the
caller's to get wrong. `get` returns an `Option` the code answers, an iterator
walks without an index, and `split_first` and the slice patterns take a slice
apart. An index the code has just proved in bounds keeps its brackets under an
`#[expect]` whose reason names the proof.

```rust,compile_fail
// fails: clippy::indexing_slicing
#[must_use]
pub fn tile(squares: &[u8], at: usize) -> u8 {
    // Bad: a square past the end panics.
    squares[at]
}
```

```rust
#[must_use]
pub fn tile(squares: &[u8], at: usize) -> Option<u8> {
    squares.get(at).copied()
}

#[must_use]
pub const fn corners(squares: &[u8]) -> Option<(u8, u8)> {
    match squares {
        [first, .., last] => Some((*first, *last)),
        [only] => Some((*only, *only)),
        [] => None,
    }
}
```

Held by `clippy::indexing_slicing`, and `clippy::string_slice` for a `str`.

## Print Only from a Binary, with a Reason

A library that prints writes into its caller's output without asking, so a
library returns what there is to say, and the binary prints it. A binary that
prints says why on the function, or once at the top of a file whose role is to
print. `dbg!` never lands, and `process::exit` is not called outside `main`.

```rust,compile_fail
// fails: clippy::print_stdout
// Bad: a library function prints into its caller's output.
pub fn report(squares: u32) {
    println!("{squares} squares");
}
```

```rust
fn report(squares: u32) -> String {
    format!("{squares} squares")
}

#[expect(clippy::print_stdout, reason = "the square count is what an operator runs this for")]
fn main() {
    println!("{}", report(9));
}
```

Held by `clippy::print_stdout`, `print_stderr`, `dbg_macro` and `exit`.

## Every Lint Beyond the Defaults

Clippy's `all`, its default groups (`correctness`, `suspicious`, `style`,
`complexity`, `perf`), is on at `deny`. Four of the defaults, rustc's and
clippy's, stand for rules this skill would otherwise spell out:

| default                                                            | refuses                                        | write instead                                                                                                                                     |
| ------------------------------------------------------------------ | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `non_camel_case_types`, `non_snake_case`, `non_upper_case_globals` | a name cased against RFC 430                   | `UpperCamelCase` for types, traits and variants; `snake_case` for functions, modules and fields; `SCREAMING_SNAKE_CASE` for constants and statics |
| `clippy::ptr_arg`                                                  | a `&String`, `&Vec<T>` or `&PathBuf` parameter | `&str`, `&[T]`, `&Path`; a `&Path` takes a `&PathBuf` but not a `&str`, which `P: AsRef<Path>` takes                                              |
| `clippy::clone_on_copy`                                            | `.clone()` on a `Copy` value                   | the value itself                                                                                                                                  |
| `clippy::needless_lifetimes`                                       | a lifetime named where elision would supply it | elision; a lifetime is named only to tie an output to one input of several                                                                        |

Beyond the defaults, these are the lints the table adds, what each refuses, and
what to write instead:

| rustc                           | refuses                                                                      | write instead                             |
| ------------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------- |
| `unsafe_op_in_unsafe_fn`        | an unsafe operation in an `unsafe fn` outside an `unsafe {}` block           | a block of its own, with its `// SAFETY:` |
| `unused_must_use`               | a `#[must_use]` value dropped unread, a `Result` above all                   | handle it, or drop it by name             |
| `unknown_lints`                 | a lint name rustc does not know, as a typo in an `#[expect]`                 | the lint's exact name, `clippy::` and all |
| `unfulfilled_lint_expectations` | an `#[expect]` whose lint no longer fires                                    | delete the `#[expect]`                    |
| `rust_2018_idioms`              | a path's hidden lifetime, a bare trait object, an unused `extern crate`      | `Formatter<'_>`, `dyn Trait`, `use`       |
| `unused_qualifications`         | allowed: a path longer than it needs, which is sometimes the clearer one     |                                           |
| `meta_variable_misuse`          | a `macro_rules!` metavariable used unbound, or at the wrong repetition depth | a matcher that binds what the body uses   |
| `macro_use_extern_crate`        | `#[macro_use] extern crate`                                                  | `use` for each macro                      |

| rustdoc                        | refuses                                                       | write instead                            |
| ------------------------------ | ------------------------------------------------------------- | ---------------------------------------- |
| `broken_intra_doc_links`       | a ``[`Name`]`` that resolves to nothing                       | a path that resolves                     |
| `private_intra_doc_links`      | a public item's doc linking a private one                     | a public target, or the fact in words    |
| `unescaped_backticks`          | an unpaired backtick                                          | a closed code span                       |
| `invalid_html_tags`            | an unclosed tag, often a generic, `Vec<T>`, outside backticks | backticks around the code                |
| `bare_urls`                    | a URL written bare                                            | `<https://…>`, or a link                 |
| `redundant_explicit_links`     | ``[`Grid`](Grid)`` where ``[`Grid`]`` resolves                | the short form                           |
| `invalid_rust_codeblocks`      | a Rust doc block that does not parse                          | Rust that parses, or a `text` block      |
| `invalid_codeblock_attributes` | a misspelt fence attribute, `compile-fail`                    | `compile_fail`, `no_run`, `should_panic` |

| cargo                 | refuses                                  | write instead |
| --------------------- | ---------------------------------------- | ------------- |
| `non_kebab_case_bins` | a `[[bin]]` whose name is not kebab-case | `tiles-cli`   |

| clippy             | refuses                                                                                                                                                                                                                     | write instead   |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| `pedantic`         | the stricter group: `must_use_candidate`, `needless_pass_by_value`, `doc_markdown`, `missing_errors_doc`, `missing_panics_doc`, the `cast_` lints, `wildcard_imports`, `return_self_not_must_use`, `iter_without_into_iter` | what each names |
| `module_inception` | allowed: a module holding a module of its own name, which a type's home sometimes is                                                                                                                                        |                 |
| `empty_enums`      | allowed: an enum with no variants, which is how a type that cannot be built is written                                                                                                                                      |                 |

The `strict` feature adds:

| rustc (`strict`)                | refuses                                                                                    | write instead                                                                            |
| ------------------------------- | ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------- |
| `unsafe_code`                   | any `unsafe`                                                                               | an `#[expect]` at the narrowest scope, with its reason, as `writing-unsafe-rust` teaches |
| `missing_docs`                  | a public item with no doc                                                                  | its doc                                                                                  |
| `missing_debug_implementations` | a public type with no `Debug`                                                              | `#[derive(Debug)]`, or a hand-written one                                                |
| `unreachable_pub`               | a `pub` item no path outside the crate reaches                                             | `pub(crate)` or `pub(super)`                                                             |
| `unused`                        | dead code, and unused imports, variables, assignments, `mut`, labels and macros, at `deny` | delete it; a leading `_` for what is kept on purpose                                     |
| `single_use_lifetimes`          | a lifetime named where it is used once                                                     | `'_`                                                                                     |
| `unused_lifetimes`              | a lifetime declared and never used                                                         | delete it                                                                                |
| `variant_size_differences`      | an enum whose largest variant is over three times the next                                 | box the large payload                                                                    |

| clippy (`strict`)                                                                                                     | refuses                                                                                                                                                                                      | write instead                                                 |
| --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `nursery`                                                                                                             | the group still being settled: `missing_const_for_fn`, `use_self`, `derive_partial_eq_without_eq`, `redundant_clone`, `option_if_let_else`, `future_not_send`, `significant_drop_tightening` | what each names: a `const fn`, `Self`, the `Eq`               |
| `redundant_pub_crate`                                                                                                 | allowed: `pub(crate)` inside a private module, which says how far an item reaches                                                                                                            |                                                               |
| `self_named_module_files`                                                                                             | `grid.rs` beside a `grid/` directory                                                                                                                                                         | `grid/mod.rs`                                                 |
| `allow_attributes`                                                                                                    | `#[allow]`                                                                                                                                                                                   | `#[expect]`                                                   |
| `allow_attributes_without_reason`                                                                                     | an `#[expect]` with no `reason`                                                                                                                                                              | a `reason` that says why the lint is wrong here               |
| `unwrap_used`, `expect_used`                                                                                          | `unwrap` and `expect`                                                                                                                                                                        | `?`, `ok_or`, a `match`; an `#[expect]` on a broken invariant |
| `panic`, `todo`, `unimplemented`, `unreachable`                                                                       | the panicking macros                                                                                                                                                                         | an error; `match never {}` for an uninhabited arm             |
| `panic_in_result_fn`, `unwrap_in_result`                                                                              | a panic, an assertion or an `unwrap` in a function that returns a `Result`                                                                                                                   | the error the function already returns                        |
| `indexing_slicing`                                                                                                    | `squares[at]` and `squares[a..b]`                                                                                                                                                            | `get`, an iterator, a slice pattern                           |
| `string_slice`                                                                                                        | `text[a..b]` on a `str`, which panics off a character boundary                                                                                                                               | `text.get(a..b)`                                              |
| `get_unwrap`                                                                                                          | `.get(at).unwrap()`                                                                                                                                                                          | `.get(at)` and an answer to `None`                            |
| `arithmetic_side_effects`                                                                                             | `+`, `-`, `*`, `/`, `%` and `<<` on integers                                                                                                                                                 | `checked_`, `saturating_`, `wrapping_`                        |
| `as_conversions`                                                                                                      | `as`                                                                                                                                                                                         | `From`, `TryFrom`                                             |
| `lossy_float_literal`                                                                                                 | a float literal its type cannot hold exactly                                                                                                                                                 | one it can                                                    |
| `error_impl_error`                                                                                                    | a type named `Error` that implements `Error`                                                                                                                                                 | `<Question>Error`                                             |
| `map_err_ignore`                                                                                                      | `map_err(\|_\| …)`                                                                                                                                                                           | `map_err(\|_gone\| …)`, naming what is dropped                |
| `unused_result_ok`                                                                                                    | `.ok();` as a statement                                                                                                                                                                      | an answer to the error, or a named drop                       |
| `let_underscore_must_use`                                                                                             | `let _ = ` on a `#[must_use]` value                                                                                                                                                          | an answer, or `let _held = `                                  |
| `mem_forget`                                                                                                          | `mem::forget` on a value with a destructor                                                                                                                                                   | let it drop, or `ManuallyDrop` with a reason                  |
| `dbg_macro`                                                                                                           | `dbg!`                                                                                                                                                                                       | delete it                                                     |
| `print_stdout`, `print_stderr`                                                                                        | `print!` and `eprint!`                                                                                                                                                                       | return the text; in a binary, an `#[expect]` with a reason    |
| `exit`                                                                                                                | `process::exit` outside `main`                                                                                                                                                               | return an error to `main`                                     |
| `absolute_paths`                                                                                                      | a path of more than two segments written inline, `core::fmt::Display`                                                                                                                        | a `use`                                                       |
| `std_instead_of_core`, `std_instead_of_alloc`                                                                         | `std::` for what `core` or `alloc` has                                                                                                                                                       | `core::`, then `alloc::` with `extern crate alloc`            |
| `clone_on_ref_ptr`                                                                                                    | `.clone()` on an `Arc` or `Rc`                                                                                                                                                               | `Arc::clone(&board)`                                          |
| `mutex_atomic`, `mutex_integer`                                                                                       | a `Mutex` around a `bool` or an integer                                                                                                                                                      | an atomic                                                     |
| `rc_buffer`, `rc_mutex`                                                                                               | `Rc<String>`, `Rc<Vec<T>>`, `Rc<Mutex<T>>`                                                                                                                                                   | `Rc<str>`, `Rc<[T]>`, `Rc<RefCell<T>>`                        |
| `wildcard_enum_match_arm`                                                                                             | `_ =>` on an enum                                                                                                                                                                            | every variant by name                                         |
| `same_name_method`                                                                                                    | an inherent method and a trait method of one name on a type                                                                                                                                  | one of them renamed, or an `#[expect]` where both are meant   |
| `mixed_read_write_in_expression`                                                                                      | a read and a write of one variable whose order depends on evaluation order                                                                                                                   | two statements                                                |
| `suspicious_xor_used_as_pow`                                                                                          | `2 ^ 8` written as a power                                                                                                                                                                   | `2_u32.pow(8)`, `1 << 8`                                      |
| `redundant_type_annotations`                                                                                          | `let grid: Grid = Grid::new(…)`                                                                                                                                                              | `let grid = Grid::new(…)`                                     |
| `unnecessary_self_imports`                                                                                            | `use core::fmt::{self};`                                                                                                                                                                     | `use core::fmt;`                                              |
| `str_to_string`                                                                                                       | `"t42".to_string()`                                                                                                                                                                          | `"t42".to_owned()`                                            |
| `empty_structs_with_brackets`                                                                                         | `struct Blank {}`                                                                                                                                                                            | `struct Blank;`                                               |
| `empty_enum_variants_with_brackets`                                                                                   | `Blank {}` or `Blank()` as a variant                                                                                                                                                         | `Blank`                                                       |
| `create_dir`                                                                                                          | `fs::create_dir`, which fails when a parent is missing                                                                                                                                       | `fs::create_dir_all`                                          |
| `verbose_file_reads`                                                                                                  | `File::open` and then `read_to_end` or `read_to_string`                                                                                                                                      | `fs::read`, `fs::read_to_string`                              |
| `filetype_is_file`                                                                                                    | `file_type().is_file()` to mean readable, which a fifo or a symlink is not                                                                                                                   | `!file_type().is_dir()`                                       |
| `semicolon_inside_block`                                                                                              | `{ grid.clear() };`                                                                                                                                                                          | `{ grid.clear(); }`                                           |
| `tests_outside_test_module`                                                                                           | a `#[test]` outside a `#[cfg(test)]` module                                                                                                                                                  | `mod tests`, in an integration test too                       |
| `unseparated_literal_suffix`                                                                                          | `0u8`                                                                                                                                                                                        | `0_u8`                                                        |
| `pub_without_shorthand`                                                                                               | `pub(in super)`                                                                                                                                                                              | `pub(super)`                                                  |
| `missing_assert_message`                                                                                              | an assertion with no message                                                                                                                                                                 | a message that states the property                            |
| `impl_trait_in_params`                                                                                                | `impl Trait` in a public function's arguments                                                                                                                                                | a named generic                                               |
| `missing_docs_in_private_items`                                                                                       | a private item, field or test helper with no doc                                                                                                                                             | its doc                                                       |
| `undocumented_unsafe_blocks`, `multiple_unsafe_ops_per_block`, `unnecessary_safety_comment`, `unnecessary_safety_doc` | the unsafe lints                                                                                                                                                                             | what `writing-unsafe-rust` teaches                            |

`strict` also names three lints the groups above already deny:
`await_holding_lock`, in `suspicious`, which refuses a `std` lock's guard held
across an `.await`, and `float_cmp` and `wildcard_imports`, in `pedantic`.

The `nightly` feature adds the two lints only nightly rustc has, which
`check-rust-lints` runs:

| rustc (`nightly`)                 | refuses                                                                                          | write instead                                                  |
| --------------------------------- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| `non_exhaustive_omitted_patterns` | a wildcard on another crate's `#[non_exhaustive]` enum that swallows a variant it could name     | the variant by name, then the wildcard                         |
| `implicit_provenance_casts`       | an `as` cast between a pointer and an integer, either way, which exposes or guesses a provenance | `.addr()` and `with_addr`, or `expose_provenance()` on purpose |
