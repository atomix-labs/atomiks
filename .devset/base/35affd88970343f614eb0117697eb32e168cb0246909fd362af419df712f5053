# Simplicity

Read this before adding a trait, an interface, a generic, an option, a
parameter, a wrapper, a helper, a module or a layer; before adding what the task
did not ask for; and before deleting code or leaving any behind. It says what
earns its place in a codebase: what its callers need now, written once, and
nothing that no longer runs. The rules hold in any language, and the examples
are Rust; where the repository applies python or shell, a section at the end
says what that language adds.

Simple is not short. A type that makes a wrong state impossible to build, a
function that names a step, an error type for each refusal: each costs lines and
is worth them, since each is one less thing a reader holds. What these rules
refuse is code that asks a reader to learn something that does nothing for them
yet.

## Build for the Callers There Are

A trait with one implementation, a generic with one type, a factory, registry or
strategy with one entry: each is a concept a reader learns and a path a change
keeps working, paid for now for a second case that may never come, and that,
when it comes, tends to want another shape than the one guessed. The concrete
type is shorter, is read without a jump to find what runs, and becomes a trait
in one change when the second implementation arrives, when its shape is known.

Three traits are no guess. A test double is a second implementation, and a trait
callers outside the crate are meant to implement is the point of the crate. An
extension trait adds methods to another crate's type, which has no other way to
gain them. A sealed trait that a published crate keeps as its extension point
lets the crate add implementations and methods later without a breaking change.

A request that the code be flexible, extensible, or ready for other kinds later
is met the same way. Code that is concrete, small and says each rule once is
what a later change bends most easily; a trait guessed before its second
implementation is what it must first undo. The change's description says where
the seam will go when the second kind comes, and the code adds nothing for it
now.

```rust
pub trait TileStore {
    fn tile_at(&self, index: usize) -> Option<u8>;
}

#[derive(Debug, Default)]
pub struct VecStore {
    squares: Vec<u8>,
}

impl TileStore for VecStore {
    fn tile_at(&self, index: usize) -> Option<u8> {
        self.squares.get(index).copied()
    }
}

// Bad: a trait, an implementation and a type parameter, for the one store there
// is.
#[derive(Debug, Default)]
pub struct Grid<S> {
    store: S,
}

impl<S: TileStore> Grid<S> {
    #[must_use]
    pub fn tile_at(&self, index: usize) -> Option<u8> {
        self.store.tile_at(index)
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
    pub fn tile_at(&self, index: usize) -> Option<u8> {
        self.squares.get(index).copied()
    }
}
```

This rule decides whether a trait should exist at all: with a second
implementation, a test double or one its callers write, it should. How a trait
that exists is used is `writing-rust`'s `references/api-design.md`, "Dispatch
Statically, and Use `dyn` Only at the Edges": library code is generic over it,
as `Grid<S: TileStore>` would be once a second store is there.

Held by review.

## No Option Nobody Sets

A setting, a parameter or a field that every caller leaves at one value is a
branch that never runs another way, and a question each reader must answer: what
happens if it is set, and does anything set it? It is left out until a caller
needs it, and that caller's change adds it, knowing what it is for.

```rust
use core::iter;

pub const EMPTY: u8 = 0;

// Bad: every caller passes a `scale` of 1 and no `border`, so neither changes
// what the function does, and each reader must work out that it never does.
#[must_use]
pub fn render(squares: &[u8], scale: usize, border: Option<char>) -> String {
    let mut line = String::new();
    if let Some(edge) = border {
        line.push(edge);
    }
    for &tile in squares {
        let glyph = if tile == EMPTY { '.' } else { '#' };
        line.extend(iter::repeat_n(glyph, scale));
    }
    if let Some(edge) = border {
        line.push(edge);
    }
    line
}
```

```rust
pub const EMPTY: u8 = 0;

#[must_use]
pub fn render(squares: &[u8]) -> String {
    squares.iter().map(|&tile| if tile == EMPTY { '.' } else { '#' }).collect()
}
```

Held by review. rustc's `dead_code` refuses a private field nothing reads,
wherever its type sits, and passes a `pub` field of an exported type; a
parameter or a field that is read, but always given one value, passes every
lint.

## No Wrapper That Only Forwards

A function, a type or a module that hands each call on unchanged, a
`GridService` whose every method calls the `Grid`'s of the same name, adds a
name to learn and a hop to follow, and nothing a reader needs. The caller calls
what it wraps. A wrapper that adds something, a check, a narrower surface, a
type kept out of an API, is no forwarder, and says in its name what it adds.

