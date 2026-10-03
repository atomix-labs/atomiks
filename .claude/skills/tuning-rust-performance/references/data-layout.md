# Data Layout

Read this before a type stored many times or read in a hot loop, an enum with a
variant much larger than the rest, a `repr`, an `Option` around a type, the
width of an integer field, a size assertion, a collection of boxes or a linked
list, and a hash map: its hasher, its keys and its lookups. A loop runs at the
speed its data reaches it: a type half the size puts twice as many in each cache
line, and data laid out in the order a loop reads it arrives before it is asked
for.

## A Hot Type's Size Is Asserted Beside It

A field added to a type stored a million times costs a million times its size,
and nothing says so until someone measures. A `const` assertion beside the type
fails the crate's own build when its size moves, so a change that grows it is
made on purpose, with its reason in the message.

What else a type must keep, `Send` or `Copy`, is asserted as
`writing-rust-tests` says.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Bad: a size a reader must count, and nothing that holds it.
pub struct Tile {
    pub glyph: u16,
    pub layer: u8,
    pub flags: u8,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    pub glyph: u16,
    pub layer: u8,
    pub flags: u8,
}

// A tile is stored for every square of every board, so a byte more is a board's worth.
const _: () = assert!(size_of::<Tile>() == 4, "a glyph, a layer and a byte of flags");
```

Held by the assertion.

## The Compiler Orders a Struct's Fields, Unless `repr(C)` Fixes Them

A struct with Rust's default layout guarantees no field order, and the compiler
reorders its fields to spend the least on padding: `u8`, `u64`, `u8` declared in
that order take 16 bytes, not the 24 their order would. So reordering such a
struct's fields to save space saves nothing. `#[repr(C)]` keeps the declared
order, for a type read through a pointer or by another program, and then the
order is the author's: widest alignment first, so padding falls at the end.

```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
// Bad: in declared order, 7 bytes of padding after each `u8`: 24 bytes.
pub struct Stamp {
    pub layer: u8,
    pub at: u64,
    pub flags: u8,
}

const _: () = assert!(size_of::<Stamp>() == 24, "declared order, padded");
```

```rust
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Stamp {
    pub at: u64,
    pub layer: u8,
    pub flags: u8,
}

const _: () = assert!(size_of::<Stamp>() == 16, "widest first, padded once at the end");
```

Held by review, and by the size assertion.

## An Integer Is as Wide as Its Domain

