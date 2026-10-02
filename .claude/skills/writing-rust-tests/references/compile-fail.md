# Compile-Fail Tests

Read this before a test that a misuse must fail to compile, before a fixture or
its `.stderr`, before a macro's tests, and before asserting what a type must
keep, its size or its `Send`. A type that refuses a misuse by construction holds
that refusal only until someone changes it; these tests say when they have.

## A Misuse the Types Refuse Is a `trybuild` Fixture

A handle that cannot outlive its board, a brush that cannot cross threads, a
builder that cannot be finished twice: each refusal is pinned by a fixture that
must not compile, `tests/compile_fail/<the_refusal>.rs`, one misuse a file,
beside the compiler's message in `<the_refusal>.stderr`, both committed.
`tests/trybuild.rs` runs them all, and fails when a fixture compiles, or fails
with another message, so a fixture broken by a typo fails too. A rustdoc
`compile_fail` block passes on any error at all, a renamed function included,
which is why it is not the test.

````text
/// A brush paints on the thread that made it.
///
/// ```compile_fail
/// // Bad: fails on any error, so a renamed `slot` passes it as well as a `Send` brush.
/// let brush = tiles::Brush::here();
/// std::thread::spawn(move || brush.slot());
/// ```
````

```text
// tests/compile_fail/a_brush_stays_on_its_thread.rs
//! A brush names a slot in its own thread's palette, so it cannot cross to another.

use std::thread;

use tiles::Brush;

fn main() {
    let brush = Brush::here();
    thread::spawn(move || brush.slot());
}

// tests/trybuild.rs
//! The misuses the types refuse, each a fixture that must not compile.

#[cfg(test)]
mod tests {
    #[test]
    fn each_misuse_fails_to_compile() {
        trybuild::TestCases::new().compile_fail("tests/compile_fail/*.rs");
    }
}
```

Held by the fixtures. trybuild is a `[dev-dependencies]` entry; a fixture
reaches the crate by its name, and the crate's dev-dependencies too.

In a crate with loom models, `tests/trybuild.rs` carries `#![cfg(not(loom))]`,
since a loom model drives no compiler.

## A Fixture's Message Is Written by `trybuild`, and Read

A fixture's `.stderr` is what the compiler said, and the test compares each run
against it. A new fixture has none: trybuild writes what it got to `wip/` beside
the crate's manifest, and fails. `TRYBUILD=overwrite` writes each message in
place instead. Either way the new message is read before it is committed, since
it is the test: an E0599 where an E0277 was meant pins a typo. A new toolchain
may reword a message, and the fixture fails until it is written again and read.

```text
# Bad: a message committed unread, and a hand-edited one that no compiler wrote.
TRYBUILD=overwrite cargo test -p tiles --test trybuild && git add -A
```

```text
TRYBUILD=overwrite cargo nextest run -p tiles --test trybuild
git diff crates/tiles/tests/compile_fail/
```

Held by review. The workspace's toolchain carries `rust-src`, so a message that
quotes the standard library quotes the same lines on every machine.

## A Fixture Builds with the Features Its Test Runs With

trybuild builds each fixture against the crate with the features the test was
built with, and the checks build every feature at once. A fixture that must fail
only while a feature is off compiles under `--all-features`, and fails the test.
So a fixture refuses what no feature allows, or it sits in a directory of its
own, which the test compiles only under `cfg!(not(feature = "…"))`. Every check
builds with `--all-features`, so that directory runs by hand, in a build without
the feature: `cargo nextest run -p tiles --no-default-features --test trybuild`.

```text
// Bad: tests/compile_fail/a_narrow_brush_has_no_width.rs, which compiles once `wide` is on.
fn main() {
    let _width = tiles::Brush::here().width();
}
```

```text
// tests/trybuild.rs
//! The misuses the types refuse, each a fixture that must not compile.

#[cfg(test)]
mod tests {
    #[test]
    fn each_misuse_fails_to_compile() {
        let cases = trybuild::TestCases::new();
        cases.compile_fail("tests/compile_fail/*.rs");
        if cfg!(not(feature = "wide")) {
            cases.compile_fail("tests/compile_fail/narrow/*.rs");
        }
    }
}
```

Held by the fixtures: the ones for every build under `just check-cargo-nextest`,
the others by hand.

## A Macro's Accepted Forms Are Fixtures Too

A macro or a derive has forms it must accept as well as forms it must refuse,
and a change to its parser can break either. What it must accept is a fixture in
`tests/compile_pass/`, which `pass` builds and runs: it passes when it compiles
and its `main` does not panic. A form it must refuse is a `compile_fail`
fixture, its message the macro's own.

```text
// Bad: only the refusals are pinned, so a change that refuses every input passes.
trybuild::TestCases::new().compile_fail("tests/compile_fail/*.rs");
```

```text
let cases = trybuild::TestCases::new();
cases.pass("tests/compile_pass/*.rs");
cases.compile_fail("tests/compile_fail/*.rs");
```

Held by the fixtures.

## Several Crates' `trybuild` Suites Run One at a Time

Each crate's `tests/trybuild.rs` builds its fixtures under
`target/tests/trybuild/`, which every crate's suite shares, and nextest runs
test binaries in parallel, so two suites contend for one build directory. A
workspace with several suites puts them in one nextest test group that runs one
at a time, as `running.md` shows. The filter names only binaries the workspace
has: nextest refuses `binary(=trybuild)` where no test binary of that name
exists, so the group is added with the first suite.

```text
# Bad: two suites building under one directory at once.
cargo nextest run --workspace
```

```toml
[test-groups.trybuild]
max-threads = 1

[[profile.default.overrides]]
filter     = 'binary(=trybuild)'
test-group = 'trybuild'
```

Held by review.

## What a Type Must Keep Is Asserted at Compile Time

A size a type must stay within, a trait it must keep implementing, `Send`,
`Sync`, `Copy`, is asserted where the type is defined, in a `const` item that
fails the build of the crate itself when the fact stops holding. A test that
checks it at run time fails only when the tests run, and only if the test is
there.

Which size a hot type keeps, and why, is `tuning-rust-performance`'s.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

#[cfg(test)]
mod tests {
    use super::Pos;

    // Bad: holds only when the tests run.
    #[test]
    fn a_pos_is_two_columns_wide() {
        assert_eq!(size_of::<Pos>(), 4, "two `u16`s and nothing else");
    }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

// A pos is copied into every square of a board, and crosses to the renderer's threads.
const _: () = assert!(size_of::<Pos>() == 4, "two `u16`s and nothing else");
const fn copy_send_sync<T: Copy + Send + Sync>() {}
const _: () = copy_send_sync::<Pos>();
```

Held by the crate's own build. A trait that must stay absent, a type that must
not be `Send`, is a trybuild fixture: Rust has no bound that says "not", and
though a trick of ambiguous impls can assert it, a fixture is the plain way, and
pins the message too.
