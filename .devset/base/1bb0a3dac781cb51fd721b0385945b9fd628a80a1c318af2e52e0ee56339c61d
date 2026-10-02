# Naming

Read this before naming a crate, a module, a type, a trait, a function, a
method, a constant or a field, and before renaming one. The names follow the
Rust API Guidelines, and the workspace adds families of its own, so that a name
says what a call costs and how it can refuse before anyone reads its body.

## A Type Parameter Is One Capital Letter

Each type parameter is one capital letter, however many a type has, so a
parameter never reads as a type: `T` for the value, `E` for an error, `N` and
`F` for a refusal that is not fatal and one that is, `A` for an attempt, `I` for
an initializer. A lifetime is short, `'a`, or names what it borrows where two
meet, `'grid`. Casing itself, RFC 430's, is rustc's to hold: `lints.md` lists
the lints.

```rust
// Bad: a parameter that reads as a type, and could be taken for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryError<Refusal, Fault> {
    NonFatal(Refusal),
    Fatal(Fault),
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryError<N, F> {
    NonFatal(N),
    Fatal(F),
}
```

Held by review.

## An Acronym Is One Word

An acronym or a contraction is cased as a word, `TileId`, `Utf8`, `Json`, `Csv`,
so where one word ends and the next begins stays visible: `HttpJsonReader`, not
`HTTPJSONReader`.

```rust
// Bad: acronyms in capitals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    JSON,
    CSV,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileID(u32);
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Json,
    Csv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);
```

Held by review: `clippy::upper_case_acronyms` sees only a name the crate does
not export, and there only one written wholly in capitals, `JSON`; `TileID`
needs its `upper-case-acronyms-aggressive` option, which is off.

## An Associated Constant Names the Value

A value a type has one of is an associated constant named for what it is,
`Pos::ORIGIN`, `Tile::BLANK`, `Grid::MAX_COLS`, `ZERO`, `MAX`, `MIN`, not a
function that returns it and not a free constant with the type's name inside.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

// Bad: a free constant that repeats the type's name.
pub const POS_ORIGIN: Pos = Pos { col: 0, row: 0 };
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl Pos {
    /// The top-left square.
    pub const ORIGIN: Self = Self { col: 0, row: 0 };
}
```

Held by review.

## A Getter Is Its Noun, Without `get_`

A method that reads a field or a property is named for what it returns,
`cols()`, `len()`, `capacity()`, since the call's parentheses already say it is
read. A newtype's one value is `get()`. `get_` appears only where it mirrors a
name of `std`, `get_mut` and `get_unchecked`, which index.

```rust
#[derive(Debug)]
pub struct Grid {
    cols: u16,
    rows: u16,
}

impl Grid {
    // Bad: `get_` says nothing the parentheses do not.
    #[must_use]
    pub const fn get_cols(&self) -> u16 {
        self.cols
    }

    #[must_use]
    pub const fn get_rows(&self) -> u16 {
        self.rows
    }
}
```

```rust
#[derive(Debug)]
pub struct Grid {
    cols: u16,
    rows: u16,
}

impl Grid {
    #[must_use]
    pub const fn cols(&self) -> u16 {
        self.cols
    }

    #[must_use]
    pub const fn rows(&self) -> u16 {
        self.rows
    }
}
```

Held by review.

## `as_`, `to_` and `into_` Say What a Conversion Costs

The prefix is the price. `as_` is free and borrows, `&self` to a view, or reads
a `Copy` value in another unit; `to_` does work, allocating or computing, from
`&self`, or from `self` for a `Copy` type; `into_` consumes `self` and hands its
parts on. A type that renders as text implements `Display`, which gives
`to_string`; it never defines a `to_string` of its own.

```rust
#[derive(Debug)]
pub struct Row {
    squares: Vec<u8>,
}

impl Row {
    // Bad: `to_` on a free borrow, and `as_` on an allocation.
    #[must_use]
    pub fn to_slice(&self) -> &[u8] {
        &self.squares
    }

    #[must_use]
    pub fn as_owned(&self) -> Vec<u8> {
        self.squares.clone()
    }
}
```

```rust
#[derive(Debug)]
pub struct Row {
    squares: Vec<u8>,
}

impl Row {
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.squares
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.squares.clone()
    }

    #[must_use]
    pub fn into_squares(self) -> Vec<u8> {
        self.squares
    }
}
```

Held by review, which alone holds the cost. `clippy::wrong_self_convention`
holds the receiver each prefix takes, a reference for `as_`, `self` for `into_`,
`&self` for `to_` or `self` on a `Copy` type, and only on a method the crate
does not export; `clippy::inherent_to_string` refuses a `to_string` of a type's
own.

## `try_` Can Refuse, `_with` Takes a Closure, `_in` Takes an Allocator

A family of functions over one operation reads by its affixes. `try_` is the one
that refuses at once where the plain one waits, grows or panics: `claim` waits
for the board, `try_claim` refuses when another editor holds it. `_with` runs a
closure the caller passes, `fill_with`, `read_with`; `_in` takes an allocator,
after the `allocator_api` names, `new_in`, `with_capacity_in`. `raw_` is the
version over raw pointers, and `_unchecked` the unsafe one that skips a check.

```rust
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: a closure version with a name of its own, so the pair does not read as
    // one operation.
    pub fn fill(&mut self, tile: u8) {
        self.squares.fill(tile);
    }

    pub fn fill_by_calling<F: FnMut() -> u8>(&mut self, make: F) {
        self.squares.fill_with(make);
    }
}
```

```rust
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn fill(&mut self, tile: u8) {
        self.squares.fill(tile);
    }

    pub fn fill_with<F: FnMut() -> u8>(&mut self, make: F) {
        self.squares.fill_with(make);
    }
}
```

Held by review.

## `new` Builds, `create` Lays, `open` Binds

`new` builds a value from its parts and `from_` converts one, `from_raw` and
`into_raw` going both ways; `with_capacity` sizes one. A resource that outlives
the process, a file or a shared segment, has three constructors: `create` lays a
new one and refuses if one is there, `open` binds to one that exists and refuses
if none is, and `open_or_create` does either. `install` sets a value the whole
process shares, once.

```rust
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct Board {
    file: File,
}

