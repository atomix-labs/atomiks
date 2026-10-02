---
name: writing-rust-tests
description: Use when writing, changing, reviewing or fixing Rust tests, whether a unit test, an integration test in `tests/`, a fixture or `testing.rs`, a trybuild compile-fail test, a proptest property, an rstest table, a cargo-fuzz target or a doctest; when a test fails, is flaky or hangs, or needs `#[ignore]` or `#[should_panic]`; when naming a test or writing its assertion and `expect` messages; when running tests with cargo-nextest, or choosing where a test goes. Covers test layout, names and messages, fixtures and doubles, compile-fail tests, property tests, tables, fuzzing, doctests, and how nextest runs them. Not for the code under test, which writing-rust covers, or a loom model, which writing-unsafe-rust covers.
---

# Writing Rust Tests

How tests are written and run in this workspace: each where its reader looks for
it, named for the property it pins, failing with a message that says what broke,
and standing alone; misuses the types refuse, pinned by fixtures; invariants
checked over every input, and input from outside fuzzed. The rules below are the
whole of it, each with its reason. The references hold each rule's why, a bad
and a good example, and what holds the rule; each Rust example compiles under
the workspace's lints.

The examples leave their docs out to stay short, and compile with the lints that
ask for docs off; real code writes them: a `//!` atop each file in `tests/`
saying what it proves, and a line on each fixture saying its role.

Under `strict`, `missing_docs` asks each file in `tests/` for its `//!`, and no
lint asks a test or a fixture for more. A `#![no_std]` library's tests need what
the examples assume: `#[cfg(test)] extern crate std;` in `lib.rs`, and
`#[cfg(test)] extern crate alloc;` where the library has no `extern crate
alloc;`, and `alloc`'s names, `alloc::string::ToString`, imported by name.

## Rules

### Where Tests Go

1. **Unit tests sit at the bottom of the file they pin, in `#[cfg(test)] mod
   tests`, importing each name, `use super::{Board, Pos}`**, so a reader meets
   the code first and sees what the tests touch; never `use super::*`.
2. **What a caller does is tested in `tests/<name>.rs`, through the public API,
   its tests in a `#[cfg(test)] mod tests` too, the file opening with a `//!`
   that says what it proves**, so every test has one form; a test of several
   files is `tests/<name>/main.rs`, since `tests/<name>/mod.rs` is no target and
   its tests never run.
3. **What the crate's tests share lives in `src/testing.rs`, declared
   `#[cfg(test)] mod testing;`, its items `pub(crate)`**, so a fixture is
   written once and stays out of the API. What integration tests share lives in
   `tests/testing/mod.rs`, declared `#[cfg(test)] mod testing;` in each file, as
   `lib.rs` declares its own (`#[path = "../testing/mod.rs"]` from a `main.rs`),
   and opening with `#![allow(dead_code, reason = "each test binary uses part of
   this module")]`, the workspace's one `allow`. What another crate's tests need
   sits behind a `testing` feature, `#[doc(hidden)] pub`, turned on only in
   their `[dev-dependencies]`.
4. **A test of what a feature adds sits under that feature's `#[cfg(feature =
   "…")]`**, since it compiles only with the feature on.

### Names and Messages

1. **A test's name is the property it pins, a sentence, its subject first,
   `a_step_past_the_last_column_is_refused`, and a test pins one property**;
   never `test_`, `it_works` or the function's name, since the name is what a
   failure prints first.
2. **Compare whole values with `assert_eq!`, an error included**, so a failure
   prints what arrived beside what was wanted; `assert!(a == b)`, `is_err()` and
   `matches!` say only that it was wrong, and `format!("{x:?}")` is no contract.
3. **An assertion's message says the property, a lowercase fragment, and one
   test's messages read as a running sentence, `"to itself"`, `"and
   forwards"`**, never `"test failed"`; it is left out only where the expression
   says it all.
