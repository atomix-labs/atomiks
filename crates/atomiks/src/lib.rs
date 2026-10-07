//! Typed atomics for any value that fits one atomic word.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out. The integer, `bool` and pointer
//! atomics are its aliases (`AtomicU64 = Atomic<u64>`), and any value implementing [`Atom`] is one
//! more: a `NonZero`, a `char`, a float, a ranged integer, an `Option` that spends a spare repr on
//! `None`, and a struct or an enum that derives it.
//!
//! # Types
//!
//! - **The atomic.** [`Atomic<T>`](Atomic), with an alias per primitive, such as [`AtomicU64`]; and
//!   [`AtomicField`], a field of an atomic packed struct, as a place of its own, which one
//!   instruction on the whole word changes alone. [`Atomic::fields`] lends one per field through
//!   the [`ProjectFields`] the derive implements, whose [`Fields`](ProjectFields::Fields) is the
//!   struct of places it writes. A place's type names its [`FieldPath`]: a [`Field`], a [`Then`] of
//!   a field's field, or [`Whole`], which [`Join`] extends; [`TopField`] marks the one that adds.
//! - **Values.** [`Atom`] encodes a value as its repr and back, its [`ReprRange`] says which reprs
//!   it takes and its [`validity`] which decode, and [`PtrAtom`] is a value stored as a pointer;
//!   [`AtomAdd`], [`AtomOrd`] and [`AtomBitwise`] add the read-modify-writes that mean something
//!   for it, and [`FieldBitwise`] and [`FieldAdd`] those of a field of it. With the `derive`
//!   feature, each derives: `Atom` for a struct or an enum, a pointer word or a pointer enum among
//!   them, which keeps small fields as tags in its pointer's low bits and the pointer's provenance,
//!   and each capability for a newtype whose field has it. [`RangedU64<MIN, MAX>`](RangedU64) and
//!   its siblings, `RangedU8` to `RangedIsize`, hold an integer from `MIN` to `MAX`, and
//!   [`RangeError`] and [`ParseRangeError`] say why one refused an integer or a text.
//! - **Orderings.** The [`ordering`] types, each accepted only where it means something, and the
//!   [`fence`](fn@fence) and [`compiler_fence`] they order.
//! - **Primitives.** [`Primitive`], [`ExactBits`] where the bits are the whole value, and what the
//!   target runs without a loop: [`Load`], [`Store`], [`Swap`], [`FetchBitwise`], [`MinMax`], and,
//!   for a field's container, [`FetchAdd`], [`MaskBitwise`], [`BitTest`].
//! - **Building blocks.** The loom-shaped [`cell`], the spin [`hint`], and `model` under loom.
//!
//! # Examples
//! ## Storing a Type of Your Own
//! ```
//! # #[cfg(feature = "derive")] {
//! use atomiks::ordering::{Acquire, Release};
//! use atomiks::{Atom, Atomic};
//!
//! /// The side of the book an order rests on.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! enum Side {
//!     Bid,
//!     Ask,
//! }
//!
//! /// A resting quote: 32 bits of price, 16 of quantity, then a bit of side, in a `u64`.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! struct Quote {
//!     price: u32,
//!     quantity: u16,
//!     side: Side,
//! }
//!
//! // Each `None` takes a repr no value is, so neither `Option` widens its word.
//! static BEST_BID: Atomic<Option<Quote>> = Atomic::new(None);
//! static LAST_FILL: Atomic<Option<Side>> = Atomic::new(None);
//!
//! BEST_BID.store(Some(Quote { price: 10_050, quantity: 300, side: Side::Bid }), Release);
//! LAST_FILL.store(Some(Side::Ask), Release);
//! assert_eq!(BEST_BID.load(Acquire).map(|quote| quote.quantity), Some(300), "the quantity bid");
//! assert_eq!(LAST_FILL.load(Acquire), Some(Side::Ask), "the side last filled");
//! assert_eq!(size_of_val(&BEST_BID), 8, "and the quote in one `u64`, `None` too");
//! # }
//! ```
//!
//! ## Changing One Field
//! ```
//! # #[cfg(feature = "derive")] {
//! use atomiks::ordering::{AcqRel, Acquire, Release};
//! use atomiks::{Atom, Atomic};
//!
//! /// A resting quote: 32 bits of quantity, whether it may fill, then a byte of flags.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! struct Quote {
//!     quantity: u32,
//!     live: bool,
//!     flags: u8,
//! }
//!
//! /// The flag of a quote that only rests on the book, never trading as it arrives.
//! const POST_ONLY: u8 = 0b100;
//!
//! static QUOTE: Atomic<Quote> = Atomic::new(Quote { quantity: 300, live: false, flags: 0 });
//!
//! // Each changes its field alone, in one instruction on the quote's word: `lock or`, or `ldset`.
//! QUOTE.fields().live.set(Release);
//! QUOTE.fields().flags.or(POST_ONLY, Release);
//! // `lock btr`, or `ldclral`: the one thread that finds it live takes it off the book.
//! assert!(QUOTE.fields().live.test_and_clear(AcqRel), "this thread took it");
//! let expected = Quote { quantity: 300, live: false, flags: POST_ONLY };
//! assert_eq!(QUOTE.load(Acquire), expected, "the bit off, the flag on, the quantity as it was");
//! # }
//! ```
//!
//! ## Keeping Tags in a Pointer
//! ```
//! # #[cfg(feature = "derive")] {
//! use core::ptr::NonNull;
//!
//! use atomiks::ordering::{AcqRel, Acquire, Relaxed};
//! use atomiks::{Atom, Atomic, RangedU8};
//!
//! /// A node of a stack, which holds the head below it: aligned to 8, so a pointer to one leaves
//! /// three low bits clear.
//! #[repr(align(8))]
//! struct Node {
//!     value: u64,
//!     next: Atomic<Head>,
//! }
//!
//! /// A node that lives as long as the program, so the statics below may hold its pointer.
//! static NODE: Node = Node { value: 7, next: Atomic::new(EMPTY) };
//!
//! /// A Treiber stack's head: the top node, then in its pointer's clear bits a version, which
//! /// counts the head's changes, and whether the stack is closed. The version tells a top popped
//! /// and pushed again from the one read.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! struct Head {
//!     top: Option<NonNull<Node>>,
//!     version: RangedU8<0, 3>,
//!     closed: bool,
//! }
//!
//! /// A slot of a table: empty, a value held inline, or a node, by a tag of two bits.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! enum Slot {
//!     Empty,
//!     Inline(u32),
//!     Node(NonNull<Node>),
//! }
//!
//! /// An open stack's empty head.
//! const EMPTY: Head = Head { top: None, version: RangedU8::MIN, closed: false };
//!
//! // A constant reads no pointer's address but null's, so a static starts with a null top, or a
//! // value held inline, and takes a node's pointer at run time.
//! static HEAD: Atomic<Head> = Atomic::new(EMPTY);
//! static SLOT: Atomic<Slot> = Atomic::new(Slot::Inline(42));
//!
//! let top = NonNull::from_ref(&NODE);
//! let empty = HEAD.load(Acquire);
//! NODE.next.store(empty, Relaxed);
//! let version = empty.version.checked_add(1).unwrap_or(RangedU8::MIN);
//! let pushed = Head { top: Some(top), version, ..empty };
//! assert_eq!(HEAD.compare_exchange(empty, pushed, AcqRel, Acquire), Ok(empty), "pushed, counted");
//! // `lock bts`, or `ldsetal`, on the pointer: the one thread that finds the stack open closes it.
//! assert!(!HEAD.fields().closed.test_and_set(AcqRel), "this thread closed the stack");
//! assert_eq!(HEAD.fields().top.load(Acquire), Some(top), "its top, read through the head");
//!
//! assert_eq!(SLOT.swap(Slot::Node(top), AcqRel), Slot::Inline(42), "the value held inline");
//! // The value lies above the three bits a node's alignment clears, beside its tag, 1.
//! assert_eq!(Slot::Inline(5).to_repr().addr(), 5 << 3 | 1, "the value and its tag in one word");
//! assert_eq!(size_of_val(&SLOT), 8, "a node, a value or nothing, in one pointer");
//! # }
//! ```
//!
//! ## Handing Out Ids
//! ```
//! use core::num::NonZero;
//!
//! use atomiks::ordering::{Acquire, Relaxed, Release};
//! use atomiks::{Atomic, AtomicU64};
//!
//! static NEXT: AtomicU64 = AtomicU64::new(1);
//! static OWNER: Atomic<Option<NonZero<u64>>> = Atomic::new(None);
//!
//! OWNER.store(NonZero::new(NEXT.fetch_add(1, Relaxed)), Release);
//! assert_eq!(OWNER.load(Acquire), NonZero::new(1), "the first id taken");
//! ```
//!
//! ## Reserving Integers for a Lock's States
//! ```
//! # #[cfg(feature = "derive")] {
//! use atomiks::ordering::{AcqRel, Acquire};
//! use atomiks::{Atom, Atomic, RangedU64};
//!
//! /// An owner's id, from 3, which leaves 0 to 2 spare.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! struct OwnerId(RangedU64<3>);
//!
//! /// A lock, whose unit variants take integers below the ids.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! enum Lock {
//!     Free,
//!     Poisoned,
//!     Held(OwnerId),
//! }
//!
//! static LOCK: Atomic<Option<Lock>> = Atomic::new(Some(Lock::Free));
//!
//! let owner = OwnerId(RangedU64::new(7).expect("7 is from 3 up"));
//! let (free, held) = (Some(Lock::Free), Some(Lock::Held(owner)));
//! assert_eq!(LOCK.compare_exchange(free, held, AcqRel, Acquire), Ok(free), "taken by owner 7");
//! assert_eq!((Lock::Free.to_repr(), Lock::Poisoned.to_repr()), (1, 2), "the states below 3");
//! assert_eq!(None::<Lock>.to_repr(), 0, "`None` below them");
//! assert_eq!(size_of_val(&LOCK), 8, "and the lock in the id's `u64`");
//! # }
//! ```
//!
//! ## Placing Atomics in Shared Memory
//! ```
//! # #[cfg(feature = "zerocopy-08")] {
//! use core::num::NonZero;
//!
//! use atomiks::ordering::{Acquire, Release};
//! use atomiks::{Atomic, AtomicU64};
//! use zerocopy::{FromBytes, IntoBytes, KnownLayout};
//!
//! /// The header of a page processes share; every repr of each field decodes, so any bytes do.
//! #[derive(FromBytes, IntoBytes, KnownLayout)]
//! #[repr(C)]
//! struct Header {
//!     seq: AtomicU64,
//!     owner: Atomic<Option<NonZero<u64>>>,
//! }
//!
//! // Words, so the page is aligned as `Header` is.
//! let mut page = [7_u64, 0];
//! // A shared atomic writes, so none is `Immutable`: a page this process lays out alone becomes a
//! // header through `&mut`; one another process may already be writing is reached through a
//! // pointer, as `Atomic::from_ptr` takes.
//! let header: &Header = Header::mut_from_bytes(page.as_mut_bytes()).expect("16 aligned bytes");
//! assert_eq!(header.owner.load(Acquire), None, "a zero owner is `None`");
//! header.owner.store(NonZero::new(42), Release);
//! assert_eq!(page, [7, 42], "and the store, in the page");
//! # }
//! ```
//!
//! # What Each Operation Costs
//!
//! An operation exists only where it is what its name says:
//!
//! - orderings are types ([`ordering`]), so `load(Release)` does not compile;
//! - a read-modify-write needs the capability that makes it mean something: [`AtomAdd`],
//!   [`AtomOrd`], [`AtomBitwise`]; anything else is [`update`](Atomic::update);
//! - an operation exists only where the target runs it without a compare-exchange loop, as
//!   [Platforms](#platforms) tables: without a 16-byte [`Load`], [`load_rmw`](Atomic::load_rmw)
//!   reads with one compare-exchange, which writes, by name; [`fetch_or`](Atomic::fetch_or),
//!   [`fetch_max`](Atomic::fetch_max) and [`fetch_min`](Atomic::fetch_min) need [`FetchBitwise`]
//!   and [`MinMax`], while [`or`](Atomic::or), which discards the value before, needs no loop
//!   anywhere in an optimized build;
//! - a field's operations keep the last two rules: each but its loops,
//!   [`update`](AtomicField::update) and [`try_update`](AtomicField::try_update), and its
//!   [`load_rmw`](AtomicField::load_rmw) is one instruction on the whole word, and needs what the
//!   field's value promises, [`FieldBitwise`] for the bitwise operations and [`FieldAdd`] for the
//!   add, and what the container's repr runs: [`MaskBitwise`], [`BitTest`] or [`FetchAdd`].
//!
//! The `fetch_` operations take core's names, and return the value before as core's do, a field's
//! the container before. Only [`and`](Atomic::and), [`or`](Atomic::or), [`xor`](Atomic::xor) and
//! [`not`](Atomic::not), on an atomic or a field, and a `bool` field's [`set`](AtomicField::set),
//! [`clear`](AtomicField::clear) and [`toggle`](AtomicField::toggle) discard it.
//!
//! A tagged pointer's store, swap or compare-exchange costs one test and branch, with no frame
//! record on its path: a word nested in others tests its pointer once against every tag of each,
//! an exchange tests both its pointers in one test, and a pointer enum's unit or data, or a pointer
//! that fills a niche beside no tag field, tests nothing. The branch is to one cold refusal, a
//! panic, where the pointer has a bit set that its tags take. A compare-exchange loop tests nothing
//! its decode already cleared: an [`update`](Atomic::update) that keeps the pointer tests, calls
//! and saves nothing, and a Treiber stack's pop tests only the next node's pointer, which it reads
//! from the node, and saves the frame record its cold refusal needs. The one exception is a word
//! that holds a pointer enum of several pointer variants: its update saves a frame record on
//! `aarch64`, and on `x86_64` keeps a branch to the cold refusal that it never takes. A match of a
//! pointer enum folds an arm's tag into the offset of the load through its pointer; where LLVM
//! merges arms that read through pointers of several tags, one mask takes the tags off. A constant
//! reads no pointer's address but null's, so it decodes only null, and tags only a null pointer.
//!
//! # Platforms
//!
//! atomiks builds for Linux and macOS, on `aarch64` and `x86_64`. What a target's default CPU
//! lacks, a flag adds, `-C target-feature` on `aarch64` and `-C target-cpu` on `x86_64`:
//!
//! | Target          | 128-Bit Atomics        | Their `Load`, `Store` | `FetchBitwise`, `MinMax` |
//! | --------------- | ---------------------- | --------------------- | ------------------------ |
//! | `aarch64` macOS | yes                    | yes, with LSE2        | yes                      |
//! | `aarch64` Linux | yes                    | `+lse2`               | yes                      |
//! | `x86_64` macOS  | yes, with `cmpxchg16b` | `x86-64-v3`, for AVX  | no                       |
//! | `x86_64` Linux  | `x86-64-v2`            | `x86-64-v3`, for AVX  | no                       |
//!
//! Every target has [`FetchAdd`] and [`MaskBitwise`] for each integer up to 64 bits, and
//! [`BitTest`] from 16 bits on `x86_64`, whose `lock bts` takes no byte, and from 8 on `aarch64`;
//! and [`MaskBitwise`] and [`BitTest`] for a pointer, whose tags they change.
//!
//! On `aarch64` Linux, a read-modify-write is LSE's one instruction with `+lse` (Armv8.1); without
//! it, an outline call runs that instruction where the CPU has LSE, and an LL/SC loop where it does
//! not. Apple's CPUs all have LSE.
//!
//! # Model Checking
//!
//! Under `--cfg loom` with the `loom` feature, every atomic, [`fence`](fn@fence), [`cell`] and
//! [`hint::spin_loop`] is loom's, so the same code is the model. [`compiler_fence`] stays core's:
//! it orders nothing between threads. There, `Atomic`'s `new`, `into_inner`, `get` and `set` are
//! not `const`, nor are the cell's methods; `Atomic`'s `as_ptr`, `from_ptr` and `get_mut`, and the
//! cell's `as_ptr` and `raw_get`, do not exist.
//!
//! A crate models its own code with this dependency, under `RUSTFLAGS="--cfg loom"`:
//!
//! ```toml
//! [target.'cfg(loom)'.dependencies]
//! atomiks = { version = "0.1", features = ["loom"] }
//! ```
//!
//! Each model runs in `atomiks::model::check`, and spawns with `atomiks::model::{thread, Arc}`.
//!
//! # Crate Features
//!
//! None is on by default. Under `--cfg loom`, whose cells are not plain memory, an atomic has
//! neither zerocopy's traits nor `Zeroable`.
//!
//! | Feature         | Adds                                                                       |
//! | --------------- | -------------------------------------------------------------------------- |
//! | `derive`        | `#[derive(Atom)]`: projections, tagged pointers; a newtype's capabilities  |
//! | `serde`         | serde's traits for an atomic, and a ranged integer, held to its range      |
//! | `zerocopy-08`   | zerocopy's traits for an atomic, as its validity allows, never `Immutable` |
//! | `bytemuck`      | `Zeroable` for an atomic, and the bit-pattern traits for a ranged integer  |
//! | `arbitrary`     | `Arbitrary` for an atomic, and a ranged integer, held to its range         |
//! | `arbitrary-int` | `Atom`, `AtomOrd`, `FieldBitwise` and `FieldAdd` for `UInt` and `Int`      |
//! | `deranged-05`   | `Atom`, `AtomOrd` and `From` with atomiks' own for deranged 0.5's integers |
//! | `loom`          | loom's types under `--cfg loom`; nothing without the cfg                   |
//!
//! atomiks holds `atomiks-derive`, which `derive` adds, at its own version whether the feature is
//! on or not, so the code a derive writes always calls the hidden items it was written against.

