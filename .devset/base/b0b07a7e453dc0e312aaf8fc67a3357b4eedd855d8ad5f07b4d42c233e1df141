# Safety Comments

Read this before an `unsafe` block, an `unsafe fn`, an `unsafe impl` or an
`unsafe trait`, before a field that unsafe code relies on, and before any change
to a module that holds unsafe code. It says where unsafe code may sit, what each
proof must establish, and what holds each rule.

How a `// SAFETY:`, an `// INVARIANT:` or a `# Safety` section is worded is
`writing-rustdoc`'s; this says what each must prove.

Each example is small enough that a safe form would serve it; it shows the shape
a proof takes where none does.

## Reach for a Safe Form First

Every unsafe block is a proof that each later change to the code around it must
keep, and a safe form is one the compiler keeps. The standard library has one
for most needs: `split_at_mut` and `chunks_exact_mut` for disjoint borrows,
`get` and iterators for indexing, `copy_from_slice`, `array::from_fn`,
`from_ne_bytes`. A bounds check is traded for a proof only once it is measured
to cost, since the optimizer removes many of them.

How a check is measured, and the loop shapes that leave none to remove, are
`tuning-rust-performance`'s.

```rust
use core::slice;

#[derive(Debug)]
pub struct Grid {
    cols: usize,
    squares: Vec<u8>,
}

impl Grid {
    // Bad: four proofs to keep, where `split_at_mut_checked` keeps its own.
    #[expect(unsafe_code, reason = "two rows of one grid borrowed mutably at once")]
    pub fn rows_mut(&mut self, top: usize) -> Option<(&mut [u8], &mut [u8])> {
        let start = top.checked_mul(self.cols)?;
        let end = self.cols.checked_mul(2)?.checked_add(start)?;
        if end > self.squares.len() {
            return None;
        }
        let base = self.squares.as_mut_ptr();
        // SAFETY: `start` is below `end`, which is within the squares.
        let upper = unsafe { base.add(start) };
        // SAFETY: `start + cols` is below `end` too.
        let lower = unsafe { upper.add(self.cols) };
        // SAFETY: the `cols` squares from `upper` end where `lower` starts, inside the vector
        // that `&mut self` holds alone.
        let upper = unsafe { slice::from_raw_parts_mut(upper, self.cols) };
        // SAFETY: the `cols` squares from `lower` end at `end`, and none of them is `upper`'s.
        let lower = unsafe { slice::from_raw_parts_mut(lower, self.cols) };
        Some((upper, lower))
    }
}
```

```rust
#[derive(Debug)]
pub struct Grid {
    cols: usize,
    squares: Vec<u8>,
}

impl Grid {
    pub fn rows_mut(&mut self, top: usize) -> Option<(&mut [u8], &mut [u8])> {
        let start = top.checked_mul(self.cols)?;
        let (upper, rest) = self.squares.get_mut(start..)?.split_at_mut_checked(self.cols)?;
        Some((upper, rest.get_mut(..self.cols)?))
    }
}
```

Held by review.

Under `strict`, `unsafe_code` makes each unsafe site say in its `#[expect]` why
it needs unsafe, which is where a reviewer asks whether a safe form serves.

## Mark Each Unsafe Site with `#[expect(unsafe_code)]` at the Narrowest Scope

Each unsafe site is allowed where it stands, with `#[expect(unsafe_code,
reason = "…")]` on the statement, the item, the `impl` block or the module that
holds the unsafe, and `#![expect]` on the crate only where unsafe is the crate's
whole purpose. The reason names the unsafe operation and why no safe form
serves, never "unsafe code". The `// SAFETY:` comment sits above the attribute
where the attribute is on the unsafe statement or `unsafe impl` itself.

The workspace denies `unsafe_code`, so no unsafe compiles without one.

```rust,compile_fail
// fails: unsafe_code
#[must_use]
pub const fn first(squares: &[u8]) -> Option<u8> {
    if squares.is_empty() {
        return None;
    }
    // Bad: unsafe the workspace was never told of.
    // SAFETY: the squares are not empty, so the first is in bounds.
    Some(unsafe { *squares.as_ptr() })
}
```

