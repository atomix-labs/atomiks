# Errors

Read this before adding or changing an error type, a `Result` a public function
returns, a `?` that crosses a module or crate, a panic, an `expect`, or the
`main` of a binary. It holds the whole model: where errors live, what they are
called, what they say, what they carry, and when code panics instead.

## Errors Live in a Private `errors.rs`, Re-Exported Flat

A caller names `tiles::PlaceError` from the crate root, where it names `Grid`;
the module that holds it is the crate's business. One file a crate is where a
reader finds every way the crate refuses, and its `//!` is the table of them. A
public module that has refusals of its own, one whose path callers write as part
of the API, has its own `errors.rs`, re-exported from that module.

```rust
// Bad: each error sits beside the verb that raises it, so no one file says how
// the crate refuses.
mod grid {
    use thiserror::Error;

    #[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
    #[error("bounds error: square {at} is past the {len} the grid has")]
    pub struct BoundsError {
        pub at: usize,
        pub len: usize,
    }
}

mod tile {
    use thiserror::Error;

    #[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
    #[error("parse tile error: want `t` and digits, like `t42`")]
    pub struct ParseTileError;
}

pub use crate::grid::BoundsError;
pub use crate::tile::ParseTileError;
```

```rust
// src/errors.rs
mod errors {
    //! Why the grid refused.

    use thiserror::Error;

    #[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
    #[error("bounds error: square {at} is past the {len} the grid has")]
    pub struct BoundsError {
        pub at: usize,
        pub len: usize,
    }

    #[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
    #[error("parse tile error: want `t` and digits, like `t42`")]
    pub struct ParseTileError;
}

// src/board/mod.rs: a namespace callers write, `tiles::board::Board`.
pub mod board {
    // src/board/errors.rs
    mod errors {
        //! Why a board file was refused.

        use thiserror::Error;

        #[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
        #[error("board error: the file holds no rows")]
        pub struct EmptyBoardError;
    }

    pub use self::errors::EmptyBoardError;
}

pub use crate::errors::{BoundsError, ParseTileError};
```

Held by review. `mod errors;` is always private and always plural; the
re-exports name each type.

## Derive `Error` with `thiserror`

thiserror's `#[derive(Error)]`, after `use thiserror::Error;`, writes `Display`
from `#[error("…")]` and `Error` with its `source`, so the type states its
message once, beside its fields. thiserror 2 with its default features off works
in a `no_std` crate. Both impls are written by hand only where the derive cannot
say it: a `Display` that hands off to another type's rendering, as an errno's to
`io::Error`'s, or a crate that takes no dependency at all, whose doc says so.

thiserror is the house's derive for an error. Where a crate already derives with
derive_more, and thiserror would bring a second major version of `syn` into its
graph, which cargo-deny's `multiple-versions = "deny"` refuses, derive_more's
`Error` is the choice instead, its message in `#[display("…")]`.

```rust
use core::{error, fmt};

// Bad: two impls by hand in a crate that could take the derive, and the
// message is far from the fields it renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}

impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bounds error: square {} is past the {} the grid has", self.at, self.len)
    }
}

impl error::Error for BoundsError {}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}
```

Held by review. `anyhow` and `eyre` are not used in a library, or anywhere: they
erase the type a caller would match on.

## One Error Type for Each Question a Verb Can Be Asked

A caller matches on what it can do about a refusal. A crate-wide enum makes
every signature claim every failure: `place` seems able to fail with a parse
error, and its caller writes an arm for a refusal its call cannot raise. One
type for each question (is that a square? is that square free? is that a tile
id?) lets the signature say which refusals are possible. A verb that asks two
questions returns a type with an arm for each, and a type that serves two verbs
says which arms each can reach.

```rust
use thiserror::Error;

// Bad: one type for the crate, so `get` and `parse_tile` seem to refuse as
// `place` does.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TilesError {
    #[error("tiles error: square {at} is past the {len} the grid has")]
    OffGrid { at: usize, len: usize },
    #[error("tiles error: square {at} holds a tile already")]
    Occupied { at: usize },
    #[error("tiles error: not a tile id")]
    NotATile,
}
```

