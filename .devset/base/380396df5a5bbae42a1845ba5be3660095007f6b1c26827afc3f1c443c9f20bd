# Exemplars: `mem-init`, `mem-region`, `mem-allocators`, Annotated

Read this to see a rule on the page, before writing docs in a form this skill's
other references only describe. Excerpts from three memory crates, each with the
rule it shows. A new crate's docs must sit comfortably next to these; when they
would not, the new docs are wrong. `mem-init`'s crate page is the problem-first
form, `mem-allocators`' the shape-first one.

The excerpts are adapted from the crates, not quoted: the docs' words and form
are theirs, and the names are written in whole words, `length`, `destination`,
`required` and `available` where the crates write `len`, `dst`, `need` and
`have`, with each error variant named for its whole condition.

Contents: 1 Crate pages · 2 Modules · 3 Types and fields · 4 Traits · 5
Functions · 6 Errors · 7 Private items · 8 `// SAFETY:` and `// ORDERING:` · 9
`#[expect]` and messages · 10 Tests · 11 Fixtures and headers · 12 Manifest · 13
Re-exports and omissions

## 1 Crate Pages

````text
//! In-place initialization: build a value where it will live, never moved there.
//!
//! Filling a location usually builds the value first, then moves it in:
//!
//! ```
//! // `[0; 4096]` is built on the stack, then copied into the heap allocation:
//! let boxed = Box::new([0_u8; 4096]);
//! # let _ = boxed;
//! ```
//!
//! Each value is constructed elsewhere and copied into place. Rust guarantees no copy elision
//! ([rust#125632]), so for a large `T` that copy can outlive LLVM optimization: real memory
//! traffic, and a stack frame the size of `T` that a big enough value overflows. …
//!
//! A facade over [pin-init]: it re-exports the whole surface, so nothing downstream reaches into
//! it directly. It swaps `init!` / `pin_init!` for wrappers that deny `mem::forget` …, and
//! replaces `pin-init`'s `Zeroable` with [`zeroed`] over `zerocopy::FromZeros`: a marker wrapped
//! around one method is one name too many when a mature one already states it.
````

- `Topic: pitch.`; the pitch is the manifest `description` verbatim.
- The problem is *shown* before it is named; the comment in the example names
  the cost.
- The external fact is cited with a reference link, not explained.
- Each departure from the obvious design is a fact plus its consequence. No
  `powerful`, no `ergonomic`: the reason *is* the pitch.

````rust
//! Allocators over a byte range you own.
//!
//! A [`Region`](mem_region::Region) is a span of bytes to build inside; an allocator carves blocks
//! from one and, where it can, takes them back:
//!
//! ```text
//!     bytes you own ── a buffer, or a mapped segment
//!     │
//!     ▼
//!     Region        ── a span of bytes, yours to write
//!     ├──▶ Arena    ── bump a cursor, never frees
//!     └──▶ Heap     ── segregated fit, coalescing
//! ```
//!
//! # Types
//!
//! - **Allocators.** `Copy`, one word wide, and each an [`Allocator`](core::alloc::Allocator), so a
//!   single process's standard containers serve from them:
//!   - [`Arena`] bumps a cursor and never frees, carving from one region.
//!   - [`Heap`] is segregated-fit and coalescing under one coarse lock, its free-lists in the
//!     region; [`Stats`] reports its occupancy.
//! - **The shape.** [`Rooted`], every block reachable from one base pointer, so a copy in another
//!   process reaches the same blocks against its own mapping.
//! - **Errors.** [`ReserveError`]: how a bounded allocator reports no room; nothing here aborts.
````

- The shape diagram: one relation, annotations in an aligned `──` column.
- `# Types` groups by role with a bold lead; three bullets cover eight exports.
  Each clause says what the type *buys*, not what it is made of.

````text
//! ## Building Many
//! [`run::each`] builds each of `n` elements from an initializer, in its slot; [`run::copied`] and
//! [`run::cloned`] are the bulk slice sources.
//! ```
//! …
//! ```
//!
//! `field: value` evaluates the value and moves it in; `field <- init` builds it in place. Prefer
//! `<-` for a large field, so nothing is copied.
````

- A task's heading, a gerund `##` under `# Examples`, in title case; blank line
  before, none after; the one-sentence lead links the entry points; guidance is
  `Prefer …, so …` with the cost stated.