```rust
#![expect(unsafe_code, reason = "unsafe code")]
// Bad: the whole crate may hold unsafe now, for one read, and the reason says nothing.

#[must_use]
pub const fn first(squares: &[u8]) -> Option<u8> {
    if squares.is_empty() {
        return None;
    }
    // SAFETY: the squares are not empty, so the first is in bounds.
    Some(unsafe { *squares.as_ptr() })
}
```

```rust
#[must_use]
pub const fn first(squares: &[u8]) -> Option<u8> {
    if squares.is_empty() {
        return None;
    }
    // SAFETY: the squares are not empty, so the first is in bounds and initialized.
    #[expect(unsafe_code, reason = "a read the emptiness check above has bounded")]
    Some(unsafe { *squares.as_ptr() })
}
```

Held by `unfulfilled_lint_expectations`, which fails an `#[expect]` once no
unsafe is left under it, and by review for its scope and reason.

Under `strict`, `unsafe_code` refuses every unsafe block, `unsafe fn`, `unsafe
impl`, `unsafe trait`, `unsafe extern` block and unsafe attribute outside an
`#[expect]`.

## A Safe Function Is Sound for Every Input

A function without `unsafe` in its signature promises that no caller, whatever
it passes, can cause undefined behaviour through it. A raw pointer, an index or
a length that decides whether its unsafe code is sound is either checked, or the
function is an `unsafe fn` whose `# Safety` hands the obligation to the caller.
A comment that says "the caller passes a valid pointer" in a safe function is a
proof of nothing.

```rust,compile_fail
// fails: clippy::not_unsafe_ptr_arg_deref
// Bad: safe to call, and any caller can pass a dangling pointer.
#[expect(unsafe_code, reason = "a read of the square the caller points at")]
#[must_use]
pub const fn first(squares: *const u8) -> u8 {
    // SAFETY: the caller passes a live square.
    unsafe { squares.read() }
}
```

```rust
#[must_use]
pub fn square(squares: &[u8], at: usize) -> Option<u8> {
    squares.get(at).copied()
}
```

Held by `clippy::not_unsafe_ptr_arg_deref`, which refuses a safe function the
crate exports that dereferences a raw pointer argument. One the crate does not
export, and an index, length or handle a caller passes unchecked, are held by
review.

## An `unsafe fn` Says What Its Caller Keeps, and Its Body Is Still Unsafe Blocks

An `unsafe fn` moves a proof to its caller, so its `# Safety` section lists each
obligation the caller keeps, in terms the caller can check. Its body proves each
unsafe operation of its own in a block, from that contract or from facts in
scope, as any other body does. An `unsafe fn` beside a safe twin is named
`_unchecked`, and the twin checks and calls it.

```rust,compile_fail
// fails: unsafe_op_in_unsafe_fn
/// The square at `at`, unchecked.
///
/// # Safety
/// `at` is below `squares.len()`.
#[expect(unsafe_code, reason = "an unchecked read for callers that have bounded the index")]
#[must_use]
pub unsafe fn square_unchecked(squares: &[u8], at: usize) -> u8 {
    // Bad: the body's unsafe call has no block, and so no proof of its own.
    *squares.get_unchecked(at)
}
```

```rust
/// The square at `at`, unchecked.
///
/// # Safety
/// `at` is below `squares.len()`.
#[expect(unsafe_code, reason = "an unchecked read for callers that have bounded the index")]
#[must_use]
pub unsafe fn square_unchecked(squares: &[u8], at: usize) -> u8 {
    // SAFETY: the caller promises `at` is below the length, which is all `get_unchecked` asks.
    unsafe { *squares.get_unchecked(at) }
}

#[must_use]
pub fn square(squares: &[u8], at: usize) -> Option<u8> {
    if at >= squares.len() {
        return None;
    }
    // SAFETY: `at` is below the length, checked above.
    #[expect(unsafe_code, reason = "the checked twin of an unchecked read")]
    Some(unsafe { square_unchecked(squares, at) })
}
```

