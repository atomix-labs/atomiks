# Pointers

Read this before a raw pointer is made, cast, offset or dereferenced, before a
pointer becomes an integer or an integer a pointer, before `transmute`,
`MaybeUninit` or `mem::zeroed`, and before an `extern` block or an exported
symbol. It says how a pointer keeps the right to reach what it points at, and
what each conversion must prove.

A pointer is an address and a provenance: the memory it may reach, and for how
long. A pointer derived from a reference or an allocation inherits that one's
provenance, and nothing can widen it. The standard library's strict provenance
functions keep the two apart, so no pointer's provenance is ever guessed.

## An Address Is `addr()`, Never `as usize`

A cast from a pointer to an integer with `as` exposes the pointer's provenance
to a table the compiler must then assume anything may draw on, which costs
optimizations and hampers the tools that check provenance. `addr()` reads the
address alone. An address changed and put back, as a tag in the spare low bits
of an aligned pointer, is `map_addr(|addr| addr | 1)`, which keeps the
provenance; a pointer that is never dereferenced, a sentinel or an address a
test needs, is `ptr::without_provenance(addr)`, and an aligned one that is never
read is `NonNull::dangling()`.

```rust,compile_fail
// fails: implicit_provenance_casts
#[must_use]
pub fn is_aligned(squares: &[u8]) -> bool {
    // Bad: the cast exposes the pointer's provenance, to read an address.
    (squares.as_ptr() as usize).is_multiple_of(64)
}
```

```rust
#[must_use]
pub fn is_aligned(squares: &[u8]) -> bool {
    squares.as_ptr().addr().is_multiple_of(64)
}
```

Held by review.

Under `nightly`, `implicit_provenance_casts` refuses an `as` cast between a
pointer and an integer, either way.

Under `strict`, `clippy::as_conversions` refuses every `as`.

## Rebuild a Pointer from One Whose Provenance Covers the Place

A pointer made from a reference reaches at least what that reference covers, and
whether it reaches further Rust has not settled: Stacked Borrows, Miri's default
model, refuses a read of the square after one from `&squares[3]`, and Tree
Borrows accepts it. The workspace treats the read as undefined behaviour,
however the address checks out, which is sound under either. A pointer that must
reach around the place a handle names takes the handle's address and the owner's
provenance, `base.with_addr(handle.addr())`, where `base` is the owner's own
pointer; `add` and `byte_add` then offset within the owner. Where the owner is a
slice, `get(offset)` does the same with no unsafe at all; `with_addr` is for an
owner that is itself a pointer: a mapping, an allocation, a header before its
payload.

```rust
use core::ptr::NonNull;

#[derive(Debug)]
pub struct Grid {
    squares: Box<[u8]>,
}

impl Grid {
    #[must_use]
    pub fn square(&self, index: usize) -> Option<NonNull<u8>> {
        self.squares.get(index).map(NonNull::from)
    }

    /// The square after `square`.
    ///
    /// # Safety
    /// `square` came from `square` on this grid, and is not its last.
    #[expect(unsafe_code, reason = "a read of the square after one a handle names")]
    #[must_use]
    pub const unsafe fn after(&self, square: NonNull<u8>) -> u8 {
        // SAFETY: the caller promises a square of this grid that is not its last, so the square
        // after it is one too.
        let next_square = unsafe { square.add(1) };
        // Bad: `square` came from a `&u8`, whose provenance may cover that square alone.
        // SAFETY: as above.
        unsafe { next_square.read() }
    }
}
```

```rust
use core::ptr::NonNull;

#[derive(Debug)]
pub struct Grid {
    squares: Box<[u8]>,
}

impl Grid {
    #[must_use]
    pub fn square(&self, index: usize) -> Option<NonNull<u8>> {
        self.squares.get(index).map(NonNull::from)
    }

    #[must_use]
    pub fn after(&self, square: NonNull<u8>) -> Option<u8> {
        let base = NonNull::from(&*self.squares).cast::<u8>();
        let next_address = square.addr().checked_add(1)?;
        let offset = next_address.get().checked_sub(base.addr().get())?;
        if offset >= self.squares.len() {
            return None;
        }
        // SAFETY: `with_addr` gives `next_address` the provenance of every square, which are
        // live and initialized for `&self`, and `offset` is below their count, checked above.
        #[expect(unsafe_code, reason = "a read at an address a handle names, on the grid's own provenance")]
        Some(unsafe { base.with_addr(next_address).read() })
    }
}
```