A field's width multiplies by every copy of it: a column on a board of at most
65,535 columns is a `u16`, two bytes, where a `usize` is eight, so a position
takes four bytes and not sixteen. The value widens where it is used, with
`From`, which cannot fail, and narrows only at the boundary it enters by, with
`TryFrom`, which refuses what does not fit.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Bad: sixteen bytes for a position no board needs more than four for.
pub struct Pos {
    pub col: usize,
    pub row: usize,
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl Pos {
    #[must_use]
    pub fn index(self, cols: u16) -> usize {
        usize::from(self.row).saturating_mul(usize::from(cols)).saturating_add(usize::from(self.col))
    }
}

const _: () = assert!(size_of::<Pos>() == 4, "two `u16`s and nothing else");
```

Held by review, and by the size assertion.

## An `Option` Costs Nothing Where Its Type Has a Spare Value

An `Option` needs somewhere to say `None`. A type with a value it never holds, a
niche, lends it: a reference, a `Box`, a `Vec`, a `String` and a `NonZeroU32`
never hold zero, and a `char` or a `bool` never holds a value past its range, so
an `Option` of one is the same size as it, and `Option<Vec<T>>` is as large as
`Vec<T>`, 24 bytes. A plain integer has no such value, so `Option<u32>` is 8
bytes and `Option<u64>` 16. An id that is never zero is a `NonZeroU32`, so an
optional id costs nothing.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square {
    pub glyph: u16,
    // Bad: 4 bytes of payload, 8 with the `Option`.
    pub owner: Option<u32>,
}

const _: () = assert!(size_of::<Square>() == 12, "a glyph, padding, and a tagged owner");
```

```rust
use core::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square {
    pub glyph: u16,
    pub owner: Option<NonZeroU32>,
}

const _: () = assert!(size_of::<Square>() == 8, "a glyph, padding, and an owner whose zero is `None`");
const _: () = assert!(size_of::<Option<Vec<u8>>>() == size_of::<Vec<u8>>(), "a `Vec` lends its niche");
```

Held by review, and by the size assertion.

## A Large, Rare Variant Is Boxed

An enum is as large as its largest variant, and its tag, so every value of it
pays for the largest: a `Tile` that may hold a 256-byte sprite takes 260 bytes
even as `Empty`. A variant much larger than the rest, and rarely made, holds a
`Box` of its payload instead, eight bytes, and the enum shrinks to its next
largest.

```rust,compile_fail
// fails: clippy::large_enum_variant
#[derive(Debug)]
pub enum Tile {
    Empty,
    Glyph(char),
    // Bad: every tile pays for a sprite's 256 bytes.
    Sprite([u8; 256]),
}
```

```rust
#[derive(Debug)]
pub enum Tile {
    Empty,
    Glyph(char),
    Sprite(Box<[u8; 256]>),
}

const _: () = assert!(size_of::<Tile>() == 16, "a tag and a box");
```

Held by `clippy::large_enum_variant`, where the largest variant passes the next
by more than 200 bytes.

Under `strict`, `variant_size_differences` also refuses a largest variant more
than three times the next.

## Data That Never Grows Is `Box<[T]>`

A `Vec` is a pointer, a length and a capacity, 24 bytes; a `Box<[T]>` is a
pointer and a length, 16, and a `Box<str>` against a `String` the same, so a
collection made once and never grown, stored many times, drops a word each and
says it will not grow. `into_boxed_slice` gives back the spare capacity first,
by a reallocation where there is any, so a vector built to its exact length
converts for free.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    // Bad: a capacity kept for a vector that is never pushed to.
    pub pixels: Vec<u8>,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    pub pixels: Box<[u8]>,
}

impl Sprite {
    #[must_use]
    pub fn blank(width: u16, height: u16) -> Self {
        let pixels = usize::from(width).saturating_mul(usize::from(height));
        Self { pixels: vec![0; pixels].into_boxed_slice() }
    }
}
```

Held by review.

## Store Values Side by Side, Not Behind Pointers

A `Vec<T>` holds its values in one run of memory, which a loop reads in order
and the hardware fetches ahead; a `Vec<Box<T>>` or a linked list holds pointers,
and each value is a load from wherever it was allocated. A value of fixed size
goes in the collection itself, a queue is a `VecDeque`, and a graph links its
nodes by index into a `Vec`, not by pointer.

```rust,compile_fail
// fails: clippy::linkedlist
extern crate alloc;

use alloc::collections::LinkedList;

#[derive(Debug, Default)]
pub struct Queue {
    // Bad: a pointer chased for every tile.
    pending: LinkedList<u32>,
}

impl Queue {
    pub fn push(&mut self, tile: u32) {
        self.pending.push_back(tile);
    }
}
```

```rust
extern crate alloc;

use alloc::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Queue {
    pending: VecDeque<u32>,
}

impl Queue {
    pub fn push(&mut self, tile: u32) {
        self.pending.push_back(tile);
    }
}
```

Held by `clippy::linkedlist`, and by `clippy::vec_box` for a `Vec<Box<T>>` whose
`T` is sized and under 4,096 bytes; neither reads a type in the crate's exported
signatures.

## Keys from Outside the Program Keep the Standard Hasher

std's `HashMap` hashes with an algorithm seeded at random, chosen to resist
HashDoS: keys an attacker picks to collide, which turn each lookup into a walk.
It is currently SipHash 1-3, and std's documentation says others outperform it
for small keys such as integers, without that protection. So a map keyed by what
comes from outside, a name a person typed or a key off the network, keeps the
default hasher. A faster one, one the repository already depends on, as FxHash
or ahash, is for keys the program makes itself, and only where a profile shows
hashing hot and a run shows the swap pays.

```text
// Bad: a predictable hasher over names a person typed, which they can make collide.
let glyphs: FxHashMap<String, char> = FxHashMap::default();
```

```rust
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Glyphs {
    // Names a person types, so the default hasher, seeded at random.
    by_name: HashMap<String, char>,
}