Held by `unsafe_op_in_unsafe_fn`, which the lint table denies, where edition
2024 only warns, and by `clippy::missing_safety_doc`, which refuses an exported
`unsafe fn` or `unsafe trait` with no `# Safety`; a private one's is held by
review.

Under `strict`, `clippy::unnecessary_safety_doc` refuses a `# Safety` section on
a safe function the crate exports.

## Unsafe Code Trusts Only What Its Module Controls

The soundness of an unsafe block rests on facts, and a fact is only as sure as
the code that keeps it. Code in the same module can keep one: a private field,
written by the module's own functions. A safe trait's impl cannot, since it may
be wrong without any unsafe code: an `ExactSizeIterator` whose `len` lies, an
`Ord` that is not an order, a `Hash` that changes, a `Clone` that panics. Nor
can a caller's closure, which may panic or call back in, nor a destructor, since
`mem::forget` is safe and a value may never be dropped. Unsafe code that must
trust an impl makes its trait an `unsafe trait`, whose `# Safety` says what an
impl promises.

```rust
#[expect(unsafe_code, reason = "writes the iterator has said will fit")]
pub fn place<I: ExactSizeIterator<Item = u8>>(row: &mut [u8], tiles: I) {
    if tiles.len() > row.len() {
        return;
    }
    for (at, tile) in tiles.enumerate() {
        // Bad: `len` is a safe method, and an iterator that reports too few writes past the row.
        // SAFETY: `len` said every tile fits the row.
        let square = unsafe { row.get_unchecked_mut(at) };
        *square = tile;
    }
}
```

```rust
pub fn place<I: Iterator<Item = u8>>(row: &mut [u8], tiles: I) {
    for (square, tile) in row.iter_mut().zip(tiles) {
        *square = tile;
    }
}
```

Held by review.

## One Unsafe Operation a Block

Each unsafe operation has preconditions of its own, and a block that holds two
has one comment for both, so no reader can tell which fact proves which. An
offset and a read, `ptr.add(at).read()`, are two operations, and the offset has
a precondition of its own: `add` lands inside the allocation, or one past its
end, even where nothing is read through it. A call whose argument is another
unsafe call is two as well. Each goes in a block of its own, with its own `//
SAFETY:`, and a proof one line up is cited, "as above", not repeated.

```rust,compile_fail
// fails: clippy::multiple_unsafe_ops_per_block
/// The square `at` squares past `squares`.
///
/// # Safety
/// `squares` and the `at` squares after it lie inside one live, initialized row that nothing
/// writes during the call.
#[expect(unsafe_code, reason = "an unchecked read inside a row the caller holds")]
#[must_use]
pub const unsafe fn square_at(squares: *const u8, at: usize) -> u8 {
    // Bad: an offset and a read, and one proof for both.
    // SAFETY: the caller promises the row reaches past `at`.
    unsafe { squares.add(at).read() }
}
```

```rust
/// The square `at` squares past `squares`.
///
/// # Safety
/// `squares` and the `at` squares after it lie inside one live, initialized row that nothing
/// writes during the call.
#[expect(unsafe_code, reason = "an unchecked read inside a row the caller holds")]
#[must_use]
pub const unsafe fn square_at(squares: *const u8, at: usize) -> u8 {
    // SAFETY: the caller promises the `at` squares after `squares` lie in one row, so the offset
    // stays inside it.
    let square = unsafe { squares.add(at) };
    // SAFETY: `square` is inside that live row, whose squares are initialized and which nothing
    // writes, and a `u8` is aligned anywhere.
    unsafe { square.read() }
}
```

Held by review.

Under `strict`, `clippy::multiple_unsafe_ops_per_block` refuses a block with
more than one unsafe operation.

## A `// SAFETY:` Proves Each Precondition from a Fact in Scope