```rust
//! Why the grid refused. One type for each question a verb can be asked:
//!
//! | Type             | Asks                             | Returned By             |
//! | ---------------- | -------------------------------- | ----------------------- |
//! | `BoundsError`    | is that a square?                | `Grid::get`             |
//! | `PlaceError`     | a square, and is it free?        | `Grid::place`           |
//! | `ParseTileError` | is that a tile id?               | `TileId::from_str`      |

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error(transparent)]
    Bounds(#[from] BoundsError),
    #[error("place error: square {at} holds a tile already")]
    Occupied { at: usize },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse tile error: want `t` and digits, like `t42`")]
pub struct ParseTileError;
```

Held by review: there is no crate-wide `Error`, and no type is named `Error`.

Under `strict`, `clippy::error_impl_error` refuses a type named `Error`.

## An Error with One Cause Is a Unit Struct

A refusal that can happen one way has nothing to choose between, so an enum of
one variant only makes the caller spell it twice. A unit struct is matched and
compared by its name alone.

```rust
use thiserror::Error;

// Bad: one variant, so every caller writes `GridFullError::Full`.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum GridFullError {
    #[error("grid full error: every square holds a tile")]
    Full,
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("grid full error: every square holds a tile")]
pub struct GridFullError;
```

Held by review.

## Name the Type for Its Question and Each Variant for Its Condition

A type is `<Question>Error`: `BoundsError`, `PlaceError`, `ParseTileError`,
`LoadError`. A variant is a short word for the condition that holds, `Occupied`,
`Past`, `Empty`, `Held`, `Corrupt`, read after the type's name: it never repeats
the type's words or ends in `Error`. Crates whose refusals read better as
something else may keep one family of their own, as a lock's `LockRefused`, and
keep it throughout.

```rust
use thiserror::Error;

// Bad: every variant repeats the type and ends in `Error`.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: square {at} is past the {len} the grid has")]
    PlaceOffGridError { at: usize, len: usize },
    #[error("place error: square {at} holds a tile already")]
    PlaceOccupiedError { at: usize },
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: square {at} is past the {len} the grid has")]
    Past { at: usize, len: usize },
    #[error("place error: square {at} holds a tile already")]
    Occupied { at: usize },
}
```

Held by review: `clippy::enum_variant_names` sees a shared prefix or suffix only
on an enum the crate does not export, and only on an enum of three variants or
more.

## Errors Are `Copy` and `Eq` Where the Payload Allows

A test compares a refusal with `assert_eq!`, and a caller keeps one in a report
without a clone. A payload of plain values allows both, so the derive line is
`Debug, Error, Clone, Copy, PartialEq, Eq`; a payload that owns something, an
`io::Error` or a value handed back, derives what it can.

```rust
use thiserror::Error;

// Bad: plain fields, but a test cannot compare it and a caller cannot copy it.
#[derive(Debug, Error)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn get(&self, at: usize) -> Result<u8, BoundsError> {
        self.squares.get(at).copied().ok_or(BoundsError { at, len: self.squares.len() })
    }
}

#[cfg(test)]
mod tests {
    use super::{BoundsError, Grid};

    #[test]
    fn a_square_past_the_end_is_refused_with_the_length() {
        let grid = Grid { squares: vec![0; 9] };
        assert_eq!(grid.get(9), Err(BoundsError { at: 9, len: 9 }), "one past the last");
    }
}
```

Held by review.

Under `strict`, `clippy::derive_partial_eq_without_eq` asks for the `Eq` beside
a `PartialEq`.

## A Message Is `<type words> error: <fragment>`, Its Fields Inline