````text
//! # What It Compiles To
//! `raw_try_init(destination, init!(Order { id, near <- init!(Leg { price, quantity }), tag: [0; 32] }))`
//! from the example above, release build, aarch64:
//!
//! ```text
//! movi v0.2d, #0      ; zero the tag
//! stp  x1, x2, [x0]   ; id, near.price -> destination, destination+8
//! …
//! ```
//!
//! Every field is stored straight through `destination` (`x0`): no stack frame, no `memcpy`, …
````

- The exact call, the build, the real listing, one paragraph on what it proves.
  This section exists only because the listing was produced; it is never written
  from expectation.

## 2 Modules

```rust
//! A bump cursor over one region.
```

```rust
//! `Arena`'s [`Allocator`] impl: it carves, and never takes a cut back.
```

```rust
//! An address held as an integer, with no right to reach it.
```

````text
//! Building `n` values where they will live: a [`RunInit`] into a caller's buffer.
//!
//! A run reports its [`len`](RunInit::len) before it writes any element, so a container can make
//! room first. A failure part-way drops exactly the prefix it wrote. [`each`] is the general
//! source, an [`Init`] per element built in its slot; [`copied`] and [`cloned`] are the bulk slice
//! sources.
//!
//! ```
//! …
//! ```
````

- Thirteen of the exemplars' twenty-one module docs are one line. The long form
  (contract paragraph, one example) is for a module that is a surface of its
  own.
- The contract paragraph: the guarantee (`len` first), the failure state (prefix
  dropped), the general entry point against the bulk ones, all linked. Nothing
  else.

## 3 Types and Fields

````text
/// The exclusive right to initialize a span of raw bytes, and the provenance to reach it.
///
/// - **Move-only.** Laying a structure into a region consumes it, so a second writer over the same
///   bytes cannot be minted in safe code. [`cut`](Self::cut) and [`split_at`](Self::split_at)
///   divide one, and both spend it.
/// - **Borrows the bytes it covers.** A structure built inside a region cannot outlive them, so
///   unmapping the backing while a handle is live does not compile.
/// - Neither [`Send`] nor [`Sync`]: what licenses sharing across threads or processes is the
///   resident structure's [`Send`] + [`Sync`] and the lock behind it, never [`Region`]. …
///
/// Describe a range without claiming the right to write it with [`span`](Self::span).
///
/// # Examples
/// ```
/// …
/// ```
pub struct Region<S: PtrStore> {
    /// First byte of the span.
    base: NonNull<u8>,
    /// Bytes the span holds.
    length: usize,
    /// Where the bytes live, and, for [`Local`](crate::Local), how long they last.
    storage: PhantomData<S>,
}
````

- Role, then the properties as bold-lead bullets, each with its consequence; the
  safe door linked; an example on the type.
- Field docs are noun phrases, one line, private ones included; types are never
  restated.

```rust
use core::num::NonZeroU64;

/// A byte distance from the process reservation's first byte: what a [`Shared`] region stores in
/// place of an address, so the same word names the same byte in every peer.
///
/// An **alias**, not a newtype: the name is worth having, a second type is not. Nothing forces the
/// choice (a newtype states `AtomNiche` for itself, as `Word<C>` does), so what decides it is that
/// every caller here already means this exact word.
pub type Offset = NonZeroU64;
```

- A design decision stated as fact + what decides it; bold on the load-bearing
  word.

```text
/// The recorded mapping's first byte, or the sentinel.
///
/// **Plain, not atomic, and that is the point**: an atomic load is not CSE-able, so a resolve that
/// takes two hops loads the base twice, 7 instructions against 6. Written once before any peer
/// resolves, which is the whole of what [`create`]'s `unsafe` asks for.
static ROOT: Root = Root(SyncUnsafeCell::new(UNSET));
```

- A static's doc: what it holds, then why it is shaped this way, with the
  measured cost.

```text
/// Drops the prefix a run wrote, unless the run completes.
struct Guard<T> { … }

/// What [`each`] builds.
#[derive(Debug, Clone, Copy)]
pub struct Each<T, E, F> {
    /// How many slots to write.
    length: usize,
    /// What initializes each one.
    initializer: F,
    /// Fixes the element and error types by owning them.
    element: PhantomData<(T, E)>,
}
```