4. **An `expect` says why the call cannot fail here, `expect("3,4 lies on an 8
   by 8 board")`**, never `unwrap` or `"failed"`, so a failure names the
   assumption that broke.
5. **An impossible arm panics with what arrived, `let … else { panic!("…, not
   {moved:?}") }`**, never `unreachable!`, since a failing test is exactly when
   the arm is reached.

### Writing a Test

1. **Test each edge and each refusal**: the empty board, the first and last
   square, one past the end, the largest value, and each error a function
   returns, compared whole, with a test that pins its message.
2. **A `#[test]` function returns `()`**, since an `Err` out of one prints its
   `Debug` and no line; a doctest is an example a caller copies, and uses `?`,
   closed by a hidden `# Ok::<(), E>(())`.
3. **`#[should_panic(expected = "…")]`, never bare, on a test that returns
   `()`**: a bare one passes on any panic, and on a `Result` test it does not
   compile. It pins a panic the API promises, as an `Index` does beside its
   `get`; a refusal a caller can cause is an error, and its test compares the
   `Err`.
4. **`#[ignore = "…"]` names what the test needs, `"needs a tile server on
   localhost:7070"`**, so whoever skips it knows what runs it; a slow test is
   made fast, not ignored.
5. **A test stands alone: it passes alone, in any order, and beside every other
   in one process**, since nextest gives each test a process of its own and
   hides a shared global, where `cargo test` runs a binary's tests in one.
6. **A test waits for a condition with a deadline, `recv_timeout`, a join, a
   polled flag, never a fixed sleep**, so a slow machine is slow, not red, and a
   hang fails with a message.
7. **A fixture that makes something outside the process makes it fresh for each
   test and gives it back on drop**, a `TempDir`, a port bound as port 0, never
   a fixed path or port, since tests run at once and may panic midway.
8. **A double is a fake, a small type in the tests implementing the trait the
   code takes**, over a value the test sets, since a mock's expectations restate
   the implementation. Where the repository already uses mockall, each value is
   an expectation of its own: a chained `returning` keeps the last.
9. **The code under test takes the clock, the environment and the working
   directory as values or a trait, and a test never calls `env::set_var` or
   `set_current_dir`, nor waits on the wall clock**, since each is shared by
   every thread of the process, and `set_var` is `unsafe` in edition 2024.

### Compile-Fail Tests

1. **A misuse the types refuse is a trybuild fixture,
   `tests/compile_fail/<the_refusal>.rs`, one misuse a file, its `//!` naming
   what it prevents, its `.stderr` committed**, run by `tests/trybuild.rs`; a
   rustdoc `compile_fail` block passes on any error, so it is not the test.
2. **A fixture's message is written by `TRYBUILD=overwrite` and read before it
   is committed**, never edited by hand, since it is the test; a new fixture's
   message lands in `wip/`, and fails.
3. **A fixture holds with every feature on**, since trybuild builds it with the
   test's features and the checks turn every one on; one that needs a feature
   off sits in a directory compiled only under `cfg!(not(feature = "…"))`, run
   by hand without the feature.
4. **A macro's accepted forms are `pass` fixtures in `tests/compile_pass/`**,
   beside its refusals, so a change that refuses every input fails.
5. **Several crates' trybuild suites share a nextest test group of one thread**,
   since they build under one `target/tests/trybuild/`.
6. **What a type must keep, its size, `Send`, `Sync`, `Copy`, is a `const`
   assertion beside it**, which fails the crate's own build, not only a test.

### Properties, Tables and Fuzzing

1. **An invariant over every input is a proptest property**: a round trip, a
   parser that never panics, agreement with a simple model; proptest runs 256
   cases and shrinks a failing input to the smallest.
2. **A property generates valid inputs, `0..8_u16` or `prop_map`, and uses
   `prop_assume!` never to discard more than a sliver of them**, since proptest
   gives up after 1,024 rejections.
3. **A property asserts with `prop_assert!` and `prop_assert_eq!`**, which
   report the smallest failing input once, where an `assert!` prints a panic for
   each shrinking step.