A reporter prints each error in a chain on one line, so each message starts with
the words of the type that refused, then a lowercase fragment with no final
period that states what was true, with the fields that make it a fact inline. A
line read alone then names its source, and a chain reads as one sentence. A name
or an acronym inside the fragment keeps its case.

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    // Bad: capitalized, a full stop, no source, and no square.
    #[error("The square is already taken.")]
    Occupied { at: usize },
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: square {at} holds a tile already")]
    Occupied { at: usize },
}
```

Held by review.

## Fields Say What Was Wanted and What Was Held

The fields of every error read alike, so a reader knows a field's side before
its type: `want` is what was asked for and `held` what was found instead; `need`
is what the operation requires and `have` what there was; `at` is an index and
`len` the length it overshot. Fields are declared in that order, `want` before
`held` and `need` before `have`.

```rust
use thiserror::Error;

// Bad: words of this type's own, so each error reads differently.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("rows error: the grid expects {expected_rows} rows, the board has {actual_rows}")]
pub struct RowsError {
    pub expected_rows: u16,
    pub actual_rows: u16,
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("rows error: the grid needs {need} rows, the board has {have}")]
pub struct RowsError {
    pub need: u16,
    pub have: u16,
}
```

Held by review.

## Context Is a Type, Not a String

What the code was doing when it failed is one of a known few things, so it is an
enum with a `Display`, not a `String` built at each call site. A type costs no
allocation on the error path, cannot be misspelt, and can be matched. Where the
same shape recurs, one crate-private constructor builds it.

```rust
use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadError {
    // Bad: the context is a string each caller formats.
    #[error("load error: {context}: {cause}")]
    Io { context: String, cause: io::Error },
}
```

```rust
use std::io;

use derive_more::Display;
use thiserror::Error;

/// What a board load was doing when the file system refused.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum IoOp {
    #[display("open the board file")]
    Open,
    #[display("read the board file")]
    Read,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("load error: i/o error during {op}: {cause}")]
    Io { op: IoOp, cause: io::Error },
}

impl LoadError {
    /// The one place an `io::Error` becomes a load error.
    pub(crate) const fn io(op: IoOp, cause: io::Error) -> Self {
        Self::Io { op, cause }
    }
}

pub fn read<R: io::Read>(file: &mut R) -> Result<Vec<u8>, LoadError> {
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|cause| LoadError::io(IoOp::Read, cause))?;
    Ok(bytes)
}
```

Held by review.

## An Error Renders Its Cause or Exposes It, Never Both

A reporter walks `source()` and prints each error in the chain; an error that
also writes its cause into its own message prints that cause twice. An error
that wraps another across a crate or module boundary is `#[error(transparent)]`
with `#[from]`, so the chain reads as the lower error alone. An error that adds
a fact renders its cause in its message, in a field not named `source` and with
no `#[source]`, so thiserror does not expose it too.

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse tile error: want `t` and digits, like `t42`")]
pub struct ParseTileError;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    // Bad: the message prints the cause, and `#[from]` hands it out again.
    #[error("load error: {0}")]
    Tile(#[from] ParseTileError),
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse tile error: want `t` and digits, like `t42`")]
pub struct ParseTileError;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    #[error(transparent)]
    Tile(#[from] ParseTileError),
    #[error("load error: row {row} has {have} squares, the grid needs {need}")]
    Short { row: u16, need: u16, have: u16 },
}

#[cfg(test)]
mod tests {
    use super::{LoadError, ParseTileError};

    #[test]
    fn a_wrapped_error_renders_once() {
        let wrapped = LoadError::from(ParseTileError);
        assert_eq!(wrapped.to_string(), ParseTileError.to_string(), "no second prefix");
    }
}
```

Held by review, and by a test that pins the rendering, as above.

## `#[from]` Only Where the Lower Error Means One Thing

`#[from]` lets `?` convert silently, which is right only when the lower error
can arrive for one reason. Where it can arrive for two, as a bounds refusal for
either end of a move, a conversion would lose which: each reason is its own
variant with no `#[from]`, whose message says what the cause meant and renders
it, and the call site writes `.map_err(MoveError::Source)`. An error from a
caller's closure, which the crate cannot interpret, is prefixed the same way.

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    // Bad: `?` converts either end's refusal, and the caller cannot tell which.
    #[error(transparent)]
    Bounds(#[from] BoundsError),
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    #[error("move error: the square to move from is off the grid: {0}")]
    Source(BoundsError),
    #[error("move error: the square to move to is off the grid: {0}")]
    Target(BoundsError),
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<Option<u8>>,
}

impl Grid {
    pub fn get(&self, at: usize) -> Result<Option<u8>, BoundsError> {
        self.squares.get(at).copied().ok_or(BoundsError { at, len: self.squares.len() })
    }

    pub fn swap(&mut self, from: usize, to: usize) -> Result<(), MoveError> {
        self.get(from).map_err(MoveError::Source)?;
        self.get(to).map_err(MoveError::Target)?;
        self.squares.swap(from, to);
        Ok(())
    }
}
```

Held by review. Where one error's arms each belong to another, thiserror cannot
flatten it, and `From` is written by hand, one arm for each: where a fill's
`FillError` has a `Past` and an `Occupied` arm of its own, `impl
From<PlaceError> for FillError` sends `PlaceError::Past` to `FillError::Past`
and `PlaceError::Occupied` to `FillError::Occupied`.

## A Refused Value Goes Back to the Caller

A verb that takes a value and refuses it hands the value back inside the error,
so a refusal costs nothing and loses nothing: the caller retries, reroutes or
drops it, as it chooses. The field is public; a payload that has no `Debug` is
skipped with `#[debug(skip)]`, from derive_more's `Debug`, imported in place of
std's, so the error stays printable whatever it holds.

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    #[error("place error: square {at} is past the {len} the grid has")]
    Past { at: usize, len: usize },
    #[error("place error: square {at} holds a tile already")]
    Occupied { at: usize },
}

