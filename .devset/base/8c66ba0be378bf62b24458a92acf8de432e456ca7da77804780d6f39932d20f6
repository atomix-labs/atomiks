# Allocation

Read this before a hot path, a loop that builds a collection or a string, a
buffer, a `format!`, a `clone` or a `collect` that runs often, a large value,
and a crate or path that must not allocate. An allocation is a call into the
allocator, and a free another; on a path that runs millions of times they are
the cost, and on a latency path each one is a pause nobody chose. The rule is to
allocate before the hot path starts and reuse what was allocated after.

## Allocate Before the Hot Path, and Reuse After

A collection made in a loop is allocated and freed on every turn. One made
before the loop and cleared on each turn keeps its capacity, since `clear` drops
the elements and not the memory, so once it has grown to what a turn needs, the
loop allocates nothing: a hundred turns of `clear` and `extend` into a buffer
with room make no allocation at all. A buffer used across calls lives in the
value that makes them, as the last rule's `Painter` holds its own.

```rust
#[must_use]
pub fn lit_weight(rows: &[&[u8]]) -> usize {
    let mut total = 0_usize;
    for row in rows {
        // Bad: a vector allocated and freed on every row.
        let tiles: Vec<u8> = row.iter().copied().filter(|tile| *tile > 0).collect();
        total = total.saturating_add(weight(&tiles));
    }
    total
}

fn weight(tiles: &[u8]) -> usize {
    tiles.iter().map(|tile| usize::from(*tile)).sum()
}
```

```rust
#[must_use]
pub fn lit_weight(rows: &[&[u8]]) -> usize {
    let mut total = 0_usize;
    let mut tiles = Vec::new();
    for row in rows {
        tiles.clear();
        tiles.extend(row.iter().copied().filter(|tile| *tile > 0));
        total = total.saturating_add(weight(&tiles));
    }
    total
}

fn weight(tiles: &[u8]) -> usize {
    tiles.iter().map(|tile| usize::from(*tile)).sum()
}
```

Held by review, and by a test under a counting allocator where a path must not
allocate, as the last rule shows.

## Size a Collection When Its Count Is Known

A `Vec` that grows by pushing doubles its capacity each time it fills, 4, 8, 16
and on, and each growth allocates anew and copies what it held: a thousand
`u32`s pushed one at a time cost nine allocations, where `with_capacity(1_000)`
costs one. `collect` sizes the result once from an iterator that knows its
length, a `map` over a range or a slice; after a `filter` it cannot, and grows.
A `HashMap` grows and rehashes the same way, so `with_capacity` serves it too,
and `reserve` serves a collection that already exists.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: u16,
    pub row: u16,
}

#[must_use]
pub fn row_of(row: u16, columns: u16) -> Vec<Position> {
    // Bad: grows by doubling, allocating and copying as it goes.
    let mut squares = Vec::new();
    for column in 0..columns {
        squares.push(Position { column, row });
    }
    squares
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: u16,
    pub row: u16,
}

#[must_use]
pub fn row_of(row: u16, columns: u16) -> Vec<Position> {
    (0..columns).map(|column| Position { column, row }).collect()
}
```

Held by review.

## An Empty Collection Allocates Nothing

`Vec::new()`, `vec![]`, `String::new()`, `HashMap::new()` and
`Vec::with_capacity(0)` allocate nothing: an empty `Vec` is a length, a capacity
and a dangling pointer, and the first push allocates. So an empty collection
needs no `Option` around it to save an allocation, and a type whose fields start
empty derives `Default`; an `Option<Vec<T>>` is no smaller than the `Vec<T>`
either, as `data-layout.md` shows.

```rust
#[derive(Debug, Default)]
pub struct Board {
    // Bad: an `Option` to spare an allocation that an empty `Vec` never makes.
    marks: Option<Vec<u32>>,
}