An unsafe operation's own `# Safety` lists what it needs:
`slice::from_raw_parts` asks for a pointer that is non-null and aligned, valid
for reads of the whole length within one allocation, to initialized values that
nothing mutates for the slice's lifetime, and a length within `isize::MAX`
bytes. The comment above the block names, for each, the fact that discharges it:
a check above it, a type's `// INVARIANT:`, the caller's `# Safety` contract, or
the step just before. A comment that restates the operation, or says the pointer
is valid, proves nothing a reviewer can check.

```rust
#[must_use]
pub const fn halves(squares: &[u8], mid: usize) -> Option<(&[u8], &[u8])> {
    if mid > squares.len() {
        return None;
    }
    // Bad: names the operation, and none of what it needs.
    // SAFETY: splits the squares at `mid`.
    #[expect(unsafe_code, reason = "a split the check above has bounded")]
    Some(unsafe { squares.split_at_unchecked(mid) })
}
```

```rust
#[must_use]
pub const fn halves(squares: &[u8], mid: usize) -> Option<(&[u8], &[u8])> {
    if mid > squares.len() {
        return None;
    }
    // SAFETY: `mid` is at most the length, checked above, which is all `split_at_unchecked`
    // needs.
    #[expect(unsafe_code, reason = "a split the check above has bounded")]
    Some(unsafe { squares.split_at_unchecked(mid) })
}
```

Held by review.

Under `strict`, `clippy::undocumented_unsafe_blocks` refuses an unsafe block or
`unsafe impl` with no `// SAFETY:` above it or above its attributes, and
`clippy::unnecessary_safety_comment` refuses one above safe code.

## An `// INVARIANT:` Sits on Each Field a Proof Relies On

A proof that rests on a field's value rests on every function that writes the
field, safe ones included: a safe method that sets a length wrong breaks an
unsafe block as surely as a wrong unsafe one. So the field is private, and the
fact it keeps is written where it is declared, as `// INVARIANT:`, with its
writers named; each `// SAFETY:` that relies on it cites "the field INVARIANT".
A change to the field then meets what it must keep.

```rust
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    // Bad: public, so safe code can build a row over any address, and nothing says what holds.
    pub base: NonNull<u8>,
    pub len: usize,
    pub squares: PhantomData<&'a [u8]>,
}

impl<'a> Row<'a> {
    #[must_use]
    pub const fn squares(self) -> &'a [u8] {
        // SAFETY: `base` and `len` are a live row's.
        #[expect(unsafe_code, reason = "a view of the row the fields describe")]
        unsafe { slice::from_raw_parts(self.base.as_ptr(), self.len) }
    }
}
```

```rust
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    // INVARIANT: `base` and `len` are the pointer and length of one `&'a [u8]`; `new` is their
    // only writer.
    base: NonNull<u8>,
    len: usize,
    squares: PhantomData<&'a [u8]>,
}

impl<'a> Row<'a> {
    #[must_use]
    pub const fn new(squares: &'a [u8]) -> Self {
        Self { base: NonNull::from_ref(squares).cast(), len: squares.len(), squares: PhantomData }
    }

    #[must_use]
    pub const fn squares(self) -> &'a [u8] {
        // SAFETY: by the field INVARIANT these are the squares of one `&'a [u8]`: non-null,
        // aligned, initialized, inside one allocation of at most `isize::MAX` bytes, and shared
        // for `'a`, so no `&mut` overlaps them.
        #[expect(unsafe_code, reason = "a view of the row `new` borrowed")]
        unsafe { slice::from_raw_parts(self.base.as_ptr(), self.len) }
    }
}
```

Held by review.

## Every `unsafe impl` Proves Its Trait, and `Send` and `Sync` Carry Their Bounds

An `unsafe impl` promises what the trait's `# Safety` asks, and its `// SAFETY:`
says why each obligation holds. `Send` says that moving the value to another
thread is sound, and `Sync` that sharing `&self` across threads is; a raw
pointer or `NonNull` field takes both away, an `UnsafeCell` takes `Sync`, and an
impl gives them back only with the bounds its access needs. A type that owns its
`T`s alone is `Send` only where `T: Send`, and a handle that shares them, as
`Arc` does, only where `T: Send + Sync`, since each thread that holds one
reaches the same `T`. It is `Sync` with `T: Sync` where it lends `&T` to several
threads at once, with `T: Send` where `&self` hands out a `&mut T` or moves a
`T` in or out, as a lock does, and with both where it does both, as `RwLock` and
`OnceLock` do; `atomics.md` shows the lock. A type that owns `T` through a
pointer holds `PhantomData<T>`, so drop check and the auto traits see the `T`s
it owns.

