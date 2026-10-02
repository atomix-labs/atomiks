# API Design

Read this before adding or changing a public type, trait, constructor,
conversion or signature: a type a caller builds, a function it calls, a trait it
implements or uses. It says how the surface is shaped so that wrong calls do not
compile and right ones read plainly.

## Configure with a Spec Struct, Not a Builder

A call that creates or opens something, and takes three parameters or more, or
one a caller may leave at its usual value, takes them as one `*Spec` struct with
public fields, built as a literal and passed as any value is, by value where it
is small and `Copy` and by reference where it is not; one or two plain values
stay arguments, `Board::create(path)`, `Grid::new(cols, rows)`. Every field is
written at the call site, so nothing is configured by a default nobody wrote
down, a missing field is a compile error, and a reader sees the whole
configuration in one place. A usual value is a named constructor or constant of
the spec, `GridSpec::square(8)`, never a `Default` nobody sees. A program's
settings, loaded from a file or the environment, are a `*Config`. A builder is
for construction that is a sequence of steps, as a message encoded field by
field, not for a set of values.

```rust
// Bad: three fields behind a builder, so a forgotten setter is a silent
// default.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GridBuilder {
    cols: u16,
    rows: u16,
    wrap: bool,
}

impl GridBuilder {
    #[must_use]
    pub const fn cols(mut self, cols: u16) -> Self {
        self.cols = cols;
        self
    }

    #[must_use]
    pub const fn rows(mut self, rows: u16) -> Self {
        self.rows = rows;
        self
    }

    #[must_use]
    pub const fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}
```

```rust
/// What a grid is laid with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSpec {
    pub cols: u16,
    pub rows: u16,
    /// Whether a move off one edge comes back on the other.
    pub wrap: Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    Edges,
    Torus,
}

impl GridSpec {
    /// A square grid of `side` squares a side, whose edges stop a move.
    #[must_use]
    pub const fn square(side: u16) -> Self {
        Self { cols: side, rows: side, wrap: Wrap::Edges }
    }
}

#[derive(Debug)]
pub struct Grid {
    spec: GridSpec,
}

impl Grid {
    #[must_use]
    pub const fn new(spec: GridSpec) -> Self {
        Self { spec }
    }

    #[must_use]
    pub const fn spec(&self) -> GridSpec {
        self.spec
    }
}

#[must_use]
pub const fn board() -> Grid {
    Grid::new(GridSpec { cols: 12, rows: 8, wrap: Wrap::Torus })
}

#[must_use]
pub const fn chessboard() -> Grid {
    Grid::new(GridSpec::square(8))
}
```

Held by review. A builder that is right has `#[must_use]` on each method that
returns `Self`: `clippy::return_self_not_must_use` asks for it.

In a published crate, a new field on a spec that callers build as a literal
breaks each of them. A spec there that will grow is `#[non_exhaustive]`, as the
rule on exhaustive types below says. Outside its crate such a spec cannot be
built as a literal at all, not even with `..GridSpec::square(8)`, so callers
start from its constructor and assign the fields they change.

## A Newtype Has a Private Field, `new` and `get`

Two values of one primitive type that mean different things, a tile id and a
count, are different types, so one cannot be passed for the other. The field is
private, so the type can later check or change what it holds; `const fn new`
wraps and `const fn get` unwraps, both `#[must_use]`. A newtype that must cost
nothing is `#[repr(transparent)]`, with a compile-time assertion that it is the
size of what it wraps.

```rust
// Bad: a bare `u32` for an id and a count, so a caller can swap them.
#[must_use]
pub const fn place(tile: u32, count: u32) -> (u32, u32) {
    (tile, count)
}
```

```rust
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileId(u32);

const _: () = assert!(size_of::<TileId>() == size_of::<u32>(), "a tile id, and nothing else");

impl TileId {
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[must_use]
pub const fn place(tile: TileId, count: u32) -> (TileId, u32) {
    (tile, count)
}
```

Held by review.

## An Alias Where the Name Is Worth Having and a Second Type Is Not