#[derive(Debug)]
pub struct Grid<T> {
    squares: Vec<Option<T>>,
}

impl<T> Grid<T> {
    // Bad: a refused tile is dropped, so a caller must clone before it asks.
    pub fn place(&mut self, at: usize, tile: T) -> Result<(), PlaceError> {
        let len = self.squares.len();
        match self.squares.get_mut(at) {
            Some(square @ None) => {
                *square = Some(tile);
                Ok(())
            },
            Some(Some(_)) => Err(PlaceError::Occupied { at }),
            None => Err(PlaceError::Past { at, len }),
        }
    }
}
```

```rust
use derive_more::Debug;
use thiserror::Error;

/// Why a tile was not placed, with the tile, unchanged.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError<T> {
    #[error("place error: square {at} is past the {len} the grid has")]
    Past {
        at: usize,
        len: usize,
        #[debug(skip)]
        tile: T,
    },
    #[error("place error: square {at} holds a tile already")]
    Occupied {
        at: usize,
        #[debug(skip)]
        tile: T,
    },
}

impl<T> PlaceError<T> {
    /// The tile that was refused, whichever the reason.
    pub fn into_tile(self) -> T {
        match self {
            Self::Past { tile, .. } | Self::Occupied { tile, .. } => tile,
        }
    }
}

#[derive(Debug)]
pub struct Grid<T> {
    squares: Vec<Option<T>>,
}

impl<T> Grid<T> {
    pub fn place(&mut self, at: usize, tile: T) -> Result<(), PlaceError<T>> {
        let len = self.squares.len();
        match self.squares.get_mut(at) {
            Some(square @ None) => {
                *square = Some(tile);
                Ok(())
            },
            Some(Some(_)) => Err(PlaceError::Occupied { at, tile }),
            None => Err(PlaceError::Past { at, len, tile }),
        }
    }
}
```

Held by review.

## An Impossible Arm Is `Infallible`

A type that serves several verbs takes a type parameter for each arm some verbs
cannot reach, and a verb that cannot reach it pins the parameter to
`Infallible`. The arm is then uninhabited: the caller writes an irrefutable
`let` or a `match` with no arm for it, and a helper that sheds it ends in `match
never {}`. No caller writes an arm for something that cannot happen.

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AddError {
    #[error("add error: every square holds a tile")]
    Full,
    // Bad: `add` never fails this way, but its callers must handle it anyway.
    #[error("add error: the tile could not be made: {0}")]
    Failed(String),
}
```