```rust
extern crate alloc;

use alloc::boxed::Box;
use core::marker::PhantomData;
use core::ptr::NonNull;

#[derive(Debug)]
pub struct Board<T> {
    // INVARIANT: from `Box::leak` in `new`, owned by this board alone, and freed once, in `drop`.
    squares: NonNull<[T]>,
    owns: PhantomData<T>,
}

// Bad: no `T: Send`, so a board of `Rc`s crosses threads, and two threads count one `Rc`.
// SAFETY: the board owns its squares alone, so moving it moves them.
#[expect(unsafe_code, reason = "a `NonNull` field takes away the auto `Send`")]
unsafe impl<T> Send for Board<T> {}

impl<T> Board<T> {
    #[must_use]
    pub fn new(squares: Box<[T]>) -> Self {
        Self { squares: NonNull::from(Box::leak(squares)), owns: PhantomData }
    }
}

impl<T> Drop for Board<T> {
    fn drop(&mut self) {
        // SAFETY: by the field INVARIANT the squares came from `Box::leak` and are freed only
        // here.
        #[expect(unsafe_code, reason = "the board frees the squares it leaked")]
        let squares = unsafe { Box::from_raw(self.squares.as_ptr()) };
        drop(squares);
    }
}
```

```rust
extern crate alloc;

use alloc::boxed::Box;
use core::marker::PhantomData;
use core::ptr::NonNull;

#[derive(Debug)]
pub struct Board<T> {
    // INVARIANT: from `Box::leak` in `new`, owned by this board alone, and freed once, in `drop`.
    squares: NonNull<[T]>,
    owns: PhantomData<T>,
}

// SAFETY: by the field INVARIANT the board owns its squares alone, so sending it sends its `T`s,
// which `T: Send` allows.
#[expect(unsafe_code, reason = "a `NonNull` field takes away the auto `Send`")]
unsafe impl<T: Send> Send for Board<T> {}

// SAFETY: `&self` only reads the squares, so sharing it shares `&T`s, which `T: Sync` allows.
#[expect(unsafe_code, reason = "a `NonNull` field takes away the auto `Sync`")]
unsafe impl<T: Sync> Sync for Board<T> {}

impl<T> Board<T> {
    #[must_use]
    pub fn new(squares: Box<[T]>) -> Self {
        Self { squares: NonNull::from(Box::leak(squares)), owns: PhantomData }
    }

    #[must_use]
    pub const fn squares(&self) -> &[T] {
        // SAFETY: by the field INVARIANT the squares are live and this board's, and `&self` lets
        // no `&mut` of them exist.
        #[expect(unsafe_code, reason = "a view of the squares the board owns")]
        unsafe { self.squares.as_ref() }
    }
}

impl<T> Drop for Board<T> {
    fn drop(&mut self) {
        // SAFETY: by the field INVARIANT the squares came from `Box::leak` and are freed only
        // here.
        #[expect(unsafe_code, reason = "the board frees the squares it leaked")]
        let squares = unsafe { Box::from_raw(self.squares.as_ptr()) };
        drop(squares);
    }
}
```

Held by review.

Under `strict`, `clippy::undocumented_unsafe_blocks` refuses an `unsafe impl`
with no `// SAFETY:`, and `clippy::non_send_fields_in_send_ty` a `Send` impl
over a field that is not `Send`, a bare `T` included. It lets through a field
behind a raw pointer, a `NonNull<T>` or a `NonNull<Rc<u8>>`, as above, which is
held by review.

