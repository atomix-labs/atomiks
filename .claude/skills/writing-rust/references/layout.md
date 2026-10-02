# Layout

Read this before adding a crate, a module, a file or a re-export, or before
changing what a crate makes public. It says where code goes and what callers see
of it.

## `lib.rs` Holds Docs, Attributes, Modules, Then Re-Exports

A reader opens `lib.rs` to learn what the crate is and what it offers, so it
holds nothing else, in one order: the `//!` crate doc, the inner attributes, any
`extern crate`, the `mod` lines in alphabetical order, the test fixtures'
`#[cfg(test)] mod testing;`, then the `pub use` lines that make the crate's
surface. Items are defined in modules, never in `lib.rs`.

```text
// Bad: an item defined among the modules, and a re-export before them.
//! A grid of tiles.
pub use crate::grid::Grid;
mod grid;
pub struct Pos { pub col: u16, pub row: u16 }
mod errors;
```

```text
//! A grid of tiles, and the moves across it.
//!
//! # Crate features
//! ...

#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod errors;
mod grid;
mod moves;
#[cfg(test)]
mod testing;

pub use crate::errors::{BoundsError, MoveError};
pub use crate::grid::{Grid, Pos};
pub use crate::moves::Move;
```

Held by review.

## Modules Are Private, and the Root Re-Exports What Callers Name

A caller writes `tiles::Pos`, not `tiles::grid::Pos`: the module a type lives in
is the crate's to change, and a private module with a flat re-export lets it
move a type without breaking anyone. A module is `pub` only where its path is
part of the API, a namespace callers are meant to write, as `tiles::board` for a
board file's types, and then on purpose.

```rust
// Bad: every module is public, so the file a type lives in is part of the API.
pub mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Pos {
        pub col: u16,
        pub row: u16,
    }
}

pub mod tile {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TileId(pub u32);
}
```

```rust
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Pos {
        pub col: u16,
        pub row: u16,
    }
}

mod tile {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TileId(pub u32);
}

pub use crate::grid::Pos;
pub use crate::tile::TileId;
```

Held by review. The workspace has no prelude module: a caller imports the few
names it uses.

## Re-Export by Name, Not by Glob

A glob re-exports whatever the module grows, an item meant to stay inside
included, and a reader cannot tell where a name comes from. Each re-export names
its items. The one glob that earns its place says why beside it, as one that
lets an explicit import shadow one of its names.

```rust
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Pos {
        pub col: u16,
        pub row: u16,
    }
}

// Bad: whatever `grid` makes public next is exported too.
pub use crate::grid::*;
```

```rust
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Pos {
        pub col: u16,
        pub row: u16,
    }
}

pub use crate::grid::Pos;
```

Held by review, and by `clippy::wildcard_imports`, which refuses a glob `use`
that is not a re-export.

## Inside a Private Module, What the Crate Does Not Export Is `pub(crate)`

`pub` on an item says it is part of the crate's surface. Inside a private
module, an item the crate does not re-export is `pub(crate)`, or `pub(super)`
where only the parent module and what it holds use it, so a reader knows from
the item itself how far it reaches. `pub(in path)` is not used.

```rust,compile_fail
// fails: unreachable_pub
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Pos {
        pub col: u16,
        pub row: u16,
    }

    // Bad: `pub`, but nothing outside the crate can reach it.
    pub fn index(at: Pos, cols: u16) -> Option<usize> {
        usize::from(at.row).checked_mul(usize::from(cols))?.checked_add(usize::from(at.col))
    }

    impl Pos {
        #[must_use]
        pub fn index(self, cols: u16) -> Option<usize> {
            index(self, cols)
        }
    }
}

pub use crate::grid::Pos;
```

```rust
mod grid {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Pos {
        pub col: u16,
        pub row: u16,
    }

    pub(crate) fn index(at: Pos, cols: u16) -> Option<usize> {
        usize::from(at.row).checked_mul(usize::from(cols))?.checked_add(usize::from(at.col))
    }

    impl Pos {
        #[must_use]
        pub fn index(self, cols: u16) -> Option<usize> {
            index(self, cols)
        }
    }
}

pub use crate::grid::Pos;
```

Held by review.