#![no_std]
#![feature(doc_cfg)]
// No badge names `loom` or atomiks-core's alias `wide`: a loom build is a model of this one, not a
// target of its own, and the 128-bit atomics write out the condition `wide` stands for.
#![doc(auto_cfg(hide(loom, wide)))]

#[doc(hidden)]
pub use atomiks_core::__private;
#[cfg(loom)]
#[doc(inline)]
pub use atomiks_core::model;
pub use atomiks_core::{
    Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, AtomicBool, AtomicField, AtomicI8, AtomicI16,
    AtomicI32, AtomicI64, AtomicIsize, AtomicPtr, AtomicU8, AtomicU16, AtomicU32, AtomicU64,
    AtomicUsize, BitTest, ExactBits, FetchAdd, FetchBitwise, Field, FieldAdd, FieldBitwise,
    FieldPath, Join, Load, MaskBitwise, MinMax, ParseRangeError, Primitive, ProjectFields, PtrAtom,
    RangeError, RangedI8, RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize, RangedU8,
    RangedU16, RangedU32, RangedU64, RangedU128, RangedUsize, RawAccess, ReadByExchange, ReprRange,
    Store, Swap, Then, TopField, Whole, compiler_fence, fence,
};
#[cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
))]
pub use atomiks_core::{AtomicI128, AtomicU128, DoubleWord, VtablePointer};
#[doc(inline)]
pub use atomiks_core::{cell, hint, ordering, validity};
#[cfg(feature = "derive")]
pub use atomiks_derive::{Atom, AtomAdd, AtomBitwise, AtomOrd};