- An active object takes a verb phrase; an adapter type says only what builds
  it.

## 4 Traits

```text
/// How to build a run of `T` at a destination that already exists.
///
/// # Safety
/// `Ok` means [`len`](Self::len) `T`s are initialized at `destination`; `Err` means none are, and
/// those bytes may only be released, never dropped.
pub unsafe trait RunInit<T, E = Infallible>: Sized {
    /// How many it writes, known before it writes any.
    fn len(&self) -> usize;

    /// Whether it writes none.
    fn is_empty(&self) -> bool { … }

    /// Writes them at `destination`. Prefer [`raw_run_init`] / [`raw_try_run_init`] to calling
    /// this.
    ///
    /// # Safety
    /// `destination` is aligned, writable, uninitialized for [`len`](Self::len) `T`s, and
    /// unaliased.
    ///
    /// # Errors
    /// Whatever the source reports, having dropped the prefix it had written.
    unsafe fn __init(self, destination: *mut T) -> Result<(), E>;
}
```

- `How to …`; the `unsafe trait`'s `# Safety` is the impl's promise in
  `Ok`/`Err` form.
- Getters name the value. A method users should not call says so and links the
  front door.
- `# Safety` is a comma list of preconditions; `# Errors` names the
  destination's state.

```text
/// An allocator laid into a region once and found there by every peer afterwards.
///
/// The doors [`Rooted`] cannot carry, because a forwarding `&A` implements that and can lay
/// nothing. One trait rather than an inherent pair per store: a trait method takes `Self` from
/// context, where `Heap::open(region)` over two inherent impls is **E0034**, resolvable only by
/// spelling `<Heap<Shared>>::open`.
pub trait Root: Rooted + Sized {
    /// Itself, branded with the borrow taken while opening: a [`Local`](mem_region::Local) region
    /// lends its bytes for `'a`, a [`Shared`](mem_region::Shared) one has none to lend.
    type Bound<'a>: Root where Self: 'a;

    /// Lays one into `region`, spending the right to write it.
    ///
    /// # Errors
    /// [`RegionError`], the region is too small or misaligned for one.
    fn create(region: Region<Self::Storage>) -> Result<Self, RegionError>;
}
```

- Why a trait and not inherent methods, with the compiler fact that decides it.
- An associated type's doc says what it is in each impl.

```text
/// Bytes returned to this allocator become available again.
///
/// # Safety
/// [`deallocate`](Allocator::deallocate) really reclaims: a block freed here may be served again.
///
/// A **container policy** marker, gating exactly `shrink_to_fit`, the one verb whose contract *is*
/// reclamation. It must never gate `push` or `reserve`: …
pub unsafe trait Reclaiming: Allocator {}
```

- A marker trait: the property, the impl's promise, then where it may and may
  not gate.

## 5 Functions

````text
/// Writes `init`'s value at `destination`.
///
/// # Safety
/// `destination` is aligned, writable, uninitialized for one `T`, and unaliased; and, unless `I`
/// is [`Init`](pin_init::Init), `destination` stays put until the `T` is dropped.
///
/// # Errors
/// Whatever the initializer reports, having written nothing.
///
/// # Examples
/// ```
/// use core::mem::MaybeUninit;
///
/// use mem_init::{init, raw_try_init};
///
/// #[derive(Debug, PartialEq)]
/// struct Leg {
///     price: i64,
///     quantity: i64,
/// }
///
/// let mut slot = MaybeUninit::<Leg>::uninit();
/// // SAFETY: a fresh slot for one `Leg`, named by nothing else.
/// let Ok(()) = (unsafe { raw_try_init(slot.as_mut_ptr(), init!(Leg { price: 100, quantity: 1 })) });
/// // SAFETY: the initializer reported success.
/// assert_eq!(unsafe { slot.assume_init() }, Leg { price: 100, quantity: 1 });
/// ```
pub unsafe fn raw_try_init<…>(destination: *mut T, init: I) -> Result<(), E> {
    // SAFETY: the caller upholds `__init`'s contract, including pinning unless `I` cancels it.
    unsafe { PinInit::__init(init, destination) }
}

/// Writes an infallible `init`'s value at `destination`.
///
/// # Safety
/// As [`raw_try_init`].
````