4. **`proptest-regressions/` is committed**, so a case that failed once runs
   first everywhere.
5. **A table of inputs and answers is an rstest, each `#[case]` a test of its
   own, named where its input does not say what it is for**, since a loop stops
   at the first wrong row.
6. **A decoder of what another program or a person wrote is fuzzed with
   cargo-fuzz, in the crate's `fuzz/`, a workspace of its own through an empty
   `[workspace]` table**, since a fuzzer finds inputs no strategy generates, and
   cargo-fuzz builds with flags and code the workspace's lints refuse; `cargo
   install cargo-fuzz` installs it, since no profile pins it.
7. **A target's corpus is committed, and replayed with `cargo fuzz run
   <target> -- -runs=0`**, so a fresh checkout starts from the coverage found
   and a change is checked against it; `cargo fuzz init` ignores it, so that
   line comes out.
8. **A crash the fuzzer finds, shrunk with `cargo fuzz tmin`, becomes a unit
   test**, so every check pins it, not only the next fuzzing run.

### Running

1. **`just check-cargo-nextest` runs every test with nextest, then the doctests
   with `cargo test --doc`**, since nextest cannot run doctests; one test by
   hand is `cargo nextest run -p <crate> <name>`.
2. **An ignored test runs by hand where what it needs is, `--run-ignored
   only`**, before a change to what it tests is done, since no recipe runs it.
3. **Tests that contend for what cannot be made fresh, a pinned core, a device,
   trybuild's build directory, run in a nextest test group of `max-threads = 1`,
   in `.config/nextest.toml`**, whose `[test-groups]` and `[profile.default]`
   keys are the repository's; the cargo-nextest profile owns only the `ci`
   profile's.
4. **A retry is how a flaky test is found, and FLAKY is a fault to fix, never a
   pass to accept**, since the run still passes; the retry count is the
   repository's, in `.config/nextest.toml`, and `--stress-count` runs one test
   until it fails.
5. **A test that can hang has a `slow-timeout` with `terminate-after`**, since
   nextest marks a slow test and stops nothing on its own.

Under `strict`, the lints hold more in tests:

- **`unreachable!` and `todo!` are refused**, and `panic_in_result_fn` refuses
  an assertion in a test that returns `Result`.
- **`tests_outside_test_module` refuses a `#[test]` outside a `#[cfg(test)]`
  module**, integration tests included.
- **`unreachable_pub` asks for `pub(crate)`** in `tests/testing/mod.rs`, and
  `as`, absolute paths and `std` where `core` has it are refused as elsewhere.
- **Arithmetic says how it overflows in a test's helpers too**:
  `arithmetic_side_effects` spares a `#[test]` function, not a helper beside it.
- **The workspace's `clippy.toml` lets code in a `#[test]` or a `#[cfg(test)]`
  module unwrap, expect, panic, index, print and use `dbg!`**, a `#[cfg(test)]
  mod testing;` included; a helper outside both carries its own `#[expect]`,
  with a reason.

## Steps

Read each reference a step names, whole, before writing the test.

1. **Changing tests**: read the module's tests first and match them; a test
   these rules break is named in the change's description, not rewritten in
   passing.
2. **A bug fix**: first a test that fails without the fix, named for what should
   have held.
3. **A new function or type**: `references/writing-tests.md`: a test for each
   edge and each refusal, named for its property, comparing whole values.
4. **A new error type**: `references/writing-tests.md`: a test that provokes
   each refusal, and one that pins each message.
5. **A new test file, a fixture, or a test of a feature**:
   `references/layout.md`.
6. **A misuse the types must refuse, a macro, or what a type must keep**:
   `references/compile-fail.md`.
7. **An invariant, a table of cases, or input from outside**:
   `references/properties.md`.
8. **A doctest**: it runs under `cargo test --doc`, never nextest, and shows a
   caller's use; a property it shows is pinned by a unit test too.
