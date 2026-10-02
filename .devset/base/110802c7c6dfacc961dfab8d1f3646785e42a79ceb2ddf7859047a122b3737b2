# Layout

Read this before adding a test, a test file or a fixture, before a test that
needs a feature, and before moving tests. It says where each kind of test lives,
and the form each file takes so the lints and the runner read it as a test.

## Unit Tests Sit at the Bottom of Their File, in `mod tests`, Importing by Name

A unit test reaches the private items of the module it pins, so it lives in that
module's file, in a `#[cfg(test)] mod tests` after everything else: a reader
meets the code, then what pins it, and only the test build compiles it. It
imports each name it uses, `use super::{Board, Pos}`, as the module's other code
does, so a reader sees what the tests touch; `use super::*` takes whatever the
module grows, and hides which items a test reaches.

```rust,compile_fail
// fails: clippy::items_after_test_module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

// Bad: the tests sit between two items, and take every name the module has.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_right_moves_one_column() {
        assert_eq!(step_right(Pos { col: 3, row: 4 }), Some(Pos { col: 4, row: 4 }), "one column");
    }
}

#[must_use]
pub fn step_right(at: Pos) -> Option<Pos> {
    at.col.checked_add(1).map(|col| Pos { col, ..at })
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

#[must_use]
pub fn step_right(at: Pos) -> Option<Pos> {
    at.col.checked_add(1).map(|col| Pos { col, ..at })
}

#[cfg(test)]
mod tests {
    use super::{Pos, step_right};

    #[test]
    fn a_step_right_moves_one_column() {
        assert_eq!(step_right(Pos { col: 3, row: 4 }), Some(Pos { col: 4, row: 4 }), "one column");
    }
}
```

Held by `clippy::items_after_test_module`, which refuses an item after the test
module. The glob is held by review: `clippy::wildcard_imports` spares `use
super::*` in a test module, and a `prelude` glob anywhere.

## A Caller's View Is Tested in `tests/`, in `mod tests` Too

What a caller of the crate does, through its public API alone, is an integration
test: `tests/<name>.rs`, which Cargo builds as a crate of its own against the
library, so a test there cannot lean on a private item. Its tests sit in a
`#[cfg(test)] mod tests`, as a unit test's do, with the file's shared helpers
inside it, so every test in the workspace has one form, and the file opens with
a `//!` that says what it proves.

```text
// Bad: tests/moves.rs, its tests loose at the top of the file.
#[test]
fn a_tile_moves_right() {
    let moved = tiles::step_right(tiles::Pos { col: 3, row: 4 });
    assert_eq!(moved, Some(tiles::Pos { col: 4, row: 4 }), "one column");
}
```

```text
//! A tile moves as a caller of the crate moves it, one square at a time.

#[cfg(test)]
mod tests {
    use tiles::{Pos, step_right};

    #[test]
    fn a_tile_moves_right() {
        assert_eq!(step_right(Pos { col: 3, row: 4 }), Some(Pos { col: 4, row: 4 }), "one column");
    }
}
```

Held by `clippy::tests_outside_test_module`, which refuses a `#[test]` outside a
`#[cfg(test)]` module, and by `missing_docs`, which asks each test crate for its
`//!`; the items inside ask for none.

The workspace's `clippy.toml` lets code in a `#[cfg(test)]` module unwrap and
expect as a `#[test]` does, so the file's helpers need no `#[expect]` of their
own.

## A Test of Several Files Is `tests/<name>/main.rs`

Cargo finds a test target in each `tests/*.rs` and each `tests/*/main.rs`, and
nowhere else unless the manifest names one: a `tests/<name>/mod.rs` compiles
into no target, so its tests never run, and no runner says so. A test that
outgrows one file becomes a directory with a `main.rs` that declares its
modules, and reaches the shared `tests/testing/mod.rs` by its path, since a `mod
testing;` there looks beside the `main.rs`.

```text
// Bad: no target, so these tests are never built or run.
tests/moves/mod.rs
tests/moves/diagonal.rs
```

```text
// tests/moves/main.rs, beside tests/moves/diagonal.rs and tests/moves/straight.rs
//! A tile moves as a caller of the crate moves it, along a line or a diagonal.

#[cfg(test)]
#[path = "../testing/mod.rs"]
mod testing;

#[cfg(test)]
mod diagonal;
#[cfg(test)]
mod straight;
```

Held by review. `cargo nextest list` names every test the targets hold; one
missing from it is in no target.

## What the Crate's Tests Share Lives in `testing.rs`

A fixture two test modules need, a board laid out, a tile set, is written once,
in `src/testing.rs`, which `lib.rs` declares `#[cfg(test)] mod testing;`, its
items `pub(crate)`; each test module imports it by name. A fixture copied into
each module drifts, and one made public for the tests is part of the crate's
API.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    pub cols: u16,
    pub rows: u16,
}

// Bad: part of the crate's API, only so its tests can share a board.
#[must_use]
pub const fn test_board() -> Board {
    Board { cols: 8, rows: 8 }
}