Held by review.

## Exposed Provenance Is for an Address from Outside the Program

`expose_provenance` and `with_exposed_provenance` are what an `as` cast between
a pointer and an integer means, said as a call. They are for an address the
program did not allocate, a device register at a fixed address, which counts as
exposed, or a foreign interface that hands a pointer back only as an integer,
exposed on the way out. Rebuilt from a bare integer, a pointer's provenance is
whichever exposed one the compiler picks, and undefined behaviour if none covers
the use. Inside the program a pointer keeps its type: a field, an argument or an
`AtomicPtr`, never an address saved to be turned back.

```rust
use core::ptr;

#[derive(Debug)]
pub struct Cursor {
    // Bad: an address that dropped its provenance, so the read below guesses which it had.
    address: usize,
}

impl Cursor {
    #[must_use]
    pub fn new(square: &u8) -> Self {
        Self { address: ptr::from_ref(square).expose_provenance() }
    }

    /// The square the cursor is on.
    ///
    /// # Safety
    /// The square `new` took is still live.
    #[expect(unsafe_code, reason = "a read at the address the cursor keeps")]
    #[must_use]
    pub const unsafe fn square(&self) -> u8 {
        let square = ptr::with_exposed_provenance::<u8>(self.address);
        // SAFETY: the caller promises the square is live, and `new` exposed its provenance.
        unsafe { square.read() }
    }
}
```

```rust
use core::ptr;

/// The tile display's status register, which the board's memory map fixes.
const STATUS: usize = 0x4000_1000;

/// The display's status word.
///
/// # Safety
/// The display is mapped at `STATUS`, as on the board this crate is built for.
#[expect(unsafe_code, reason = "a read of a device register at a fixed address")]
#[must_use]
pub unsafe fn status() -> u32 {
    let register = ptr::with_exposed_provenance::<u32>(STATUS);
    // SAFETY: the caller promises the display is mapped at `STATUS`, memory outside the program
    // counts as exposed, and the register is aligned for a `u32`.
    unsafe { register.read_volatile() }
}
```

Held by review.

Under `nightly`, `implicit_provenance_casts` refuses the `as` spelling of
either, so each is written as the call it is.

## Cast with `cast` and `&raw`, and Write Only Through a Pointer from a `&mut`

`as` between pointers changes the pointee type, the mutability and, to or from
an integer, the provenance, and says nothing of which it meant. Each has its own
spelling: `cast::<T>()` changes the type, `cast_mut` and `cast_const` the
mutability, `ptr::from_ref` and `ptr::from_mut` take a reference, and `&raw
const place` or `&raw mut place` a place that must not be referenced: a field of
a packed struct, or one not yet initialized. A pointer is written through only
where it came from a `&mut` or from the owner: one from a shared borrow may not
be written outside an `UnsafeCell`, and `cast_mut` does not change that; it is
for an interface that takes `*mut` and never writes, as `AtomicPtr` does.

```rust,compile_fail
// fails: clippy::ptr_cast_constness
#[expect(unsafe_code, reason = "a write through the row's base")]
pub fn clear_first(row: &[u8]) {
    if row.is_empty() {
        return;
    }
    // Bad: `as` hides that it casts `const` away, and a pointer from a shared borrow may never
    // be written through.
    let base = row.as_ptr() as *mut u8;
    // SAFETY: the row is not empty.
    unsafe {
        base.write(0);
    }
}
```

```rust
#[expect(unsafe_code, reason = "a write through the row's base")]
pub const fn clear_first(row: &mut [u8]) {
    if row.is_empty() {
        return;
    }
    let base = row.as_mut_ptr();
    // SAFETY: the row is not empty, and `&mut` makes its first square this call's to write.
    unsafe {
        base.write(0);
    }
}
```

Held by `clippy::ptr_as_ptr`, `ptr_cast_constness`, `borrow_as_ptr` and
`ref_as_ptr`, which refuse the `as` forms; a write through a shared borrow's
pointer, spelled with `cast_mut`, is held by review.

Under `strict`, `clippy::as_ptr_cast_mut` refuses `as_ptr() as *mut`, and
`clippy::as_conversions` every `as`.