An alias names a type for readers and changes nothing for the compiler, so it
fits where every use already means that exact type; a newtype is for where
confusing two uses would be a bug. The choice is argued in the item's doc: "an
alias, not a newtype: the name is worth having, a second type is not".

```rust
// Bad: an alias for two things a caller can swap, so nothing stops it.
pub type Col = u16;
pub type Row = u16;

#[must_use]
pub fn index(col: Col, row: Row, cols: u16) -> Option<usize> {
    usize::from(row).checked_mul(usize::from(cols))?.checked_add(usize::from(col))
}
```

```rust
/// A column, counted from the left edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Col(u16);

impl Col {
    #[must_use]
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// A row, counted from the top edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row(u16);

impl Row {
    #[must_use]
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// The squares of one row: an alias, since every use means this exact slice.
pub type Squares<'a> = &'a [u8];

#[must_use]
pub fn index(col: Col, row: Row, cols: u16) -> Option<usize> {
    usize::from(row.get()).checked_mul(usize::from(cols))?.checked_add(usize::from(col.get()))
}
```

Held by review.

## Check at the Boundary, and Let the Type Carry the Proof

Input is checked once, where it enters, by a constructor that returns a typed
refusal: `FromStr`, `TryFrom`, or a `parse` function. Past that point a function
takes the checked type and never checks again, because holding one is the proof.
A string or an integer that stands for something checked is not passed around as
itself.

```rust
// Bad: every function that takes an id must check it again.
#[must_use]
pub fn is_tile(id: &str) -> bool {
    id.strip_prefix('t').is_some_and(|digits| digits.parse::<u32>().is_ok())
}

#[must_use]
pub fn draw(id: &str) -> Option<char> {
    is_tile(id).then_some('#')
}
```

```rust
use core::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse tile error: want `t` and digits, like `t42`")]
pub struct ParseTileError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);

impl FromStr for TileId {
    type Err = ParseTileError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text.strip_prefix('t').ok_or(ParseTileError)?;
        digits.parse().map(Self).map_err(|_not_digits| ParseTileError)
    }
}

#[must_use]
pub const fn glyph(id: TileId) -> char {
    if id.0 == 0 { '.' } else { '#' }
}
```

Held by review.

## Implement `From` for What Cannot Fail, `TryFrom` for What Can, Never `Into`

`From` is a lossless widening or a shedding of a payload, and it cannot fail; a
conversion that can fail is `TryFrom`, with a typed error. `From` is what a type
implements: the standard library derives `Into` from it, and not the other way.

```rust,compile_fail
// fails: clippy::from_over_into
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);

// Bad: `Into` by hand, so `TileId: From` does not hold for anyone.
impl Into<u32> for TileId {
    fn into(self) -> u32 {
        self.0
    }
}
```

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);

impl From<TileId> for u32 {
    fn from(id: TileId) -> Self {
        id.0
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("tile id error: {held} is past the last id, {max}")]
pub struct TileIdError {
    pub held: u32,
    pub max: u32,
}

impl TryFrom<u32> for TileId {
    type Error = TileIdError;

    fn try_from(held: u32) -> Result<Self, Self::Error> {
        const MAX: u32 = 1 << 20;
        if held > MAX { Err(TileIdError { held, max: MAX }) } else { Ok(Self(held)) }
    }
}
```

Held by `clippy::from_over_into`.

## Printing and Parsing Agree on One Spelling

A type that is written as text and read back as text uses one spelling for both,
so what a program prints a user can type, and a test pins the round trip.

```rust
use core::fmt;
use core::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);

// Bad: printed `#42`, read back only as `t42`.
impl fmt::Display for TileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl FromStr for TileId {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.strip_prefix('t').and_then(|digits| digits.parse().ok()).map(Self).ok_or(())
    }
}
```

```rust
use core::fmt;
use core::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("parse tile error: want `t` and digits, like `t42`")]
pub struct ParseTileError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileId(u32);

impl fmt::Display for TileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl FromStr for TileId {
    type Err = ParseTileError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text.strip_prefix('t').ok_or(ParseTileError)?;
        digits.parse().map(Self).map_err(|_not_digits| ParseTileError)
    }
}