impl Board {
    #[must_use]
    pub fn squares(self) -> u32 {
        u32::from(self.cols).saturating_mul(u32::from(self.rows))
    }
}

#[cfg(test)]
mod tests {
    use super::test_board;

    #[test]
    fn a_chessboard_has_sixty_four_squares() {
        assert_eq!(test_board().squares(), 64, "eight by eight");
    }
}
```

```rust
mod board {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Board {
        pub cols: u16,
        pub rows: u16,
    }

    impl Board {
        #[must_use]
        pub fn squares(self) -> u32 {
            u32::from(self.cols).saturating_mul(u32::from(self.rows))
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::testing::chessboard;

        #[test]
        fn a_chessboard_has_sixty_four_squares() {
            assert_eq!(chessboard().squares(), 64, "eight by eight");
        }
    }
}

pub use crate::board::Board;

// src/testing.rs, under `#[cfg(test)] mod testing;` in lib.rs.
#[cfg(test)]
mod testing {
    use crate::Board;

    pub(crate) const fn chessboard() -> Board {
        Board { cols: 8, rows: 8 }
    }
}
```

Held by review. A fixture no test uses is dead code: delete it.

## What Integration Tests Share Lives in `tests/testing/mod.rs`

Each file in `tests/` is its own crate, so what several share lives in
`tests/testing/mod.rs`, a directory Cargo makes no target of, and each file that
uses it declares `#[cfg(test)] mod testing;`, as `lib.rs` declares its own. Each
test binary compiles the whole module and uses part of it, so a helper one
binary leaves unused is dead code there and used in the next: the module says so
once, at its top, with the one `allow` the workspace has, since an `#[expect]`
goes unfulfilled in the binary that uses every helper.

```text
// Bad: tests/testing/mod.rs, and an `#[expect]` that fails the binary using both helpers.
#![expect(dead_code, reason = "each test binary uses part of this module")]

pub(crate) fn chessboard() -> tiles::Board { tiles::Board { cols: 8, rows: 8 } }
pub(crate) fn strip() -> tiles::Board { tiles::Board { cols: 8, rows: 1 } }
```

```text
// tests/testing/mod.rs
//! What the integration tests share.

#![allow(dead_code, reason = "each test binary uses part of this module")]

use tiles::Board;

pub(crate) const fn chessboard() -> Board {
    Board { cols: 8, rows: 8 }
}

pub(crate) const fn strip() -> Board {
    Board { cols: 8, rows: 1 }
}

// tests/squares.rs
//! A board counts its squares as a caller reads them.

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use crate::testing::chessboard;

    #[test]
    fn a_chessboard_has_sixty_four_squares() {
        assert_eq!(chessboard().squares(), 64, "eight by eight");
    }
}
```

Held by `unfulfilled_lint_expectations`, in the binary that uses every helper;
`clippy::allow_attributes` refuses an outer `#[allow]` and spares this inner
one, and `unreachable_pub` asks for `pub(crate)` on each helper.

Declared under `#[cfg(test)]`, the module is test code to clippy, so its helpers
may expect without an `#[expect]`; declared bare, `mod testing;`, each `expect`
in it needs one.

## What Another Crate's Tests Need Sits Behind a `testing` Feature

A `#[cfg(test)]` module is invisible across a crate boundary: another crate's
tests never see it. A fixture they need, a counter of drops, a fake store, is a
module behind a `testing` feature, public and hidden from the docs, and the
crate that needs it turns the feature on in its `[dev-dependencies]` alone, so
no build that ships reaches it.

```text
// Bad: crates/tiles/src/lib.rs. `cfg(test)` holds only while tiles' own tests build, so
// tiles-render's tests never see the module, and copy it.
#[cfg(test)]
pub mod testing;
```

```text
# crates/tiles/Cargo.toml
[features]
testing = []

// crates/tiles/src/lib.rs
#[cfg(any(test, feature = "testing"))]
#[doc(hidden)]
pub mod testing;

# crates/tiles-render/Cargo.toml
[dev-dependencies]
tiles = { workspace = true, features = ["testing"] }
```

Held by review. The module's `//!` says why it is public and hidden.

## A Test of a Feature Sits Under That Feature

A test that calls what a feature adds compiles only with the feature on, so it
sits under the same `#[cfg(feature = "…")]` as the code, on the test or on a
module of such tests. Unmarked, it passes in a build with every feature, and
breaks every build without that one.

```text
// Bad: fails to compile without `wide`, where `width` does not exist.
#[test]
fn a_wide_brush_covers_two_squares() {
    assert_eq!(Brush::wide().width(), 2, "two squares");
}
```

```text
#[cfg(feature = "wide")]
#[test]
fn a_wide_brush_covers_two_squares() {
    assert_eq!(Brush::wide().width(), 2, "two squares");
}
```

Held by `just nightly-cargo-hack`, which lints every crate's targets, its tests
among them, with no features and with each feature alone. `just check` builds
every feature at once, so it passes the unmarked test.