impl Glyphs {
    pub fn name(&mut self, name: String, glyph: char) {
        self.by_name.insert(name, glyph);
    }

    #[must_use]
    pub fn glyph(&self, name: &str) -> Option<char> {
        self.by_name.get(name).copied()
    }
}
```

Held by review.

A public function that takes a `HashMap` is generic over its hasher, `fn
glyph_of<S: BuildHasher>(glyphs: &HashMap<String, char, S>, …)`, so a caller's
choice of hasher passes through; `clippy::implicit_hasher` asks for it.

## A Map Keyed by a Small Dense Id Is a `Vec`

Ids the program hands out in order, from zero, index a `Vec`: a lookup is an
offset, with no hash and no probe, and the values sit side by side. A `HashMap`
keyed by such an id hashes every lookup to find what an index would.

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    pub width: u16,
}

#[derive(Debug, Default)]
pub struct Atlas {
    // Bad: ids 0, 1, 2 and on, each hashed to find its sprite.
    sprites: HashMap<u16, Sprite>,
}

impl Atlas {
    #[must_use]
    pub fn sprite(&self, id: u16) -> Option<&Sprite> {
        self.sprites.get(&id)
    }
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    pub width: u16,
}

#[derive(Debug, Default)]
pub struct Atlas {
    sprites: Vec<Sprite>,
}

impl Atlas {
    #[must_use]
    pub fn sprite(&self, id: u16) -> Option<&Sprite> {
        self.sprites.get(usize::from(id))
    }
}
```

Held by review.

## `entry` Finds a Key Once

`contains_key` and then `insert` hash the key and probe the table twice; `entry`
finds the slot once, and fills it or hands back what is there.

```rust,compile_fail
// fails: clippy::map_entry
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FirstSeen {
    at: HashMap<char, u16>,
}

impl FirstSeen {
    pub fn note(&mut self, glyph: char, at: u16) {
        // Bad: two lookups where one serves.
        if !self.at.contains_key(&glyph) {
            self.at.insert(glyph, at);
        }
    }
}
```

```rust
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FirstSeen {
    at: HashMap<char, u16>,
}

impl FirstSeen {
    pub fn note(&mut self, glyph: char, at: u16) {
        self.at.entry(glyph).or_insert(at);
    }
}
```

Held by `clippy::map_entry`.

## What a Hot Loop Reads Together Is Stored Together

A loop that reads one field of each value still brings the whole value into the
cache, so the fields it skips cost it their bytes. Where a hot loop reads a few
fields of many values, those fields live apart from the rest: the hot part in a
`Vec` of its own, the cold part, a name or a history, beside it, joined by
index; and where it reads one field of all of them, that field is a `Vec` of its
own. Whether it pays is measured: a loop that reads every field gains nothing.

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub name: String,
    pub pos: (f32, f32),
    pub history: Vec<(f32, f32)>,
}

pub fn drift(sprites: &mut [Sprite], by: (f32, f32)) {
    // Bad: a name and a history pulled through the cache for each position moved.
    for sprite in sprites {
        sprite.pos = (sprite.pos.0 + by.0, sprite.pos.1 + by.1);
    }
}
```

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sprites {
    pub pos: Vec<(f32, f32)>,
    pub names: Vec<String>,
    pub histories: Vec<Vec<(f32, f32)>>,
}

impl Sprites {
    pub fn drift(&mut self, by: (f32, f32)) {
        for pos in &mut self.pos {
            *pos = (pos.0 + by.0, pos.1 + by.1);
        }
    }
}
```

Held by review, and by a benchmark.