```rust
use core::convert::Infallible;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("grid full error: every square holds a tile")]
pub struct GridFullError;

/// Why a tile was not added. `E` is the maker's own error: `add`, which is
/// given a tile, pins it to `Infallible`, leaving `Failed` uninhabited.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum AddError<E> {
    #[error(transparent)]
    Full(#[from] GridFullError),
    #[error("add error: the tile could not be made: {0}")]
    Failed(E),
}

impl AddError<Infallible> {
    /// The one arm there is, for a verb that was given its tile.
    #[must_use]
    pub const fn full(self) -> GridFullError {
        match self {
            Self::Full(full) => full,
            Self::Failed(never) => match never {},
        }
    }
}

#[derive(Debug)]
pub struct Grid<T> {
    squares: Vec<Option<T>>,
}

impl<T> Grid<T> {
    pub fn add_with<E, F: FnOnce() -> Result<T, E>>(&mut self, make: F) -> Result<usize, AddError<E>> {
        let (at, square) =
            self.squares.iter_mut().enumerate().find(|(_, square)| square.is_none()).ok_or(GridFullError)?;
        *square = Some(make().map_err(AddError::Failed)?);
        Ok(at)
    }

    pub fn add(&mut self, tile: T) -> Result<usize, GridFullError> {
        self.add_with(|| Ok(tile)).map_err(AddError::full)
    }
}
```

Held by review.

## Retry Semantics Live in the Types

Whether another attempt could succeed is the first thing a caller asks, so the
type answers it, not the caller's reading of a message. A function whose
refusals split by permanence returns `TryError<N, F>`, defined once in the
crate's `errors.rs`: `NonFatal` another attempt could beat, `Fatal` none would,
so a retry loop has no verdict of its own to get wrong. Where one variant alone
is transient, its doc says so: "a caller may retry".

```rust
use thiserror::Error;

// Bad: nothing says which refusal a second attempt could beat, so each caller
// guesses, and one retries a corrupt board forever.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    #[error("claim error: another editor holds the board")]
    Held,
    #[error("claim error: the board's header names no board")]
    Corrupt,
}
```

```rust
use core::hint;

use thiserror::Error;

/// A refusal another attempt could beat, or a fault none would.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryError<N, F> {
    NonFatal(N),
    Fatal(F),
}

/// Transient: another editor holds the board, so a caller may retry.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("held error: another editor holds the board")]
pub struct HeldError;

/// Permanent: the header names no board, and no attempt will change that.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("corrupt error: the board's header names no board")]
pub struct CorruptError;

/// Attempts up to `attempts` times, waiting out each refusal another attempt
/// could beat; the last refusal comes back once they are spent.
pub fn waited<T, N, F, A>(attempts: u32, mut attempt: A) -> Result<T, TryError<N, F>>
where
    A: FnMut() -> Result<T, TryError<N, F>>,
{
    let mut left = attempts;
    loop {
        match attempt() {
            Err(TryError::NonFatal(_held)) if left > 1 => {
                left = left.saturating_sub(1);
                hint::spin_loop();
            },
            done => return done,
        }
    }
}
```

Held by review.

## Alias an Error Type, Never `Result`

An alias of `Result` hides the error at every signature that uses it, and one
named `Result` shadows the prelude's in every module that imports it. An alias
of an error type names a long type once and leaves each signature saying what it
returns.