#[cfg(test)]
mod tests {
    use super::TileId;

    #[test]
    fn a_tile_id_reads_back_as_it_prints() {
        let id = TileId(42);
        assert_eq!(id.to_string().parse::<TileId>(), Ok(id), "one spelling, both ways");
    }
}
```

Held by review, and by the round-trip test.

## Derive What the Type Supports

Every public type is `Debug`, so any value can be printed in a failure. Each of
`Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` and `Default` is
derived where it holds for the type's meaning, not only for its fields: a caller
cannot add one later, and a missing `Eq` or `Hash` keeps the type out of a set.
A derive that would put a bound on a type parameter nobody needs is written by
hand, or with `derive_more` or `derive_where`, and a hand-written `Debug` that
leaves fields out ends in `finish_non_exhaustive()`.

```rust,compile_fail
// fails: missing_debug_implementations
// Bad: no `Debug`, so no failure can print one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}
```

Held by review.

Under `strict`, `missing_debug_implementations` refuses a public type with no
`Debug`, and `clippy::derive_partial_eq_without_eq` a `PartialEq` without the
`Eq` it could have.

## `Default` Where One Value Is Obvious, and `new` Delegates to It

A type whose natural empty value needs no arguments implements `Default`, and a
`new()` beside it returns `Self::default()`, so the two cannot drift. Where a
default would mislead, the type has none, and an `#[expect]` on its `new` says
why: a `new` that reads the clock, or one that panics until a startup
precondition holds, is no value a caller could assume. A spec has no `Default`:
its fields are written at every call site.

```rust,compile_fail
// fails: clippy::new_without_default
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: a `new()` with no `Default`, so `Grid::default()` does not exist.
    #[must_use]
    pub const fn new() -> Self {
        Self { squares: Vec::new() }
    }

    #[must_use]
    pub const fn squares(&self) -> &[u8] {
        self.squares.as_slice()
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn squares(&self) -> &[u8] {
        self.squares.as_slice()
    }
}
```

Held by `clippy::new_without_default`. A derived `Default` on an enum marks its
default variant `#[default]`; no attribute sets a field's default.

## Mark What Is Worth Using `#[must_use]`, and Give a Guard a Reason

A function whose only effect is its return value is `#[must_use]`, bare. A type
whose value holds something until it drops, a guard, is `#[must_use = "…"]`, the
reason saying what stays held. A function that returns a `Result` or a guard
itself needs nothing more, since the type already says it; one that returns
either inside an `Option` carries the guard's reason, since `Option` is not
`#[must_use]`.

```rust,compile_fail
// fails: clippy::double_must_use
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("occupied error: square {at} holds a tile already")]
pub struct OccupiedError {
    pub at: usize,
}

// Bad: `Result` is `#[must_use]` already.
#[must_use]
pub const fn claim(at: usize, taken: bool) -> Result<usize, OccupiedError> {
    if taken { Err(OccupiedError { at }) } else { Ok(at) }
}
```

```rust
#[derive(Debug)]
#[must_use = "the square stays claimed until the guard drops"]
pub struct ClaimGuard<'g> {
    square: &'g mut Option<u8>,
}

impl ClaimGuard<'_> {
    pub const fn put(self, tile: u8) {
        *self.square = Some(tile);
    }
}

#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<Option<u8>>,
}

impl Grid {
    #[must_use = "the square stays claimed until the guard drops"]
    pub fn claim(&mut self, at: usize) -> Option<ClaimGuard<'_>> {
        self.squares.get_mut(at).filter(|square| square.is_none()).map(|square| ClaimGuard { square })
    }

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

Held by `clippy::must_use_candidate`, which asks for the bare attribute, and
`clippy::double_must_use`, which refuses it on a `Result`. A guard in an
`Option` is held by review: `must_use_candidate` passes over a method on `&mut
self`.

## Public Signatures Name Their Generics