## A Marker Field, Not a Doc, Keeps a Type on One Thread

The auto traits follow a type's fields, so a type whose meaning ties it to one
thread, a slot in a `thread_local!` table or a handle bound to the thread that
made it, is `Send` and `Sync` unless a field says otherwise. A marker says so,
where the compiler holds it: `PhantomData<*const ()>` takes both away, and
`PhantomData<Cell<()>>` takes `Sync` alone. `impl !Sync` needs nightly's
`negative_impls`, so a crate that builds on stable writes the marker.

```rust
// Bad: a doc that says "one thread only" binds no caller; this is `Send` and `Sync`.
#[derive(Debug, Clone, Copy)]
pub struct Brush {
    slot: usize,
}

impl Brush {
    #[must_use]
    pub const fn slot(self) -> usize {
        self.slot
    }
}
```

```rust
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct Brush {
    slot: usize,
    // Neither `Send` nor `Sync`: the slot is one in this thread's palette.
    here: PhantomData<*const ()>,
}

impl Brush {
    #[must_use]
    pub const fn slot(self) -> usize {
        self.slot
    }
}
```

Held by the compiler, which refuses to send or share the type, `E0277`, and by a
compile-fail test that pins it, as `verifying.md` shows.

## `debug_assert!` Checks the Contract of an `unsafe fn`, and Proves Nothing

An `unsafe fn`'s caller keeps its contract; a `debug_assert!` of the part a
check can see catches a broken one in the tests, at no cost to a release build.
Its message names the violation. A build without debug assertions, a release
build among them, skips it, so a safe function that rests its unsafe code on one
is unsound there: its check is a real one, or it is an `unsafe fn`.

```rust
#[must_use]
pub fn square(squares: &[u8], at: usize) -> u8 {
    // Bad: gone in a release build, and with it every guard on the read below.
    debug_assert!(at < squares.len(), "an index past the row");
    // SAFETY: the assertion above bounds `at`.
    #[expect(unsafe_code, reason = "a read the assertion has bounded")]
    unsafe { *squares.get_unchecked(at) }
}
```

```rust
/// The square at `at`, unchecked.
///
/// # Safety
/// `at` is below `squares.len()`.
#[expect(unsafe_code, reason = "an unchecked read for callers that have bounded the index")]
#[must_use]
pub unsafe fn square_unchecked(squares: &[u8], at: usize) -> u8 {
    debug_assert!(at < squares.len(), "an index past the row");
    // SAFETY: the caller promises `at` is below the length, which is all `get_unchecked` asks.
    unsafe { *squares.get_unchecked(at) }
}
```

Held by review.

Under `strict`, `clippy::missing_assert_message` refuses a `debug_assert!` with
no message.

## A Panic Midway Leaves Nothing a Drop Would Misread

A panic unwinds through an unsafe function part way, and whatever it leaves is
dropped as though it were whole: a length past what was written drops values
that were never made, and a block freed but still named is freed twice. Each
step leaves a state a drop reads correctly: write, then count; allocate the new
before freeing the old; and where a run is built in place, a guard counts what
landed, so a failure drops exactly those.

```rust,compile_fail
// fails: clippy::uninit_vec
use core::ptr;

#[derive(Debug, Clone)]
pub struct Tile {
    pub label: String,
}

#[expect(unsafe_code, reason = "a row written in place")]
#[must_use]
pub fn repeat(tile: &Tile, times: usize) -> Vec<Tile> {
    let mut row = Vec::with_capacity(times);
    // Bad: counted before written, so a `clone` that panics drops `times` tiles never made.
    // SAFETY: the loop below writes every one.
    unsafe { row.set_len(times) };
    for slot in &mut row {
        // SAFETY: a slot of the row, not yet written.
        unsafe { ptr::write(slot, tile.clone()) };
    }
    row
}
```