```rust
use core::result;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("grid full error: every square holds a tile")]
pub struct GridFullError;

// Bad: every signature reads `Result<usize>`, and says nothing of the error.
pub type Result<T> = result::Result<T, GridFullError>;
```

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryError<N, F> {
    NonFatal(N),
    Fatal(F),
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("held error: another editor holds the board")]
pub struct HeldError;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("corrupt error: the board's header names no board")]
pub struct CorruptError;

/// What claiming a board answers with.
pub type ClaimError = TryError<HeldError, CorruptError>;

pub const fn claim(held: bool) -> Result<(), ClaimError> {
    if held { Err(TryError::NonFatal(HeldError)) } else { Ok(()) }
}
```

Held by review.

## A Binary Fails with a Boxed Error

A binary whose failure only an operator reads, an example or a bench reports a
failure and stops; nobody matches on it. It returns `Result<(), BoxError>`,
where `BoxError` boxes any error that is `Send` and `Sync`, so `?` converts
every library error into it, and a one-off failure is `io::Error::other("…")`:
no error type is defined for a failure only an operator reads. A dropped cause
is named: a thread's panic payload is `_panicked`. A command a person runs fails
the same way inside, and its `main` reports the message instead, as "A Command a
Person Runs Reports the Error's Message" says.

```rust
// Bad: a `String` loses the chain, and every `?` needs a `map_err` first.
fn squares(cols: u16, rows: u16) -> Result<u16, String> {
    cols.checked_mul(rows).ok_or_else(|| String::from("the board is too big"))
}

fn main() -> Result<(), String> {
    squares(3, 3).map(drop)
}
```

```rust
use core::error::Error;
use std::{io, thread};

/// What a failure in this binary is.
type BoxError = Box<dyn Error + Send + Sync>;

fn squares(cols: u16, rows: u16) -> Result<u16, BoxError> {
    let squares = cols.checked_mul(rows);
    Ok(squares.ok_or_else(|| io::Error::other("the board has more squares than a u16 counts"))?)
}

fn main() -> Result<(), BoxError> {
    let render = thread::spawn(|| squares(3, 3));
    let drawn = render.join().map_err(|_panicked| io::Error::other("the render thread panicked"))??;
    if drawn == 0 {
        return Err(io::Error::other("an empty board has nothing to draw").into());
    }
    Ok(())
}
```

Held by review.

## A Command a Person Runs Reports the Error's Message

A command a person runs is a command line a user types, whose failure the person
who typed it reads; a service, a daemon or a tool whose output only an operator
reads is not one. When `main` returns an `Err`, the standard library prints
`Error:` and the error's `Debug` form, which names a type and its fields where
the person needs to know what went wrong. So such a command's `main` returns
`ExitCode` and calls a `run` that returns `Result<(), BoxError>`. On an `Err`,
it prints `error: {error}` to stderr, then each error beneath it as `caused by:
{cause}`, and returns `ExitCode::FAILURE`.

Walking `source()` prints each fact once, because an error renders its cause or
exposes it, never both: the standard library's `Error` docs ask it of an error
that wraps another, and "An Error Renders Its Cause or Exposes It, Never Both"
holds it here. An error that renders its cause exposes nothing and prints as one
line; one that exposes its cause, as an error from another crate may, adds a
`caused by:` line for it. The message alone would drop a cause that an error
exposes and does not render. `main` prints under an
`#[expect(clippy::print_stderr)]` whose reason says why.

```rust
use core::error;
use std::env;

use thiserror::Error;

/// What a failure in this command is.
type BoxError = Box<dyn error::Error + Send + Sync>;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
struct BoundsError {
    at: usize,
    len: usize,
}

// Bad: `tiles 12` prints `Error: BoundsError { at: 12, len: 9 }`, the error's `Debug`.
fn main() -> Result<(), BoxError> {
    let at = env::args().nth(1).unwrap_or_default().parse()?;
    if at >= 9 {
        return Err(BoundsError { at, len: 9 }.into());
    }
    Ok(())
}
```