## A Read Is Aligned for Its Type

A read or write through a `*const T` needs an address aligned for `T`, and bytes
from a buffer are aligned for nothing wider than a byte: a `u32` read from a row
of squares at any offset may be misaligned, which is undefined behaviour even
where the hardware allows it. Bytes read as a wider value go through
`from_le_bytes` on a copied chunk, or `read_unaligned`, which takes any address;
a pointer cast to a wider type is aligned first, or checked with `is_aligned`.

```rust,compile_fail
// fails: clippy::cast_ptr_alignment
/// The first four squares, as one word.
///
/// # Safety
/// `squares` holds at least four bytes.
#[expect(unsafe_code, reason = "four squares read as one word")]
#[must_use]
pub const unsafe fn word(squares: &[u8]) -> u32 {
    // Bad: a slice of bytes is aligned for a byte, not for a `u32`.
    // SAFETY: the caller promises four bytes.
    unsafe { squares.as_ptr().cast::<u32>().read() }
}
```

```rust
#[must_use]
pub fn word(squares: &[u8]) -> Option<u32> {
    squares.first_chunk::<4>().copied().map(u32::from_le_bytes)
}
```

Held by `clippy::cast_ptr_alignment`, which refuses a cast to a more strictly
aligned pointer unless `read_unaligned` or `write_unaligned` uses it, and by
review.

## A Reference Made from a Pointer Holds for All of Its Lifetime

A reference promises, for as long as it lives, that its pointer is non-null,
aligned and points at a valid, initialized value; a `&T` that nothing writes the
value outside an `UnsafeCell`, and a `&mut T` that nothing else reads or writes
it. So a safe function takes a reference's lifetime from a borrow its signature
shows, `&self` giving `&'_ T`; an unbounded `'a` appears only in an `unsafe fn`
whose `# Safety` says what lives for it, as `slice::from_raw_parts` does, and a
`&mut` comes from a `&mut`, from an owner, or from an `UnsafeCell` whose
exclusive access the module proves, as a lock's guard does: a `&mut` from
`&self` with no such proof lets two callers hold one each.

```rust,compile_fail
// fails: clippy::mut_from_ref
use core::cell::UnsafeCell;

#[derive(Debug)]
pub struct Board {
    squares: UnsafeCell<[u8; 9]>,
}

impl Board {
    // Bad: two calls give two `&mut` of the same squares.
    #[expect(unsafe_code, reason = "the squares, for a caller to change")]
    #[must_use]
    pub const fn squares(&self) -> &mut [u8; 9] {
        // SAFETY: the board is alive for as long as `&self`.
        unsafe { &mut *self.squares.get() }
    }
}
```

```rust
#[derive(Debug)]
pub struct Board {
    squares: [u8; 9],
}

impl Board {
    #[must_use]
    pub const fn squares_mut(&mut self) -> &mut [u8; 9] {
        &mut self.squares
    }
}
```

Held by `clippy::mut_from_ref`, which refuses a `&mut` returned from only shared
borrows, and by review for the rest.

## No `transmute`: Name the Conversion

`transmute` reinterprets bytes and checks only that the sizes agree: not byte
order, not that every pattern is a valid value of the target, not alignment, not
lifetimes. A named conversion says what it means and checks what it can:
`from_le_bytes` and `to_le_bytes` with the byte order stated, `f32::from_bits`,
`cast` between pointers, `ptr::from_ref`, a `TryFrom` or a `match` for an enum
from its discriminant, and a derive that proves a type's layout where the
workspace has one, as zerocopy's does. A `transmute` that remains names both
types, `transmute::<A, B>`, and its `// SAFETY:` proves every pattern valid for
the target, the alignment and the lifetimes.

```rust,compile_fail
// fails: unnecessary_transmutes
use core::mem;

#[expect(unsafe_code, reason = "four squares read as one word")]
#[must_use]
pub const fn pack(squares: [u8; 4]) -> u32 {
    // Bad: says nothing of byte order.
    // SAFETY: any four bytes are a `u32`.
    unsafe { mem::transmute::<[u8; 4], u32>(squares) }
}
```

```rust
#[must_use]
pub const fn pack(squares: [u8; 4]) -> u32 {
    u32::from_le_bytes(squares)
}
```