Nor is a forwarder the language's conventions ask for: a `new` that returns
`Self::default()`, an `IntoIterator` for `&Grid` that calls `iter`, a `From`,
`AsRef`, `Deref` or `Display` that hands on to a newtype's inner value, a
re-export or facade that gives a private module's item its public path. Each is
a name callers expect to find, and a caller who reaches for it finds it. The
line is whether a caller needs the name: what is refused is a type or a function
that adds nothing a caller needs, and that no convention asks for.

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    #[must_use]
    pub fn tile_at(&self, index: usize) -> Option<u8> {
        self.squares.get(index).copied()
    }
}

// Bad: a service that hands each call to the grid, unchanged.
#[derive(Debug, Default)]
pub struct GridService {
    grid: Grid,
}

impl GridService {
    #[must_use]
    pub fn tile_at(&self, index: usize) -> Option<u8> {
        self.grid.tile_at(index)
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
    pub fn tile_at(&self, index: usize) -> Option<u8> {
        self.squares.get(index).copied()
    }
}
```

Held by review.

## Add What the Task Needs, and Nothing Beside It

A change asked for one operation adds that one, with its tests, and not the
neighbours a complete API might have: `remove`, `clear`, a batch form, a
`Display` nobody prints. Each is a promise to keep and a surface to test, and
one written before its caller guesses what that caller will want. What the task
needs includes what its own code calls; what it does not is left for the change
that needs it.

```rust
use core::mem;

use thiserror::Error;

pub const EMPTY: u8 = 0;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("off grid error: square {index} is past the last")]
pub struct OffGridError {
    index: usize,
}

#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

// Bad: asked for `place`, and given three more that nothing calls.
impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) -> Result<(), OffGridError> {
        let square = self.squares.get_mut(index).ok_or(OffGridError { index })?;
        *square = tile;
        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Option<u8> {
        self.squares.get_mut(index).map(mem::take)
    }

    pub fn clear(&mut self) {
        self.squares.fill(EMPTY);
    }

    pub fn place_all(&mut self, tiles: &[(usize, u8)]) -> Result<(), OffGridError> {
        tiles.iter().try_for_each(|&(index, tile)| self.place(index, tile))
    }
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("off grid error: square {index} is past the last")]
pub struct OffGridError {
    index: usize,
}

#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) -> Result<(), OffGridError> {
        let square = self.squares.get_mut(index).ok_or(OffGridError { index })?;
        *square = tile;
        Ok(())
    }
}
```

Held by review. `dead_code` refuses a function nothing reaches only where the
crate does not export it.

## Merge What Changes Together, Not What Looks Alike

Two blocks that look alike are merged only when they change for the same reason,
so that one change to the rule is one edit. Two that look alike by chance, a
column's bound and a score's cap, merged into one helper, tie together what the
domain keeps apart: the first time one of them changes, the helper grows a flag
for which caller it serves, and the flag grows a branch. A copy that stays two
is cheaper than that, and the time to merge is when the rule, not the text, is
shared.

```rust
// Bad: a column and a score shared a helper because each had a limit; the
// score's rule changed, and the helper grew a flag.
#[must_use]
pub const fn limit(value: u16, max: u16, refuse: bool) -> Option<u16> {
    if value <= max {
        Some(value)
    } else if refuse {
        None
    } else {
        Some(max)
    }
}
```

```rust
pub const MAX_SCORE: u16 = 999;

#[must_use]
pub const fn column(value: u16, columns: u16) -> Option<u16> {
    if value < columns { Some(value) } else { None }
}

#[must_use]
pub fn score(points: u16) -> u16 {
    points.min(MAX_SCORE)
}
```

Held by review.

## Use What the Language Already Has

What the standard library or the codebase already has, a search, a sum, a
saturating step, a parser, is used, not written again: a reader knows `position`
and `saturating_sub` on sight and reads a loop that does the same line by line,
looking for the way it differs. A codebase's own helper for a thing is found and
used before a second one is written beside it.

```rust
pub const EMPTY: u8 = 0;

// Bad: a search written out, which the reader must check does what `position`
// does.
#[must_use]
pub fn first_empty(squares: &[u8]) -> Option<usize> {
    for (index, tile) in squares.iter().enumerate() {
        if *tile == EMPTY {
            return Some(index);
        }
    }
    None
}
```

```rust
pub const EMPTY: u8 = 0;