```rust
#[derive(Debug, Clone)]
pub struct Tile {
    pub label: String,
}

#[must_use]
pub fn repeat(tile: &Tile, times: usize) -> Vec<Tile> {
    let mut row = Vec::with_capacity(times);
    for slot in row.spare_capacity_mut().iter_mut().take(times) {
        slot.write(tile.clone());
    }
    // SAFETY: `with_capacity` left room for `times`, and the loop wrote the first `times` spare
    // slots; a `clone` that panicked left the length at zero, leaking what it wrote.
    #[expect(unsafe_code, reason = "a row counted once its tiles are written")]
    unsafe {
        row.set_len(times);
    }
    row
}
```

Where a run is built in place and no length counts it, a guard does: it drops
what the build wrote if the build stops early, and is told to drop nothing once
the run is whole.

```rust
use core::mem::MaybeUninit;

#[derive(Debug, Clone)]
pub struct Tile {
    pub label: String,
}

struct Written<'a> {
    row: &'a mut [MaybeUninit<Tile>],
    // INVARIANT: the first `written` slots of `row` hold tiles that nothing else drops.
    written: usize,
}

impl Drop for Written<'_> {
    fn drop(&mut self) {
        for slot in self.row.iter_mut().take(self.written) {
            // SAFETY: by the field INVARIANT the slot holds a tile that nothing else drops.
            #[expect(unsafe_code, reason = "drops the tiles a stopped build wrote")]
            unsafe {
                slot.assume_init_drop();
            }
        }
    }
}

#[must_use]
pub fn fill(tile: &Tile) -> [Tile; 4] {
    let mut row = [const { MaybeUninit::<Tile>::uninit() }; 4];
    let mut guard = Written { row: &mut row, written: 0 };
    while let Some(slot) = guard.row.get_mut(guard.written) {
        slot.write(tile.clone());
        guard.written = guard.written.wrapping_add(1);
    }
    // The row owns its tiles from here, so the guard drops none.
    guard.written = 0;
    drop(guard);
    // SAFETY: the loop wrote every slot before the guard let go of them.
    #[expect(unsafe_code, reason = "a row whose every slot the loop above wrote")]
    unsafe {
        MaybeUninit::<[Tile; 4]>::from(row).assume_init()
    }
}
```

Held by `clippy::uninit_vec`, which refuses a `set_len` straight after a
`with_capacity` or a `reserve`, and by review.

## A Value Moved Out with `ptr::read` Is Not Dropped Again Where It Lay

`ptr::read` copies a value's bytes out and leaves them where they were, so two
places now own the value, and the second drop frees what the first freed. A
field moved out of a type that has its own `Drop`, which the compiler refuses to
move from, is read out of a `ManuallyDrop` of the whole, which never drops;
where the field has a cheap empty value, `mem::take` does it with no unsafe.

```rust
use core::ptr;

#[derive(Debug)]
pub struct Draft {
    squares: Vec<u8>,
}

impl Drop for Draft {
    fn drop(&mut self) {
        self.squares.fill(0);
    }
}

impl Draft {
    #[must_use]
    pub fn commit(self) -> Vec<u8> {
        // Bad: `self` still drops its squares at the end of this call, and the caller drops them
        // again.
        // SAFETY: `squares` is a live field of `self`.
        #[expect(unsafe_code, reason = "the squares moved out of a draft with its own `Drop`")]
        unsafe {
            ptr::read(&raw const self.squares)
        }
    }
}
```

```rust
use core::mem::ManuallyDrop;
use core::ptr;

#[derive(Debug)]
pub struct Draft {
    squares: Vec<u8>,
}

impl Drop for Draft {
    fn drop(&mut self) {
        self.squares.fill(0);
    }
}

impl Draft {
    #[must_use]
    pub fn commit(self) -> Vec<u8> {
        let this = ManuallyDrop::new(self);
        // SAFETY: `this` is never dropped, so the squares read out here have one owner, the
        // caller.
        #[expect(unsafe_code, reason = "the squares moved out of a draft with its own `Drop`")]
        unsafe {
            ptr::read(&raw const this.squares)
        }
    }
}
```

Held by review.