A public function names each type parameter, `fn fill_with<F: FnMut() -> u8>`,
so a caller can name it with a turbofish and a reader sees every bound in one
place. `impl Trait` in a return type is fine: it hides a type the caller never
names. A private helper may take `impl Trait` in an argument.

```rust,compile_fail
// fails: clippy::impl_trait_in_params
#[derive(Debug)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: a caller cannot name the closure's type.
    pub fn fill_with(&mut self, make: impl FnMut() -> u8) {
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
    pub fn fill_with<F: FnMut() -> u8>(&mut self, make: F) {
        self.squares.fill_with(make);
    }

    pub fn tiles(&self) -> impl Iterator<Item = u8> {
        self.squares.iter().copied().filter(|&tile| tile != 0)
    }
}
```

Held by review.

Under `strict`, `clippy::impl_trait_in_params` refuses `impl Trait` in a public
function's arguments.

## Dispatch Statically, and Use `dyn` Only at the Edges

A generic is resolved at compile time, inlined and checked; a trait object costs
an indirection and forgets the type. Library code is generic over a trait, even
a composition of layers, and a trait object appears only where a type must be
forgotten: `&mut dyn fmt::Write` for any writer, a boxed error at a binary's
edge.

```rust
use core::fmt;

pub trait Painter {
    fn paint(&mut self, tile: u8) -> fmt::Result;
}

// Bad: a box and a virtual call for a painter the caller already has.
pub fn draw(painter: &mut Box<dyn Painter>, squares: &[u8]) -> fmt::Result {
    squares.iter().try_for_each(|&tile| painter.paint(tile))
}
```

```rust
use core::fmt;

pub trait Painter {
    fn paint(&mut self, tile: u8) -> fmt::Result;
}

pub fn draw<P: Painter>(painter: &mut P, squares: &[u8]) -> fmt::Result {
    squares.iter().try_for_each(|&tile| painter.paint(tile))
}
```

Held by review.

## State Lives in the Type

Where a call is valid only in some states, the state is a type parameter, a
marker type such as `Editing` or `Sealed`, and each state's methods sit on its
own `impl`, so a wrong call does not compile. A value that must be used once is
taken by value, so after it is spent nothing can use it.

```rust,compile_fail
// fails: E0599
use core::marker::PhantomData;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Editing;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sealed;

#[derive(Debug, Default)]
pub struct Board<S> {
    squares: Vec<u8>,
    state: PhantomData<S>,
}

impl Board<Editing> {
    pub fn place(&mut self, tile: u8) {
        self.squares.push(tile);
    }

    #[must_use]
    pub fn seal(self) -> Board<Sealed> {
        Board { squares: self.squares, state: PhantomData }
    }
}

impl Board<Sealed> {
    #[must_use]
    pub const fn squares(&self) -> &[u8] {
        self.squares.as_slice()
    }
}

pub fn edit() {
    let mut board = Board::<Editing>::default();
    board.place(3);
    let mut sealed = board.seal();
    // Bad: a sealed board has no `place`, so this is refused.
    sealed.place(4);
}
```

```rust
use core::marker::PhantomData;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Editing;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Sealed;

#[derive(Debug, Default)]
pub struct Board<S> {
    squares: Vec<u8>,
    state: PhantomData<S>,
}

impl Board<Editing> {
    pub fn place(&mut self, tile: u8) {
        self.squares.push(tile);
    }

    #[must_use]
    pub fn seal(self) -> Board<Sealed> {
        Board { squares: self.squares, state: PhantomData }
    }
}

impl Board<Sealed> {
    #[must_use]
    pub const fn squares(&self) -> &[u8] {
        self.squares.as_slice()
    }
}

#[must_use]
pub fn edit() -> Board<Sealed> {
    let mut board = Board::<Editing>::default();
    board.place(3);
    board.seal()
}
```

Held by the compiler, as the first block shows: `E0599`, no such method. A crate
proves such a refusal with a compile-fail test.

## Seal a Trait Callers Use but Must Not Implement

A public trait whose implementations the crate relies on being its own has a
supertrait in a private module. Callers can name the trait, bound on it and call
its methods, but not implement it, so the crate can add a method without a
breaking change. A trait sealed this way is sealed whole: no caller implements
any part of it.