```rust
use core::{error, iter};
use std::env;
use std::process::ExitCode;

use thiserror::Error;

/// What a failure in this command is.
type BoxError = Box<dyn error::Error + Send + Sync>;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
struct BoundsError {
    at: usize,
    len: usize,
}

fn run() -> Result<(), BoxError> {
    let at = env::args().nth(1).unwrap_or_default().parse()?;
    if at >= 9 {
        return Err(BoundsError { at, len: 9 }.into());
    }
    Ok(())
}

#[expect(clippy::print_stderr, reason = "stderr is where a person reads why the command failed")]
fn main() -> ExitCode {
    let Err(error) = run() else { return ExitCode::SUCCESS };
    eprintln!("error: {error}");
    for cause in iter::successors(error.source(), |cause| cause.source()) {
        eprintln!("caused by: {cause}");
    }
    ExitCode::FAILURE
}
```

```text
$ tiles 12
error: bounds error: square 12 is past the 9 the grid has
```

Held by review.

Under `strict`, `clippy::print_stderr` refuses the `eprintln!` without its
`#[expect]`, as "Print Only from a Binary, with a Reason" in
`references/lints.md` says.

## Return an Error for Anything a Caller Can Cause

A library cannot know whether its caller can recover, so anything a caller can
cause, an index, a string, a file, a full grid, is refused with an error, and
the caller decides. A refusal is never clamped into something else: an index
past the end that silently becomes the last square hides the caller's mistake.

```rust
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: a caller's index panics the process instead of being refused.
    #[must_use]
    pub fn get(&self, at: usize) -> Option<&u8> {
        assert!(at < self.squares.len(), "the square is on the grid");
        self.squares.get(at)
    }
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("bounds error: square {at} is past the {len} the grid has")]
pub struct BoundsError {
    pub at: usize,
    pub len: usize,
}

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn get(&self, at: usize) -> Result<&u8, BoundsError> {
        self.squares.get(at).ok_or(BoundsError { at, len: self.squares.len() })
    }
}
```

Held by review, and by `clippy::missing_panics_doc`, which asks a public
function that can panic for a `# Panics` section.

Under `strict`, `clippy::panic`, `unwrap_used`, `expect_used`,
`indexing_slicing`, `unreachable`, `todo`, `unimplemented`, `panic_in_result_fn`
and `unwrap_in_result` refuse the panicking forms.

The workspace's `clippy.toml` lets a test unwrap, expect, panic and index.

## Panic Only for a Broken Invariant, and Say Which

A panic is for a state the code promised could not happen. Two kinds of function
may panic: one whose precondition the process sets up at startup, and one that
mirrors a `core` name, `expect` or `unwrap`, whose caller asked for the panic.
Such a function carries `#[track_caller]`, so the report points at the caller
that broke the promise, and a `# Panics` section saying when; its `expect`
message states the precondition as an instruction.

A check a release build need not pay for is a `debug_assert!` whose message
names the violation it catches, as a fact, so a failure reads as what went
wrong: an invariant the type's own code keeps, or the contract an `unsafe`
function states, which `writing-unsafe-rust` teaches. A safe function's input is
never such a check: it is refused with an error, as above.

```rust
use core::num::NonZeroU16;

#[derive(Debug)]
pub struct Grid {
    cols: NonZeroU16,
    squares: Vec<u8>,
}

impl Grid {
    /// A grid of `cols` columns and `rows` rows, every square blank.
    #[must_use]
    pub fn new(cols: NonZeroU16, rows: u16) -> Self {
        let squares = usize::from(cols.get()).saturating_mul(usize::from(rows));
        Self { cols, squares: vec![0; squares] }
    }

    /// The grid's rows, each `cols` squares long.
    pub fn rows(&self) -> impl Iterator<Item = &[u8]> {
        // `new` lays whole rows, and nothing changes the length after it: an
        // invariant of this type's own, which no caller can break.
        debug_assert!(
            self.squares.len().is_multiple_of(usize::from(self.cols.get())),
            "a row cut short at the grid's end"
        );
        self.squares.chunks(usize::from(self.cols.get()))
    }
}
```