Held by `unnecessary_transmutes`, which refuses a `transmute` a safe function
does, and by clippy's `transmute_ptr_to_ref`, `transmute_ptr_to_ptr`,
`transmute_int_to_bool` and `missing_transmute_annotations`, each for the form
it names; the rest is held by review.

## A Type Read as Another Has a Layout That Says So

Rust lays out a struct or an enum as it chooses, and may choose differently for
two types with the same fields, so a pointer to one read as the other is sound
only where a `repr` fixes both layouts: `#[repr(transparent)]` for a newtype
read as its one field, `#[repr(C)]` for a struct whose fields are read in C's
order.

```rust
use core::slice;

// Bad: no `repr`, so nothing promises a `Glyph` is laid out as its `u8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph(u8);

#[expect(unsafe_code, reason = "a row of squares read as a row of glyphs")]
#[must_use]
pub const fn glyphs(squares: &[u8]) -> &[Glyph] {
    // SAFETY: a `Glyph` holds one `u8`.
    unsafe { slice::from_raw_parts(squares.as_ptr().cast::<Glyph>(), squares.len()) }
}
```

```rust
use core::slice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Glyph(u8);

#[expect(unsafe_code, reason = "a row of squares read as a row of glyphs")]
#[must_use]
pub const fn glyphs(squares: &[u8]) -> &[Glyph] {
    // SAFETY: `Glyph` is `repr(transparent)` over a `u8`, so the squares' pointer, length and
    // borrow are a valid `[Glyph]`'s.
    unsafe { slice::from_raw_parts(squares.as_ptr().cast::<Glyph>(), squares.len()) }
}
```

Held by review.

## Uninitialized Memory Is `MaybeUninit`, Read Only Once It Is Whole

Memory never written holds no value at all, not even an arbitrary one, so
reading it as any type is undefined behaviour, a `u8` included. `MaybeUninit<T>`
holds it until it is written; `assume_init` is called once every field is, and
not before. `mem::uninitialized` is never used; `mem::zeroed` only for a type
that all-zero bytes are a valid value of: integers, floats, raw pointers,
`Option<&T>`, not a reference, a `NonNull`, a `NonZero` or a function pointer.
An array of slots is `[const { MaybeUninit::uninit() }; N]`, a `Vec`'s spare
room is `spare_capacity_mut`, counted with `set_len` once written, and where
`array::from_fn` or `extend` builds the value, neither is needed.

```rust,compile_fail
// fails: clippy::uninit_assumed_init
use core::mem::MaybeUninit;

#[expect(unsafe_code, reason = "a row filled after it is made")]
#[must_use]
pub fn row() -> [u8; 8] {
    // Bad: eight bytes never written, read as eight `u8`s.
    // SAFETY: a `u8` has no invalid bit pattern.
    let mut row: [u8; 8] = unsafe { MaybeUninit::uninit().assume_init() };
    for (tile, square) in (0_u8..).zip(&mut row) {
        *square = tile;
    }
    row
}
```

```rust
use core::mem::MaybeUninit;

#[derive(Debug)]
pub struct Tile {
    pub label: String,
}

#[must_use]
pub fn row(label: &str) -> [Tile; 8] {
    let mut row = [const { MaybeUninit::<Tile>::uninit() }; 8];
    for slot in &mut row {
        slot.write(Tile { label: label.to_owned() });
    }
    // SAFETY: the loop wrote every slot; had it panicked, the slots would have leaked what they
    // held, since a `MaybeUninit` drops nothing.
    #[expect(unsafe_code, reason = "a row whose every slot the loop above wrote")]
    unsafe { MaybeUninit::<[Tile; 8]>::from(row).assume_init() }
}
```

Held by `clippy::uninit_assumed_init` and `clippy::uninit_vec`, and by rustc's
`invalid_value`, which refuses `mem::zeroed` or `mem::uninitialized` of a type
it can see they are wrong for; the rest is held by review.

## Pin with `pin!` or `Box::pin`; an Unchecked Pin Proves the Value Never Moves

A pinned value promises that it stays at its address until it is dropped, which
a type that holds its own address, or hands it out, rests on. `pin!` and
`Box::pin` keep that promise by construction. `Pin::new_unchecked`,
`get_unchecked_mut` and a hand-written projection are unsafe because the code
keeps it instead: its `// SAFETY:` proves the value is never moved again, its
`Drop` included, and a field the projection pins is named, as structurally
pinned, in an `// INVARIANT:`. A type that must not move once pinned holds a
`PhantomPinned`, so it is not `Unpin`.