- Verb first; the conditional precondition after a semicolon; the example's
  `use` blocks, domain type, `// SAFETY:` lines, and closing `assert_eq!`; the
  twin delegates with ``As [`…`].``

````text
/// The base, once `layout` is known to fit it.
///
/// The one check every structure runs before writing a header; the two ways to get it wrong
/// are named here, not at each site.
///
/// # Errors
/// - [`RegionError::TooSmall`], the region is shorter than `layout`.
/// - [`RegionError::Misaligned`], the base does not meet `layout`'s alignment.
///
/// # Examples
/// ```
/// let mut backing = [0_u8; 64];
/// let region = Region::from_slice(&mut backing);
/// assert!(region.fit(Layout::new::<u64>()).is_ok(), "8 bytes fit, 8-aligned");
/// assert!(region.fit(Layout::new::<[u64; 16]>()).is_err(), "128 do not");
/// ```
pub fn fit(&self, layout: Layout) -> Result<NonNull<u8>, RegionError> {
    if self.length < layout.size() {
        return Err(RegionError::TooSmall { required: layout.size(), available: self.length });
    }
    // A mask, not a remainder: `align()` is a power of two, but only a divide would prove it.
    if self.base.addr().get() & !layout.alignment().mask() != 0 { … }
    Ok(self.base)
}
````

- The summary names the value; the body says why this fn exists (one check,
  named once); the list-form `# Errors`; assertion messages that read as a pair.

````text
/// The first `layout`-shaped sub-region, or `None` when the range cannot hold one.
///
/// Aligns the address, so a weakly aligned base still yields one at the alignment asked for.
/// Consumes, as every division does: the right to write passes to the returned part; the rest
/// is given up.
///
/// # Examples
/// ```
/// // A byte array is 1-aligned, so over-allocate 63 (`alignment - 1`) bytes of slack; the pad
/// // to 64 fits wherever the buffer fell.
/// let mut backing = [0_u8; 256 + 63];
/// let region = Region::from_slice(&mut backing)
///     .cut(Layout::from_size_align(256, 64)?)
///     .expect("the slack absorbs the pad");
/// assert_eq!(region.base().addr().get() % 64, 0);
/// # Ok::<(), core::alloc::LayoutError>(())
/// ```
pub fn cut(self, layout: Layout) -> Option<Self> { … }
````

- `Option` form: value, then ``, or `None` when …``. Contract stated as
  consequence (`Consumes, as every division does`). `?` closed by the trailing
  `# Ok::<(), E>(())`; `expect` with a fragment.