impl Board {
    // Bad: `new` for a constructor that may lay a file or bind to one, which a caller
    // cannot tell from the name, and which clobbers what it finds.
    pub fn new(path: &Path) -> io::Result<Self> {
        Ok(Self { file: File::create(path)? })
    }

    pub fn is_empty(&self) -> io::Result<bool> {
        Ok(self.file.metadata()?.len() == 0)
    }
}
```

```rust
use std::fs::File;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct Board {
    file: File,
}

impl Board {
    pub fn create(path: &Path) -> io::Result<Self> {
        Ok(Self { file: File::create_new(path)? })
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self { file: File::options().read(true).write(true).open(path)? })
    }

    pub fn open_or_create(path: &Path) -> io::Result<Self> {
        match Self::open(path) {
            Err(absent) if absent.kind() == io::ErrorKind::NotFound => Self::create(path),
            opened => opened,
        }
    }

    pub fn is_empty(&self) -> io::Result<bool> {
        Ok(self.file.metadata()?.len() == 0)
    }
}
```

Held by review.

## A Boolean Reads as a Question

A method that answers yes or no starts `is_`, `has_` or `can_`: `is_empty`,
`is_full`, `has_tile`. A type with `len` has `is_empty` beside it.

```rust,compile_fail
// fails: clippy::len_without_is_empty
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: a length with no `is_empty`, so callers write `len() == 0`.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.squares.len()
    }
}
```

```rust
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.squares.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.squares.is_empty()
    }
}
```

Held by `clippy::len_without_is_empty`.

## Iterators Are `iter`, `iter_mut` and `into_iter`

A collection's iterators have the names `std`'s have, and `for` works on it as
on a slice: `iter` has an `IntoIterator` for `&Grid` beside it, and `iter_mut`
one for `&mut Grid`. An iterator type is named for the method that returns it,
`Iter`, `IterMut`, `IntoIter`, `Squares` for `squares()`.

```rust,compile_fail
// fails: clippy::iter_without_into_iter
use core::slice;

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: `for square in &grid` does not compile.
    pub fn iter(&self) -> slice::Iter<'_, u8> {
        self.squares.iter()
    }
}
```

```rust
use core::slice;

#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn iter(&self) -> slice::Iter<'_, u8> {
        self.squares.iter()
    }
}

impl<'a> IntoIterator for &'a Grid {
    type Item = &'a u8;
    type IntoIter = slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
```

Held by `clippy::iter_without_into_iter`.

## A Type's Name Says Its Role: `Spec`, `Config`, `Guard`, `Error`

The last word of a type's name says what it is for. A `*Spec` holds the
parameters of one call that creates or opens something, `Grid::new(GridSpec { …
})`, built as a literal; a `*Config` holds a program's settings, as loaded from
a file or the environment; a `*Guard` holds something until it drops; an
`*Error` is a refusal. A marker type is an adjective or a role, `Editing`,
`Sealed`, `Shared`. No struct is a catch-all `*Options` or `*Params`: `std`'s
`OpenOptions` is a builder, a different shape, and no model for a struct of
parameters.

```rust
// Bad: a catch-all name, which says neither whose parameters these are nor
// where they come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridOptions {
    pub cols: u16,
    pub rows: u16,
    pub theme: String,
}
```

```rust
use std::env;

/// What a grid is laid with: the parameters of `Grid::new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSpec {
    pub cols: u16,
    pub rows: u16,
    pub wrap: Wrap,
}

/// Whether a move off one edge comes back on the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    Edges,
    Torus,
}

/// The editor's settings, as its environment gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorConfig {
    pub theme: String,
}

impl EditorConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self { theme: env::var("TILES_THEME").unwrap_or_else(|_unset| String::from("plain")) }
    }
}
```

Held by review.

## One Word for One Thing

The domain's word for a thing is used everywhere it appears: in names, docs,
messages and tests. A second word for the same thing makes a reader wonder
whether it is a second thing. A crate that has settled a word says so in its
docs: "square, never cell".

```rust
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: a square here, a cell there, for the same place.
    #[must_use]
    pub fn square(&self, at: usize) -> Option<u8> {
        self.squares.get(at).copied()
    }

    #[must_use]
    pub const fn cell_count(&self) -> usize {
        self.squares.len()
    }
}
```

```rust
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    #[must_use]
    pub fn square(&self, at: usize) -> Option<u8> {
        self.squares.get(at).copied()
    }

    #[must_use]
    pub const fn square_count(&self) -> usize {
        self.squares.len()
    }
}
```

Held by review.