```rust
use core::marker::PhantomPinned;
use core::pin::Pin;

#[derive(Debug, Default)]
pub struct Cursor {
    index: usize,
    _pinned: PhantomPinned,
}

impl Cursor {
    #[expect(unsafe_code, reason = "a field of a pinned cursor changed in place")]
    pub const fn advance(self: Pin<&mut Self>) {
        // SAFETY: `index` is not structurally pinned, and nothing here moves the cursor.
        let cursor = unsafe { self.get_unchecked_mut() };
        cursor.index = cursor.index.wrapping_add(1);
    }
}

#[expect(unsafe_code, reason = "a cursor pinned in place")]
#[must_use]
pub fn walk() -> usize {
    let mut cursor = Cursor::default();
    // Bad: the pin lasts one call, and `cursor` moves on the next line.
    // SAFETY: the cursor stays put while it is pinned.
    unsafe { Pin::new_unchecked(&mut cursor) }.advance();
    let moved_cursor = cursor;
    moved_cursor.index
}
```

```rust
use core::marker::PhantomPinned;
use core::pin::{Pin, pin};

#[derive(Debug, Default)]
pub struct Cursor {
    index: usize,
    _pinned: PhantomPinned,
}

impl Cursor {
    #[expect(unsafe_code, reason = "a field of a pinned cursor changed in place")]
    pub const fn advance(self: Pin<&mut Self>) {
        // SAFETY: `index` is not structurally pinned, and nothing here moves the cursor.
        let cursor = unsafe { self.get_unchecked_mut() };
        cursor.index = cursor.index.wrapping_add(1);
    }
}

#[must_use]
pub fn walk() -> usize {
    let mut cursor = pin!(Cursor::default());
    cursor.as_mut().advance();
    cursor.index
}
```

Held by review.

## An `extern` Block Is `unsafe extern`, and Each Call Proves What C Asks

The compiler cannot check a foreign declaration, so an `extern` block is `unsafe
extern`, which edition 2024 requires, and each item in it is `unsafe` to call
unless marked `safe fn`, a promise that no argument value can make the call
unsound: `getpid` qualifies, and `abs` does not, since `abs(INT_MIN)` is
undefined in C. A function that takes a pointer stays unsafe, and a safe wrapper
passes only pointers it derived from references it holds for the call: a
`CString` is bound to a name before its pointer is taken, since a temporary's
pointer dangles at the end of its statement. An exported symbol is
`#[unsafe(no_mangle)]` or `#[unsafe(export_name = "…")]`, a promise that no
other symbol of the program has its name, and a type that crosses is
`#[repr(C)]` and spelled with `core::ffi`'s types.

```rust,compile_fail
// fails: dangling_pointers_from_temporaries
extern crate alloc;

use alloc::ffi::{CString, NulError};
use core::ffi::c_char;

#[expect(unsafe_code, reason = "the C library's `strlen`, which reads up to a NUL")]
unsafe extern "C" {
    fn strlen(label: *const c_char) -> usize;
}

pub fn label_len(label: &str) -> Result<usize, NulError> {
    // Bad: the `CString` is dropped at the end of this line, and the pointer dangles.
    let label = CString::new(label)?.as_ptr();
    // SAFETY: `label` is NUL-terminated.
    #[expect(unsafe_code, reason = "a C call over a label")]
    Ok(unsafe { strlen(label) })
}
```

```rust
extern crate alloc;

use alloc::ffi::{CString, NulError};
use core::ffi::c_char;

#[expect(unsafe_code, reason = "the C library's `strlen`, which reads up to a NUL")]
unsafe extern "C" {
    fn strlen(label: *const c_char) -> usize;
}

pub fn label_len(label: &str) -> Result<usize, NulError> {
    let label = CString::new(label)?;
    // SAFETY: a `CString` is NUL-terminated, and `label` lives past the call, which only reads
    // it.
    #[expect(unsafe_code, reason = "a C call over a label this function holds")]
    Ok(unsafe { strlen(label.as_ptr()) })
}
```

