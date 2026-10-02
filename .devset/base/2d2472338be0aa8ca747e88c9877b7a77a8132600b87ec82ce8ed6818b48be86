# Ownership

Read this before choosing how a function takes or returns a value, before a
`.clone()`, an `Arc`, an `Rc`, a `Cow` or a named lifetime, and when the borrow
checker refuses a call. Slices over owned parameters, `.clone()` on a `Copy`
value and needless lifetimes are clippy's defaults, in `lints.md`'s table. It
says who owns what, and how a signature says so.

## Take a Value Only Where the Function Keeps It

A parameter taken by value is a value the caller gives up, which is right when
the function stores it or hands it on, and a needless move or clone otherwise. A
function that keeps what it is given takes it by value, so a caller that has one
to spare moves it in rather than the function cloning a borrow. One that only
reads takes a slice, `&str` or `&[T]`, which `clippy::ptr_arg` holds.

```rust,compile_fail
// fails: clippy::needless_pass_by_value
// Bad: the labels are only read, but the caller must give them up.
#[must_use]
pub fn longest(labels: Vec<String>) -> usize {
    labels.iter().map(String::len).max().unwrap_or(0)
}
```

```rust
#[derive(Debug, Default)]
pub struct Legend {
    labels: Vec<String>,
}

impl Legend {
    /// Kept, so taken by value: a caller with a `String` moves it in.
    pub fn add(&mut self, label: String) {
        self.labels.push(label);
    }

    #[must_use]
    pub fn longest(&self) -> usize {
        self.labels.iter().map(String::len).max().unwrap_or(0)
    }
}
```

Held by `clippy::needless_pass_by_value`.

## Pass a Small `Copy` Value by Value

A value that is `Copy` and a few words wide costs nothing to copy, and a
reference to it costs an indirection and a lifetime to read; it is taken by
value. A large value is borrowed, or boxed where it moves often. A `*Spec` is no
exception: a small `Copy` one goes by value, `Grid::new(spec)`, and one that is
large or not `Copy`, as a spec that holds a path, by reference.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

// Bad: a reference to four bytes.
#[must_use]
pub const fn right(at: &Pos) -> Pos {
    Pos { col: at.col.saturating_add(1), row: at.row }
}
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

#[must_use]
pub const fn right(at: Pos) -> Pos {
    Pos { col: at.col.saturating_add(1), row: at.row }
}
```

Held by review: `clippy::trivially_copy_pass_by_ref` and
`large_types_passed_by_value` see only a function the crate does not export.

## Clone Only What Must Be Owned Twice

A clone is a copy of everything the value owns, so it appears where two owners
need the value, not to quiet the borrow checker or to save a caller a borrow.
Before one, the code is shaped so a borrow lasts long enough: a shorter scope, a
reference kept instead of a value, a value moved rather than copied. A method
that reads a field that is not `Copy` returns a borrow of it, and a caller that
keeps it clones it there.

```rust
#[derive(Debug, Default)]
pub struct Legend {
    labels: Vec<String>,
}

impl Legend {
    // Bad: a new `String` for every caller, though most only read it.
    #[must_use]
    pub fn first(&self) -> Option<String> {
        self.labels.first().cloned()
    }
}
```

```rust
#[derive(Debug, Default)]
pub struct Legend {
    labels: Vec<String>,
}

impl Legend {
    #[must_use]
    pub fn first(&self) -> Option<&str> {
        self.labels.first().map(String::as_str)
    }
}
```

Held by review, and by `clippy::implicit_clone` for a `to_owned` that is a
clone.

Under `strict`, `clippy::redundant_clone` refuses a clone whose original is
never used again.

## Clone an `Arc` as `Arc::clone(&board)`

A reference-counted pointer is cloned to share it, which costs a count, not a
copy of what it points to. `Arc::clone(&board)` says so where `board.clone()`
reads as a deep copy.

```rust,compile_fail
// fails: clippy::clone_on_ref_ptr
extern crate alloc;

use alloc::sync::Arc;

#[must_use]
pub fn share(board: &Arc<[u8]>) -> Arc<[u8]> {
    // Bad: reads as a copy of every square.
    board.clone()
}
```

```rust
extern crate alloc;

use alloc::sync::Arc;