Under `strict`, `unreachable_pub` refuses a `pub` item no path outside the crate
reaches, `clippy::redundant_pub_crate`, which would call `pub(crate)` redundant
inside a private module, is allowed, and `clippy::pub_without_shorthand` refuses
`pub(in super)` for `pub(super)`.

## A Module with Children Is `mod.rs`

A module's own code and its children sit in one directory, so a reader finds all
of it in one place, and a leaf module is one file. Code for one platform splits
into `sys/mod.rs`, which re-exports with `pub(super) use`, and `sys/linux.rs`.

```text
// Bad: the module's code beside its directory, not in it.
src/grid.rs
src/grid/moves.rs
src/grid/walk.rs
```

```text
src/grid/mod.rs
src/grid/moves.rs
src/grid/walk.rs
src/tile.rs
```

Held by review.

Under `strict`, `clippy::self_named_module_files` refuses `grid.rs` beside a
`grid/` directory. It is that lint that asks for `mod.rs`, and
`clippy::mod_module_files` the reverse: some guides have the two swapped.

## A Module Is Named for What It Holds

A module is a short singular noun for the thing it holds: `grid`, `tile`,
`board`, `walk`. A few names are fixed, so a reader knows each on sight:
`errors.rs`, plural, for the crate's refusals; `testing.rs`, under
`#[cfg(test)]`, for fixtures the crate's tests share, its items `pub(crate)`;
`consts.rs` for constants several modules use; and `tests/testing/mod.rs` for
what integration tests share, a directory Cargo compiles into no test of its
own. No module is `utils`, `helpers` or `common`, which say nothing of what is
inside.

```text
// Bad: names that say nothing of what the module holds.
src/error.rs
src/utils.rs
src/helpers/mod.rs
src/common.rs
src/test_utils.rs
```

```text
src/errors.rs
src/grid/mod.rs
src/grid/walk.rs
src/consts.rs
src/testing.rs
tests/testing/mod.rs
```

Held by review.

What goes in `testing.rs` and `tests/testing/mod.rs`, and how each is declared,
is `writing-rust-tests`'s.

## A Library Is `no_std`, with `std` the Opt-In

A library that needs no operating system works in a kernel, a WASM module or on
a board with no files, so it is `#![no_std]`, reaches for `core`, then `alloc`,
then `std`, and puts what needs the OS (files, threads, clocks) behind a `std`
feature that only adds. A binary uses `std` and still writes `core::` and
`alloc::` paths where they exist.

```rust,compile_fail
// fails: clippy::std_instead_of_core
use std::cmp::Ordering;

#[must_use]
pub fn by_row(a: (u16, u16), b: (u16, u16)) -> Ordering {
    a.1.cmp(&b.1).then(a.0.cmp(&b.0))
}
```

```rust
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::cmp::Ordering;

#[must_use]
pub fn by_row(a: (u16, u16), b: (u16, u16)) -> Ordering {
    a.1.cmp(&b.1).then(a.0.cmp(&b.0))
}

#[must_use]
pub fn sorted(mut squares: Vec<(u16, u16)>) -> Vec<(u16, u16)> {
    squares.sort_by(|a, b| by_row(*a, *b));
    squares
}
```

Held by `clippy::std_instead_of_core` and `clippy::std_instead_of_alloc`.

## What a Macro Needs Is `#[doc(hidden)] pub`, Named with `__`

A `macro_rules!` macro expands in the caller's crate, so what it calls must be
public; it is hidden from the docs and named with a leading `__`, so no caller
takes it for part of the API. The macro reaches it through `$crate::`.

```rust
// Bad: public and documented, so callers take it for part of the API.
#[must_use]
pub fn squares_of(cols: u16, rows: u16) -> Option<u32> {
    u32::from(cols).checked_mul(u32::from(rows))
}

#[macro_export]
macro_rules! squares {
    ($cols:expr, $rows:expr) => {
        $crate::squares_of($cols, $rows)
    };
}
```

```rust
#[doc(hidden)]
#[must_use]
pub fn __squares(cols: u16, rows: u16) -> Option<u32> {
    u32::from(cols).checked_mul(u32::from(rows))
}

#[macro_export]
macro_rules! squares {
    ($cols:expr, $rows:expr) => {
        $crate::__squares($cols, $rows)
    };
}
```

Held by review.