#[must_use]
pub fn first_empty(squares: &[u8]) -> Option<usize> {
    squares.iter().position(|&tile| tile == EMPTY)
}
```

Held by review. Clippy refuses some hand-written forms, a `needless_range_loop`
over indices, a `manual_find`, but not the loop above.

## What Would Cost Far More to Add Later Is Judged on Its Own

These rules weigh a guess against a known cost. A few things are known: a file
or wire format that will change and cannot say which version it is, data that is
lost if it is not kept when it arrives, a published type that cannot grow
without breaking its callers. Adding them later costs every file already written
or every caller already built, so they are judged by that cost, each on its own,
and not refused as generality.

```rust
// Bad: a board file with no version, so the first change to the layout cannot
// tell old files from new.
#[must_use]
pub fn encode(squares: &[u8]) -> Vec<u8> {
    squares.to_vec()
}
```

```rust
/// The board file's layout, written first so a reader of the file knows it.
pub const FORMAT: u8 = 1;

#[must_use]
pub fn encode(squares: &[u8]) -> Vec<u8> {
    let mut file = Vec::with_capacity(squares.len().saturating_add(1));
    file.push(FORMAT);
    file.extend_from_slice(squares);
    file
}
```

A published Rust type's room to grow, `#[non_exhaustive]`, is judged type by
type, as `writing-rust`'s `references/api-design.md`, "A Type Is Exhaustive,
Unless a Published Crate Means It to Grow", says; it is never applied by
default.

Held by review.

## Dead Code Is Deleted

Code nobody runs is deleted: not commented out, not kept behind a flag that is
always off, not renamed `_old` or left beside the version that replaced it.
Version control keeps every line ever written; a copy in the tree goes stale
while it looks alive, turns up in every search, and leaves the next reader
asking whether it still matters. A change deletes what it makes obsolete, in the
same change: the function it replaced, the import it no longer needs, the branch
no caller takes.

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) {
        if let Some(square) = self.squares.get_mut(index) {
            *square = tile;
        }
    }

    // Bad: the old version, kept in case.
    // pub fn place_old(&mut self, index: usize, tile: u8) {
    //     self.squares.insert(index, tile);
    // }
}
```

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) {
        if let Some(square) = self.squares.get_mut(index) {
            *square = tile;
        }
    }
}
```

Held in part by `just check-rust-clippy`, whose warnings are errors: rustc's
`dead_code` refuses an item nothing in the crate reaches. No lint sees
commented-out code, or an exported item nothing calls; review holds those.

## A Parameter Nothing Reads Is Removed

A parameter the body no longer reads is removed from the signature and from
every call, not renamed with a leading `_` to quiet the lint that found it. The
`_` keeps a lie in the signature: every caller still computes and passes a value
that goes nowhere, and a reader of the call assumes it matters. A parameter a
trait or a callback's signature fixes stays, named with `_` and a word,
`_index`, so the drop reads as chosen.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    Refuse,
    Replace,
}

#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    // Bad: `clash` stopped mattering, and was renamed rather than removed.
    pub fn place(&mut self, index: usize, tile: u8, _clash: Clash) {
        if let Some(square) = self.squares.get_mut(index) {
            *square = tile;
        }
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Grid {
    squares: Vec<u8>,
}

impl Grid {
    pub fn place(&mut self, index: usize, tile: u8) {
        if let Some(square) = self.squares.get_mut(index) {
            *square = tile;
        }
    }
}
```

Held by review: rustc's `unused_variables` finds the parameter, and is quieted
by the `_` that this rule refuses.

## In the Shell

- **No function that only calls another** with the same arguments.
- **No option nobody passes**: a script parses the flags its callers use.
- **A command that no longer runs is deleted**, not commented out above the one
  that replaced it.

```bash
# Bad: a function that forwards, and the old command left above the new.
grid=$1

run_check() {
    check_grid "$@"
}

# sed -i 's/ 0$/ -/' "$grid"
awk '$2 == 0 { $2 = "-" } 1' "$grid" > "$grid.new"
```

```bash
grid=$1
awk '$2 == 0 { $2 = "-" } 1' "$grid" > "$grid.new"
```

Held by review, and in part by `just check-shell`: ShellCheck's `SC2034` refuses
a variable nothing reads.