impl Board {
    pub fn mark(&mut self, square: u32) {
        self.marks.get_or_insert_with(Vec::new).push(square);
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Board {
    marks: Vec<u32>,
}

impl Board {
    pub fn mark(&mut self, square: u32) {
        self.marks.push(square);
    }
}
```

Held by review.

## Format into a Buffer, Not a New `String`

Each `format!` allocates the `String` it returns, so `push_str(&format!(…))`
allocates a string only to copy it and free it. `write!` formats into a `String`
through `fmt::Write`, or into bytes through `io::Write`, and a `String` cleared
and written again keeps its capacity: a hundred `format!`s make a hundred
allocations, and a hundred `write!`s into a cleared `String` with room make
none.

```rust,compile_fail
// fails: clippy::format_push_string
#[must_use]
pub fn render(row: &[u16]) -> String {
    let mut line = String::new();
    for column in row {
        // Bad: a string allocated for each column, copied, and freed.
        line.push_str(&format!("{column},"));
    }
    line
}
```

```rust
use core::fmt::{self, Write};

pub fn render(row: &[u16], line: &mut String) -> fmt::Result {
    line.clear();
    for column in row {
        write!(line, "{column},")?;
    }
    Ok(())
}
```

Held by `clippy::format_push_string`.

## Clone into a Held Value with `clone_from`

`palette = saved.clone()` allocates a new copy and frees what `palette` had;
`palette.clone_from(&saved)` copies into `palette`'s own allocation where it has
room, so a hundred of them into a `String` with room allocate nothing. A derived
`Clone` implements `clone` alone, and its `clone_from` is `*self =
source.clone()`, which reuses nothing: a type cloned into a held value often
clones field by field, or implements `clone_from` by hand.

```rust,compile_fail
// fails: clippy::assigning_clones
#[derive(Debug, Clone, Default)]
pub struct Palette {
    pub glyphs: String,
}

pub fn restore(palette: &mut Palette, saved: &Palette) {
    // Bad: frees what `palette` had, and allocates a copy.
    palette.glyphs = saved.glyphs.clone();
}
```

```rust
#[derive(Debug, Clone, Default)]
pub struct Palette {
    pub glyphs: String,
}

pub fn restore(palette: &mut Palette, saved: &Palette) {
    palette.glyphs.clone_from(&saved.glyphs);
}
```

Held by `clippy::assigning_clones`.

## Borrow from the Input Rather Than Copy It

A value parsed from a buffer can hold slices of it, `&'a str` or `&'a [u8]`,
instead of owned copies of each field, so parsing allocates nothing and copies
nothing; serde's `#[serde(borrow)]` does the same for a field of a derived
`Deserialize`. It lives no longer than the buffer, which is the point: a caller
that needs a field past it copies that field alone. A value usually handed back
unchanged is a `Cow`.

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub glyph: String,
}

#[must_use]
pub fn parse(line: &str) -> Option<Entry> {
    let (name, glyph) = line.split_once('=')?;
    // Bad: two allocations for fields the line already holds.
    Some(Entry { name: name.to_owned(), glyph: glyph.to_owned() })
}
```

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    pub name: &'a str,
    pub glyph: &'a str,
}

#[must_use]
pub fn parse(line: &str) -> Option<Entry<'_>> {
    let (name, glyph) = line.split_once('=')?;
    Some(Entry { name, glyph })
}
```

Held by review.

## Make a Large Value Where It Lives

`Box::new(value)` builds `value` first, then moves it into the allocation, and
Rust promises no elision of that copy, so a large array is built on the stack:
`Box::new([0_u8; 16 << 20])` overflows any stack smaller than its 16 MiB, as a
main thread's usually is, in a build that does not optimize the copy away, as
the `test` profile does not. `vec![0; length]` asks the allocator for zeroed
memory directly, and `.into_boxed_slice()` keeps it, all in safe code.
`Box::new_zeroed()` and `Box::new_uninit()` make one value in place too, but
hand back a `MaybeUninit`, and its `assume_init` is `unsafe`: a proof that every
byte of the value is valid for its type.

That proof is `writing-unsafe-rust`'s.

```rust,compile_fail
// fails: clippy::large_stack_arrays
#[must_use]
pub fn blank_atlas() -> Box<[u8; 1 << 20]> {
    // Bad: a megabyte built on the stack, then copied.
    Box::new([0_u8; 1 << 20])
}
```

```rust
#[must_use]
pub fn blank_atlas() -> Box<[u8]> {
    vec![0_u8; 1 << 20].into_boxed_slice()
}
```

Held by `clippy::large_stack_arrays`, for an array over 16 KiB.

Under `strict`, `clippy::large_stack_frames` also refuses a function whose frame
may pass 512,000 bytes.

## A Crate That Must Not Allocate Lists What Allocates in Its Own `clippy.toml`

A crate whose code runs where an allocation is a fault lists what allocates in
its own `clippy.toml`, each with its reason, so a call to one fails the lints:
`disallowed-methods` for `Box::new`, `Vec::new` and `Vec::with_capacity`, and
`disallowed-macros` for `vec!` and `format!`. clippy reads the `clippy.toml`
nearest the crate's manifest and no other, so the crate's file replaces the
workspace's, and repeats every key the workspace's holds. A list sees only the
calls written in the crate: `vec![]` meets it through `Vec::new`, and `vec![1,
2]` through `disallowed-macros` alone, while a `collect`, a `to_vec` or a
`Vec::default()` passes it. It holds in the crate's tests too, where a test that
builds a vector carries an `#[expect(clippy::disallowed_methods, reason =
"…")]`.

The workspace's `clippy.toml` holds the allowances for tests, which the crate's
repeats, as below.

```toml
# Bad: crates/tiles/clippy.toml, which drops the workspace's keys.
disallowed-methods = [
  { path = "alloc::vec::Vec::new" },
]
```

```toml
# crates/tiles/clippy.toml
allow-dbg-in-tests              = true
allow-expect-in-tests           = true
allow-indexing-slicing-in-tests = true
allow-panic-in-tests            = true
allow-print-in-tests            = true
allow-unwrap-in-tests           = true

disallowed-methods = [
  { path = "alloc::boxed::Box::new", reason = "no heap allocation on the paint path" },
  { path = "alloc::vec::Vec::new", reason = "no heap allocation on the paint path" },
  { path = "alloc::vec::Vec::with_capacity", reason = "no heap allocation on the paint path" },
]
disallowed-macros = [
  { path = "alloc::vec", reason = "no heap allocation on the paint path" },
  { path = "alloc::format", reason = "no heap allocation on the paint path" },
]
```

Held by `clippy::disallowed_methods` and `clippy::disallowed_macros`, for the
calls the list names.

Under `strict`, a library is `no_std` first: one that needs no heap at all has
no `extern crate alloc`, and then cannot allocate by construction, which no list
has to hold.

## A Path That Must Not Allocate Is Proven by a Test Under a Counting Allocator

A comment that a path does not allocate is a claim; a test that counts proves
it. A `#[global_allocator]` in the tests passes each call to `System` and counts
it, in a thread-local, so the tests `cargo test` runs on other threads do not
add to it; the test warms the path up once, then asserts that running it again
allocates nothing. A failure says how many allocations crept in.

```rust
extern crate alloc;

use alloc::vec::Vec;

#[derive(Debug, Default)]
pub struct Painter {
    lit_tiles: Vec<u8>,
}

impl Painter {
    // Bad: a promise nothing checks.
    /// Allocates nothing once warm.
    pub fn paint(&mut self, row: &[u8]) -> &[u8] {
        self.lit_tiles.clear();
        self.lit_tiles.extend(row.iter().copied().filter(|tile| *tile > 0));
        &self.lit_tiles
    }
}
```

```rust
extern crate alloc;

use alloc::vec::Vec;

#[derive(Debug, Default)]
pub struct Painter {
    lit_tiles: Vec<u8>,
}

impl Painter {
    pub fn paint(&mut self, row: &[u8]) -> &[u8] {
        self.lit_tiles.clear();
        self.lit_tiles.extend(row.iter().copied().filter(|tile| *tile > 0));
        &self.lit_tiles
    }
}

#[cfg(test)]
mod tests {
    use core::alloc::{GlobalAlloc, Layout};
    use core::cell::Cell;
    use std::alloc::System;
    use std::thread_local;

    use super::Painter;

    thread_local! {
        static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    }

    struct Counting;

    // SAFETY: each call is passed on to `System` unchanged, so every block it serves is `System`'s
    // and comes back to it; the count reads and writes a thread-local cell, which allocates nothing.
    #[expect(unsafe_code, reason = "a global allocator that counts what this thread allocates")]
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.with(|count| count.set(count.get().saturating_add(1)));
            // SAFETY: the caller's contract for `alloc` is passed on as it came.
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
            // SAFETY: `block` came from `alloc` or `realloc` above, which is `System`'s.
            unsafe { System.dealloc(block, layout) }
        }

        unsafe fn realloc(&self, block: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            ALLOCATIONS.with(|count| count.set(count.get().saturating_add(1)));
            // SAFETY: `block` came from `alloc` or `realloc` above, which is `System`'s.
            unsafe { System.realloc(block, layout, size) }
        }
    }

    #[global_allocator]
    static COUNTING: Counting = Counting;

    fn allocations() -> usize {
        ALLOCATIONS.with(Cell::get)
    }

    #[test]
    fn a_warm_painter_paints_without_allocating() {
        let mut painter = Painter::default();
        let row = [0, 3, 0, 5];
        painter.paint(&row);
        let before = allocations();
        let lit_count = painter.paint(&row).len();
        assert_eq!(allocations(), before, "a second row allocates nothing");
        assert_eq!(lit_count, 2, "and paints its two lit tiles");
    }
}
```

Held by the test. A binary has one `#[global_allocator]`, so it sits in the
crate's own tests, or in a file of `tests/` that holds only the tests that
count.