#[must_use]
pub fn share(board: &Arc<[u8]>) -> Arc<[u8]> {
    Arc::clone(board)
}
```

Held by review.

Under `strict`, `clippy::clone_on_ref_ptr` refuses `.clone()` on an `Arc` or an
`Rc`.

## Share Across Threads with `Arc`, Within One with `Rc`

`Arc` pays for an atomic count so its value can cross threads, which is worth it
only for a value that is `Send` and `Sync`; within one thread, `Rc` and
`RefCell` do the same work without the atomics. A value shared across threads
and changed there is `Arc<Mutex<T>>`, or an atomic for a lone integer or flag,
whose orderings `writing-unsafe-rust` teaches, as it does an `unsafe impl` of
`Send` or `Sync`.

```rust,compile_fail
// fails: clippy::arc_with_non_send_sync
extern crate alloc;

use alloc::sync::Arc;
use core::cell::RefCell;

#[derive(Debug, Default)]
pub struct Board {
    squares: Vec<u8>,
}

// Bad: a `RefCell` is not `Sync`, so this `Arc` can never cross a thread.
#[must_use]
pub fn shared() -> Arc<RefCell<Board>> {
    Arc::new(RefCell::new(Board::default()))
}

pub fn place(board: &RefCell<Board>, tile: u8) {
    board.borrow_mut().squares.push(tile);
}
```

```rust
extern crate alloc;

use alloc::rc::Rc;
use core::cell::RefCell;

#[derive(Debug, Default)]
pub struct Board {
    squares: Vec<u8>,
}

#[must_use]
pub fn shared() -> Rc<RefCell<Board>> {
    Rc::new(RefCell::new(Board::default()))
}

pub fn place(board: &RefCell<Board>, tile: u8) {
    board.borrow_mut().squares.push(tile);
}
```

Held by `clippy::arc_with_non_send_sync`.

Under `strict`, `clippy::rc_mutex` refuses an `Rc<Mutex<T>>`,
`clippy::rc_buffer` an `Rc<String>` or `Rc<Vec<T>>` for the `Rc<str>` or
`Rc<[T]>` that saves an allocation, and `clippy::mutex_atomic` and
`clippy::mutex_integer` a `Mutex` around what an atomic holds.

## Return `Cow` When the Input Usually Comes Back Unchanged

A function that most often returns what it was given, and sometimes a changed
copy, returns `Cow<'_, str>` or `Cow<'_, [T]>`: the common case borrows, and
only the rare one allocates. A function that always changes its input returns an
owned value.

```rust
// Bad: a copy of every label, though most are already lowercase.
#[must_use]
pub fn label(name: &str) -> String {
    name.to_ascii_lowercase()
}
```

```rust
extern crate alloc;

use alloc::borrow::Cow;

#[must_use]
pub fn label(name: &str) -> Cow<'_, str> {
    if name.bytes().all(|byte| byte.is_ascii_lowercase()) {
        Cow::Borrowed(name)
    } else {
        Cow::Owned(name.to_ascii_lowercase())
    }
}
```

Held by review.

## A Type That Borrows Shows It: `Formatter<'_>`

A type with a lifetime parameter is written with it, `'_` where elision fills it
in, so a reader sees at the signature that the value borrows. A lifetime is
named only to tie an output to one input of several, as `fn longer<'a>(top: &'a
[u8], bottom: &'a [u8]) -> &'a [u8]`; elsewhere elision supplies it.

```rust,compile_fail
// fails: elided_lifetimes_in_paths
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl fmt::Display for Pos {
    // Bad: nothing shows that the formatter borrows.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.col, self.row)
    }
}
```

```rust
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub col: u16,
    pub row: u16,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.col, self.row)
    }
}
```

Held by `elided_lifetimes_in_paths`, in `rust_2018_idioms`.

## A Returned `impl Trait` Captures Every Borrow, and `use<>` Captures Fewer

In edition 2024, an `impl Trait` in a return type captures every lifetime in
scope, so `+ '_` is no longer written. A value that does not borrow its inputs
says so with `use<>`, or names the lifetimes it keeps, `use<'a, T>`; without
that, the caller's borrow lasts as long as the value.

```rust,compile_fail
// fails: E0502
pub fn numbers(squares: &[u8]) -> impl Iterator<Item = usize> {
    0..squares.len()
}

#[must_use]
pub fn clear(squares: &mut Vec<u8>) -> usize {
    let numbers = numbers(squares);
    // Bad: `numbers` still borrows `squares`, though it holds only a range.
    squares.clear();
    numbers.count()
}
```

```rust
pub fn numbers(squares: &[u8]) -> impl Iterator<Item = usize> + use<> {
    0..squares.len()
}

#[must_use]
pub fn clear(squares: &mut Vec<u8>) -> usize {
    let numbers = numbers(squares);
    squares.clear();
    numbers.count()
}
```

Held by the borrow checker, `E0502`, at the caller.