```text
/// The address `bytes` further on, wrapping: how a caller walks a range it has already
/// bounded.
pub const fn wrapping_add(self, bytes: usize) -> Self { … }

/// Where `pointer` points, dropping its provenance.
pub fn from_pointer<T: ?Sized>(pointer: NonNull<T>) -> Self { … }

/// Takes `bytes` as the span to build inside: the safe door, enough without a mapping.
///
/// The `&'a mut [u8]` proves both halves of the raw door's contract: the bytes are this
/// region's alone and outlive it.
pub const fn from_slice(bytes: &'a mut [u8]) -> Self { … }
```

- One-line getters and constructors; a summary may wrap once when its second
  clause says *when* to use it; a constructor's body says what the argument type
  proves.

## 6 Errors

```text
//! Why a placement did not work out: for a layout that will not fit, or for the mapping a caller
//! resolves its shared addresses against.

/// Why a [`Layout`](core::alloc::Layout) or a [`Span`] does not fit a [`Region`](crate::Region).
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RegionError {
    /// The region is shorter than the layout.
    #[error("region error: needs {required} bytes, region holds {available}")]
    TooSmall {
        /// Bytes the layout needs.
        required: usize,
        /// Bytes the region holds.
        available: usize,
    },
    /// The region's base does not meet the layout's alignment.
    #[error("region error: base {base:#x} does not meet the {alignment}-byte alignment")]
    Misaligned { … },
}
```

```text
//! Why an allocator refused.
//!
//! Nothing aborts: a bounded allocator running out is routine, so every verb needing bytes returns
//! [`ReserveError`].

/// Why an allocator could not make room.
pub enum ReserveError {
    /// No run of free bytes fits this layout: free something and retry, or ask for less.
    #[error("reserve error: the allocator refused {layout:?}")]
    NoFreeRunFits {
        /// What it refused.
        layout: Layout,
    },
}
```

- `Why …` on the enum and the module; the variant names the whole condition, and
  its doc says it, with what to do about it after a colon; the fields are whole
  words, `required` and `available`; the message leads with the type's words and
  inlines the fields.

## 7 Private Items

```text
/// Counts one more written element.
///
/// # Safety
/// That element is initialized and nothing else drops it.
const unsafe fn grow(&mut self) { … }

/// Hands them all to the caller.
const fn disarm(&mut self) { … }

/// The record both doors proved is there.
///
/// Never a `&Body`: that would retag the tail, which a peer holding a cut is writing.
fn record(self) -> &'static Header { … }
```

- Private items are documented as fully as public ones when they carry a
  contract; a private `unsafe fn` gets `# Safety`. No example, since no user
  calls it. A private fn's body says the one thing a maintainer must not change.

## 8 `// SAFETY:` and `// ORDERING:`

```text
// SAFETY: the guard counts what landed, so a refusal at `index` drops exactly the slots below it.
unsafe impl<…> RunInit<T, E> for Each<T, E, F> {
    unsafe fn __init(mut self, destination: *mut T) -> Result<(), E> {
        …
            // SAFETY: the `index`th of the `length` slots the caller promised, still fresh.
            let slot = unsafe { destination.add(index) };
            // Built where it lands, so nothing is moved in; on `Err` it wrote nothing.
            //
            // SAFETY: a fresh unaliased slot for one `T`, and `Init` cancels the pinning duty.
            unsafe { PinInit::__init(init, slot) }?;
            // SAFETY: the initializer reported success, so the guard may count it.
            unsafe { guard.grow() }
        …
    }
}
```

```text
loop {
    // ORDERING: Relaxed throughout. The word hands out disjoint ranges and publishes
    // nothing; whatever a caller lays in the bytes it took, it releases itself.
    let start = record.next.load(Relaxed).0;
    …
}
```

```text
// SAFETY: a slice's base is never null.
let base = unsafe { NonNull::new_unchecked(bytes.as_mut_ptr()) };

// SAFETY: `mid <= length`, so the tail's base stays inside the region.
let after = unsafe { self.base.byte_add(mid) };
// SAFETY: the head lies inside the region, backing outlives `'a`, and `self` is consumed,
// so nothing else covers those bytes.
let head = unsafe { Self::from_raw_parts(self.base, mid) };
// SAFETY: as the head, and the tail starts where the head ends, so the two are disjoint.
let tail = unsafe { Self::from_raw_parts(after, self.length.wrapping_sub(mid)) };
```

- One `unsafe` op per block, its `// SAFETY:` naming the fact for each
  precondition, in as many lines as that takes: the head's runs to two.
  Non-safety rationale sits above, separated by a bare `//`. Chaining (`as the
  head, and …`) when the fact is one up.
- `// ORDERING:` states the ordering and what it does or does not publish.

## 9 `#[expect]` and Messages

```text
#![expect(
    unsafe_code,
    reason = "re-deriving a pointer from an address and writing raw byte spans is this crate's whole purpose"
)]

#[expect(clippy::mem_forget, reason = "pin-init disarms its field guards this way")]

#[expect(
    clippy::indexing_slicing,
    reason = "a `Bucket` is below `NUM_BUCKETS`, this array's length: `of` clamps to `TOP`, \
              `up` stops there, and the occupancy scan masks to `Bucket::MAP`"
)]
```

```text
debug_assert!(chunk.word().is_inuse(), "a block given back twice, or never served");
```

- Reasons are causes, lowercase, no period. A `debug_assert!`'s message names
  the violation, as a fact about the input.

## 10 Tests

```text
/// Slots the run writes into, and the one that refuses.
const SLOTS: usize = 32;
/// See [`SLOTS`].
const REFUSES: usize = 4;

/// Runs `source` over `N` fresh slots and hands back what it built.
fn built<…>(source: R) -> Result<[T; N], E> { … }

#[test]
fn an_each_that_gives_up_drops_the_prefix_exactly_once() {
    …
        // A `Result` is an initializer, so a refusing element is just an `Err`. `T`/`E` are
        // annotated here because a bare `Result` is ambiguous between value- and fallible-init;
        // at a real call site the container fixes `T`.
    …
    assert_eq!(counted, (refused, refused), "the first four, exactly once");
}
```

```text
#[test]
fn an_offset_is_only_measured_forwards() {
    let base = VirtAddr::new(0x1000);
    assert_eq!(base.offset_to(base), Some(0), "to itself");
    assert_eq!(base.offset_to(VirtAddr::new(0x1040)), Some(0x40), "and forwards");
    assert_eq!(base.offset_to(VirtAddr::new(0x0FFF)), None, "never backwards");
}

// The bug this closes aligned the offset, so it was correct only when the base already was.
#[test]
fn a_cut_aligns_the_address_not_the_offset() { … }

/// The links open through a [`Free`] and nowhere else; nothing past the two constructions
/// is guarded, so an accessor taking its obligation back stops compiling here.
#[deny(unused_unsafe)]
#[test]
fn only_a_free_chunk_opens_its_links() { … }
```

- Names are the property, subject first. Messages continue one sentence across
  the assertions. A `//` above a test says what it pins when the name cannot; a
  `///` when the test carries a compile-time property.

## 11 Fixtures and Headers

```rust
//! `init!` writes fields through raw pointers, so a field it does not name would be left
//! uninitialized. The struct-literal check it emits is what makes forgetting one a compile error.
```

```rust
//! A region borrows the bytes it covers, so a structure built inside one cannot escape them.
```

```text
//! The misuses a region refuses, each a fixture that must not compile: a region spent twice,
//! and a structure built inside one escaping the bytes it borrows. Regenerate a message with
//! `TRYBUILD=overwrite cargo nextest run -p mem-region --test trybuild`.

// A loom model drives no compiler.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    #[test]
    fn each_misuse_fails_to_compile() { … }
}
```

```text
//! What the arena's cursor costs: the exchange a shared handle pays, against the register bump an
//! exclusive one could.
//!
//! [`Arena::carve`](mem_allocators::Arena::carve) takes `&self`, so every range costs a
//! `compare_exchange_weak`. … Both arms below run the same [`Span::cut`] arithmetic over a range
//! wide enough never to exhaust, so the gap between them is the exchange and nothing else.

/// The isolated core the measuring thread runs on.
const CORE: u32 = 6;
```

```text
//! What the vocabulary is for: one buffer becomes a partition, and the type system holds the line.
//!
//! Run with `cargo run -p mem-region --example region-tour`.

#![expect(clippy::print_stdout, reason = "a demo binary reports its result on stdout")]
```

```text
//! Cross-process proof for [`Heap`]: two processes share one heap's free-lists and lock, and each
//! stamps its own identity into that lock.
//!
//! **One crossing datum, and it is the segment's name.** …
```

- A fixture states the unsoundness it prevents, nothing about the harness; the
  harness's test is named for what it pins, and only it carries a `cfg`. A bench
  states what the gap between arms measures. An example says what it shows and
  how to run it. An integration test states its proof and the one datum that
  crosses.

## 12 Manifest

```toml
description = "place structures in raw bytes and address them by location, not by pointer."

[dependencies]
# external
pin-init = { workspace = true }
zerocopy = { workspace = true }
# internal
mem-assert = { workspace = true }
mem-region = { workspace = true }
```

- The pitch clause, lowercased, with its period. The group markers, external
  first, are the only comments the manifest carries: why `pin-init` is a
  dependency, though no source names it, is said in the crate docs where `init!`
  is, not beside the entry.

## 13 Re-Exports and Omissions

```text
/// What this crate takes from `pin-init` unchanged, globbed so an explicit import can shadow one
/// of its names.
mod borrowed {
    // `Zeroable`, `MaybeZeroable`, `ZeroableOption`, `init_zeroed` and `zeroed` are
    // deliberately absent: `pin-init`'s marker is one trait around one default method, and
    // [`zeroed`](super::zeroed) is that method over `zerocopy::FromZeros`, so one marker
    // states "a zeroed place is a value" for the whole codebase rather than two.
    pub use pin_init::{…};
}

// No `Reclaiming`: a bump cursor never steps back, so a block returned here is not served again.
```

- The module line explains the glob's purpose. A deliberate omission or a
  deliberately missing impl is named with its reason in a `//` comment, so the
  next maintainer does not "fix" it.