9. **A failing test**: `references/running.md`: run it alone and read its name
   and message. If it passes alone under nextest, it competes for something
   outside the process or for time: give it a fresh fixture, a deadline, or a
   test group. If it fails only under `cargo test`, it shares a global.
10. **A flaky or hanging test**: `references/running.md` and
    `references/writing-tests.md`: stress it until it fails, find the sleep or
    the shared global, and fix that; a FLAKY line in a run is such a test.
11. **Before finishing**: the checks below, and each ignored test the change
    touches.

The types and errors a test pins follow `writing-rust`; a test of unsafe code or
a loom model also follows `writing-unsafe-rust`.

A doctest's form, and the `//!` of a test file or a fixture, are
`writing-rustdoc`'s.

## Checks

- `just check`: every check, as CI runs them, after `just fix`.
- `just check-cargo-nextest`: every test under nextest, then the doctests under
  `cargo test --doc`; under CI, the `ci` profile, which runs every test however
  many fail.
- `just check-rust-clippy`: clippy on every target and feature, tests included.
- `just nightly-cargo-hack`: clippy with no features and with each alone, the
  tests included, which finds a test not gated on the feature it needs; it runs
  nightly, not in `just check`.
- By hand, since no recipe runs them: `cargo nextest run --run-ignored only`,
  `TRYBUILD=overwrite cargo nextest run -p <crate> --test trybuild` for a new or
  changed fixture, and `cargo fuzz run <target> -- -runs=0` for each fuzz target
  a change reaches.

## What Not to Do

| Thought                                    | Instead                                                         |
| ------------------------------------------ | --------------------------------------------------------------- |
| "`use super::*` is shorter"                | Import each name the tests use.                                 |
| "`test_parse` says what it tests"          | The property: `a_pos_with_no_row_is_refused`.                   |
| "`assert!(result.is_err())`"               | `assert_eq!(result, Err(BoundsError { col: 8, cols: 8 }))`.     |
| "`unwrap()`, it is only a test"            | `expect("…")`, saying why it cannot fail here.                  |
| "`-> Result` and `?` keep the test short"  | `()` and `expect`: an `Err` out of a test prints no line.       |
| "`#[should_panic]` is enough"              | `expected = "…"`, with the text of the message.                 |
| "Sleep 50 ms for the thread"               | Wait for the thing itself, with a deadline.                     |
| "It passes under nextest"                  | It passes under `cargo test` too, with nothing shared.          |
| "It passed on the retry"                   | FLAKY is a fault: `--stress-count` until it fails, then fix it. |
| "A mock for the store"                     | A fake that implements the trait over a plain value.            |
| "`pub`, so `tests/` can reach it"          | A unit test in its module; `tests/` tests what callers reach.   |
| "`tests/common/mod.rs` for the helpers"    | `tests/testing/mod.rs`; a test of several files is `main.rs`.   |
| "A `compile_fail` doctest pins it"         | A trybuild fixture, with its `.stderr`.                         |
| "Ignore `proptest-regressions/`, `corpus`" | Commit both.                                                    |
| "A snapshot crate for the message"         | `assert_eq!` on `to_string()`; only trybuild keeps snapshots.   |

## References

Read every reference a task touches before writing a test, and read them again
after compaction: this body is the summary, and the examples are there.

- `references/layout.md`: before a test file, a fixture, a `testing` feature, a
  test of a feature, or moving tests.
- `references/writing-tests.md`: before any test: its name, assertions and
  messages, `expect`, `#[should_panic]`, `#[ignore]`, what it shares, how it
  waits, the clock and environment it reads, its fixtures and doubles.
- `references/running.md`: before running tests by hand, when a test fails,
  hangs or flakes, and before a test group or `.config/nextest.toml`.
- `references/compile-fail.md`: before a compile-fail or compile-pass fixture, a
  `.stderr`, a macro's tests, or a compile-time assertion.
- `references/properties.md`: before a property, a table of cases, a fuzz
  target, or a crash one found.
- `references/sources.md`: before citing a source for a rule, or adapting one.