Under `strict`, a function that panics says in an `#[expect]` why the lint is
wrong there:

```rust,compile_fail
// fails: clippy::expect_used
use std::sync::OnceLock;

static TILESET: OnceLock<Vec<char>> = OnceLock::new();

#[must_use]
pub fn tileset() -> &'static [char] {
    // Bad: a message that says nothing, and no word on why this may panic.
    TILESET.get().expect("failed")
}
```

```rust
use std::sync::OnceLock;

static TILESET: OnceLock<Vec<char>> = OnceLock::new();

/// The glyphs every grid draws with.
///
/// # Panics
/// `install` has not run in this process: a wiring fault, which fires on the
/// first call of a run that never installed and on no call of one that did.
#[must_use]
#[track_caller]
#[expect(clippy::expect_used, reason = "the tile set is a startup precondition; see `# Panics`")]
pub fn tileset() -> &'static [char] {
    TILESET.get().expect("install the tile set before drawing a grid")
}
```

Held by review.

Under `strict`, `clippy::expect_used` and `clippy::panic` make each function
that panics state its reason in an `#[expect]`.

## An `expect` Says Why It Cannot Fail

In a test or an example, an `expect` message says why the call cannot fail here,
so a failure reads as the broken assumption; in library code it states the
precondition, as above. It never restates the call, and it never says "BUG" or
"failed": the panic already says so.

```rust
#[must_use]
pub fn centre(squares: &[u8]) -> Option<&u8> {
    squares.get(squares.len().checked_div(2)?)
}

#[cfg(test)]
mod tests {
    use super::centre;

    #[test]
    fn a_grid_of_nine_has_a_centre() {
        // Bad: says what failed, which the panic says anyway.
        let square = centre(&[0; 9]).expect("failed to get the centre");
        assert_eq!(*square, 0, "an empty square");
    }
}
```

```rust
#[must_use]
pub fn centre(squares: &[u8]) -> Option<&u8> {
    squares.get(squares.len().checked_div(2)?)
}

#[cfg(test)]
mod tests {
    use super::centre;

    #[test]
    fn a_grid_of_nine_has_a_centre() {
        let square = centre(&[0; 9]).expect("nine squares have a fifth");
        assert_eq!(*square, 0, "an empty square");
    }
}
```

Held by review.

An example that uses `expect` throughout says so once at its top:
`#![expect(clippy::expect_used, reason = "an example reports a broken invariant
by dying loudly")]`.

A test needs no such attribute: the workspace's `clippy.toml` lets a `#[test]`
function and a `#[cfg(test)]` module expect, so a file-wide `#[expect]` there
goes unfulfilled and fails the build. Only a shared helper outside both needs
one, on the helper.

## An Error Is Handled, or Dropped by Name

A `Result` nobody reads is a refusal nobody saw, so the compiler refuses an
unused one. Where dropping the error is the decision, the code names what it
drops, `|_gone|`, `Err(_held)`, `Ok(_vacant)`, so a reader sees it was chosen;
`.map(drop)` drops an `Ok` value the caller does not need.

```rust,compile_fail
// fails: unused_must_use
use std::sync::mpsc::Sender;

pub fn announce(tiles: &Sender<u8>) {
    // Bad: a send to a render thread that ended is lost without a word.
    tiles.send(7);
}
```

```rust
use std::sync::mpsc::Sender;

use thiserror::Error;

/// The render thread has ended, so a tile sent to it would never be drawn.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("render gone error: the render thread ended before the tile was sent")]
pub struct RenderGoneError;

pub fn announce(tiles: &Sender<u8>) -> Result<(), RenderGoneError> {
    tiles.send(7).map_err(|_gone| RenderGoneError)
}
```

Held by `unused_must_use`.

Under `strict`, `clippy::let_underscore_must_use` and `clippy::unused_result_ok`
refuse `let _ =` and `.ok()` on a `Result`, and `clippy::map_err_ignore` refuses
`|_|`.