```rust
// Bad: callers may implement `Shape`, so adding a method breaks them.
pub trait Shape {
    fn sides(&self) -> u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square;

impl Shape for Square {
    fn sides(&self) -> u8 {
        4
    }
}
```

```rust
mod sealed {
    /// Kept private: which shapes there are is this crate's business.
    pub trait Sealed {}
}

pub trait Shape: sealed::Sealed {
    fn sides(&self) -> u8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square;

impl sealed::Sealed for Square {}

impl Shape for Square {
    fn sides(&self) -> u8 {
        4
    }
}
```

Held by review.

## A Match Names Every Variant

A wildcard arm on an enum the crate owns swallows the next variant anyone adds,
so what should be a compile error at every match becomes a silent default. Each
variant is named, several joined with `|` where they share an arm. A match on
another crate's `#[non_exhaustive]` enum still needs its wildcard, and names
every variant it knows before it.

```rust,compile_fail
// fails: clippy::wildcard_enum_match_arm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

#[must_use]
pub const fn arrow(side: Side) -> char {
    match side {
        Side::North => '^',
        Side::South => 'v',
        // Bad: a fifth side would be drawn as a sideways arrow.
        _ => '>',
    }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

#[must_use]
pub const fn arrow(side: Side) -> char {
    match side {
        Side::North => '^',
        Side::South => 'v',
        Side::East => '>',
        Side::West => '<',
    }
}
```

Held by `clippy::wildcard_enum_match_arm`.

## A Type Is Exhaustive, Unless a Published Crate Means It to Grow

Enums and structs are exhaustive by default. `#[non_exhaustive]` costs every
caller outside the crate: a `match` on the enum can no longer be exhaustive and
ends in a wildcard arm, where the variant added next lands without a word, and
the struct can no longer be built as a literal or taken apart without `..`.
Within a workspace built together, a new variant is a compile error at every
match that must decide what to do with it, which is the alarm wanted. So the
attribute goes only on a public type of a published crate whose variants or
fields are expected to grow within a major version, the error enum of a library
that will gain causes being the usual one, and never on a type complete by
nature: a direction, a closed set of states. The author judges each type; the
attribute is never applied by default.

```rust
// Bad: marked to grow, though a grid has no fifth side, so every caller's match
// needs a wildcard for a variant that will never come.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}
```

```rust
use thiserror::Error;

/// Complete by nature: exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

/// Why a board file was refused. `#[non_exhaustive]`: the crate is published,
/// and each new board format brings new ways for a file to be wrong.
#[non_exhaustive]
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LoadError {
    #[error("load error: the file holds no rows")]
    Empty,
    #[error("load error: row {row} has {have} squares, the grid needs {need}")]
    Short { row: u16, need: u16, have: u16 },
}
```

Held by review.

`check-rust-lints` runs `non_exhaustive_omitted_patterns`, which refuses a
wildcard on another crate's `#[non_exhaustive]` enum that swallows a variant the
match could name, so a caller's wildcard is watched.

## Bound a Generic Where It Is Used

A bound on a type's definition must be repeated on every `impl`, every function
that names the type and every caller, whether or not it needs the bound. A type
puts a bound on its definition only where a field's type needs it; everywhere
else it sits on the `impl` or the function that uses it.

```rust
// Bad: every use of `Grid<T>` must now prove `T: Clone`, even to read its size.
#[derive(Debug)]
pub struct Grid<T: Clone> {
    squares: Vec<T>,
}

impl<T: Clone> Grid<T> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.squares.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.squares.is_empty()
    }

    pub fn fill(&mut self, tile: &T) {
        self.squares.fill(tile.clone());
    }
}
```

```rust
#[derive(Debug)]
pub struct Grid<T> {
    squares: Vec<T>,
}

impl<T> Grid<T> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.squares.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.squares.is_empty()
    }
}

impl<T: Clone> Grid<T> {
    pub fn fill(&mut self, tile: &T) {
        self.squares.fill(tile.clone());
    }
}
```

Held by review.