Held by `dangling_pointers_from_temporaries`, which refuses a pointer taken from
a temporary that is dropped at once, by the compiler, which refuses a bare
`extern` block, `#[no_mangle]` or `#[export_name]` in edition 2024, and by
review for a `safe fn` and for what each call proves.

## A C Enum Arrives as an Integer, and `TryFrom` Checks It

C lets an enum hold any value of its integer type, and a Rust enum holding a
discriminant it has no variant for is undefined behaviour, however it got there.
So a foreign function that returns a C enum, or a field that holds one, is
declared with the integer, and the value becomes a Rust enum through `TryFrom`,
which refuses what has no variant.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Blank = 0,
    Wall = 1,
}

#[expect(unsafe_code, reason = "the tile library's C interface")]
unsafe extern "C" {
    // Bad: a library that returns 7 hands Rust a `Kind` with no variant.
    safe fn tiles_kind(index: u32) -> Kind;
}

#[must_use]
pub fn kind(index: u32) -> Kind {
    tiles_kind(index)
}
```

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    Blank = 0,
    Wall = 1,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[error("kind error: {discriminant} names no kind of tile")]
pub struct KindError {
    pub discriminant: u8,
}

impl TryFrom<u8> for Kind {
    type Error = KindError;

    fn try_from(discriminant: u8) -> Result<Self, KindError> {
        match discriminant {
            0 => Ok(Self::Blank),
            1 => Ok(Self::Wall),
            _ => Err(KindError { discriminant }),
        }
    }
}

#[expect(unsafe_code, reason = "the tile library's C interface")]
unsafe extern "C" {
    // Safe for any square: it reads the library's own table, and writes nothing.
    safe fn tiles_kind(index: u32) -> u8;
}

pub fn kind(index: u32) -> Result<Kind, KindError> {
    Kind::try_from(tiles_kind(index))
}
```

Held by review.

## A Foreign Slice Is Empty Before It Is Null

A foreign interface passes a slice as a pointer and a length, and an empty one
may come with a null pointer. `slice::from_raw_parts` takes no null pointer,
even for no elements, so a length of zero becomes `&[]` before any pointer is
read.

```rust
use core::slice;

/// The squares C passed.
///
/// # Safety
/// `squares` points at `length` initialized squares that live and stay unwritten for `'a`.
#[expect(unsafe_code, reason = "a slice from the pointer and length C passed")]
#[must_use]
pub const unsafe fn squares<'a>(squares: *const u8, length: usize) -> &'a [u8] {
    // Bad: C may pass a null pointer with a length of zero.
    // SAFETY: the caller promises `length` squares at `squares`.
    unsafe { slice::from_raw_parts(squares, length) }
}
```

```rust
use core::slice;

/// The squares C passed.
///
/// # Safety
/// Where `length` is not zero, `squares` points at `length` initialized squares that live and
/// stay unwritten for `'a`.
#[expect(unsafe_code, reason = "a slice from the pointer and length C passed")]
#[must_use]
pub const unsafe fn squares<'a>(squares: *const u8, length: usize) -> &'a [u8] {
    if length == 0 {
        return &[];
    }
    // SAFETY: `length` is not zero, so the caller promises `length` squares at `squares`.
    unsafe { slice::from_raw_parts(squares, length) }
}
```

Held by review.

## A Callback Handed to C Catches Its Panics

A panic cannot unwind out of an `extern "C"` function: it aborts the process
there. So a Rust function C calls back runs its body under `catch_unwind` and
turns a panic into the error code C reads; `extern "C-unwind"` is for a boundary
where both sides unwind. The other way, a C++ exception thrown into Rust through
an `extern "C"` import is undefined behaviour, so a foreign function that may
throw is declared `"C-unwind"`, or wrapped on its own side.

```rust
fn place(index: u32) {
    assert!(index < 81, "a square of the nine-by-nine board");
}

// Bad: a panic in `place` aborts the program C is running.
#[must_use]
pub extern "C" fn tiles_on_place(index: u32) -> i32 {
    place(index);
    0
}
```

```rust
use std::panic::catch_unwind;

fn place(index: u32) {
    assert!(index < 81, "a square of the nine-by-nine board");
}

#[must_use]
pub extern "C" fn tiles_on_place(index: u32) -> i32 {
    match catch_unwind(|| place(index)) {
        Ok(()) => 0,
        Err(_panic) => -1,
    }
}
```

Held by review.
