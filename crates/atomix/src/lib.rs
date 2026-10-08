//! Typed atomics for any value that fits one atomic, and the locks built on them.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out. The integer, `bool` and pointer
//! atomics are its aliases (`AtomicU64 = Atomic<u64>`), and any value implementing [`Atom`] is one
//! more: a `NonZero`, a `char`, a float, a ranged integer, an `Option` that spends a spare repr on
//! `None`, and a struct or an enum that derives it.
//!
//! A crate depends on `atomix-rs`, since crates.io's `atomix` is an unrelated 2017 placeholder:
//!
//! ```toml
//! [dependencies]
//! atomix-rs = { version = "0.1", features = ["derive"] }
//! ```
//!
//! and imports it as `atomix`:
//!
//! ```
//! use atomix::{Atom, Atomic};
//! ```
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
//!   for it, and [`FieldBitwise`] and [`FieldAdd`] those of a field of it. Two words hold a pair
//!   `(P, Q)` of values stored as pointers, and a pointer to a slice, a `str` or a trait object.
//!   With the `derive` feature, each derives: `Atom` for a struct or an enum, a pointer word or a
//!   pointer enum among them, which keeps small fields as tags in its pointer's low bits and the
//!   pointer's provenance, and each capability for a newtype whose field has it. A pointer word
//!   takes two words where a second pointer, a wide one or tags no alignment holds need them.
//!   [`RangedU64<MIN, MAX>`](RangedU64) and its siblings, `RangedU8` to `RangedIsize`, hold an
//!   integer from `MIN` to `MAX`, and [`RangeError`] and [`ParseRangeError`] say why one refused an
//!   integer or a text.
//! - **Orderings.** The [`ordering`] types, each accepted only where it means something, and the
//!   [`fence`](fn@fence) and [`compiler_fence`] they order.
//! - **Primitives.** [`Primitive`], [`ExactBits`] where the bits are the whole value, a
//!   [`DoubleWord`], two words in one 16-byte atomic, and what the target runs without a loop:
//!   [`Load`], [`Store`], [`Swap`], [`FetchBitwise`], [`MinMax`], and, for a field's container,
//!   [`FetchAdd`], [`MaskBitwise`], [`BitTest`], which an atomic's [`bit_set`](Atomic::bit_set)
//!   needs too; [`ReadByExchange`], which [`load_rmw`](Atomic::load_rmw) needs; and [`RawAccess`],
//!   a primitive whose cell lends its place, which every primitive but a double word is.
//! - **Building blocks.** The loom-shaped [`cell`], the spin [`hint`], and `model` under loom.
//!
//! # Examples
//! ## Storing a Type of Your Own
//! ```
//! # #[cfg(feature = "derive")] {
//! use atomix::ordering::{Acquire, Release};
//! use atomix::{Atom, Atomic};
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
//! use atomix::ordering::{AcqRel, Acquire, Release};
//! use atomix::{Atom, Atomic};
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
//! use atomix::ordering::{AcqRel, Acquire, Relaxed};
//! use atomix::{Atom, Atomic, RangedU8};
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
//! ## Holding Two Words in One Atomic
//! ```
//! # #[cfg(all(
//! #     feature = "derive",
//! #     any(target_arch = "aarch64", target_arch = "arm64ec", target_feature = "cmpxchg16b"),
//! # ))] {
//! use core::ptr::NonNull;
//!
//! use atomix::ordering::{AcqRel, Acquire, Relaxed, Release};
//! use atomix::{Atom, Atomic};
//!
//! /// A node of a stack, which holds the node below it.
//! struct Node {
//!     value: u64,
//!     next: Atomic<Option<NonNull<Node>>>,
//! }
//!
//! /// A node that lives as long as the program, so the statics below may hold its pointer.
//! static NODE: Node = Node { value: 7, next: Atomic::new(None) };
//!
//! /// A Treiber stack's head: the top node, and a version that counts the head's changes, so a top
//! /// popped and pushed again is told from the one read. No alignment leaves its 64 bits clear in
//! /// the pointer, so it takes a word of its own.
//! #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
//! struct Head {
//!     top: Option<NonNull<Node>>,
//!     version: u64,
//! }
//!
//! static HEAD: Atomic<Head> = Atomic::new(Head { top: None, version: 0 });
//! /// The last message a feed received, as the bytes it arrived in: their pointer and their length.
//! static LAST_MESSAGE: Atomic<Option<NonNull<[u8]>>> = Atomic::new(None);
//! /// A message that lives as long as the program.
//! static MESSAGE: [u8; 4] = *b"fill";
//!
//! let top = NonNull::from_ref(&NODE);
//! let push = |head: Head| {
//!     NODE.next.store(head.top, Relaxed);
//!     Head { top: Some(top), version: head.version.wrapping_add(1) }
//! };
//! assert_eq!(HEAD.update(AcqRel, Acquire, push).top, None, "pushed onto the empty stack");
//! // The pop changes both words in one `lock cmpxchg16b`, or `caspal`.
//! let pushed = Head { top: Some(top), version: 1 };
//! let popped = Head { top: NODE.next.load(Relaxed), version: 2 };
//! assert_eq!(HEAD.compare_exchange(pushed, popped, AcqRel, Acquire), Ok(pushed), "popped");
//! assert_eq!(size_of_val(&HEAD), 16, "the top and its version in two words");
//!
//! // `store_rmw` and `load_rmw` run on every target, with compare-exchanges: `store` and `load`
//! // need LSE2 or AVX.
//! LAST_MESSAGE.store_rmw(Some(NonNull::from_ref(&MESSAGE[..])), Release);
//! assert_eq!(LAST_MESSAGE.load_rmw(Acquire).map(NonNull::len), Some(4), "its length, beside its pointer");
//! # }
//! ```
//!
//! ## Handing Out Ids
//! ```
//! use core::num::NonZero;
//!
//! use atomix::ordering::{Acquire, Relaxed, Release};
//! use atomix::{Atomic, AtomicU64};
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
//! use atomix::ordering::{AcqRel, Acquire};
//! use atomix::{Atom, Atomic, RangedU64};
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
//! use atomix::ordering::{Acquire, Release};
//! use atomix::{Atomic, AtomicU64};
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
//! the container before. [`bit_set`](Atomic::bit_set), [`bit_clear`](Atomic::bit_clear) and
//! [`bit_toggle`](Atomic::bit_toggle), on a bit chosen at run time, take [portable-atomic]'s
//! names, and return the bit before as portable-atomic's do. Only [`and`](Atomic::and),
//! [`or`](Atomic::or), [`xor`](Atomic::xor) and [`not`](Atomic::not), on an atomic or a field, and
//! a `bool` field's [`set`](AtomicField::set), [`clear`](AtomicField::clear) and
//! [`toggle`](AtomicField::toggle) discard the value before.
//!
//! A tagged pointer's store, swap or compare-exchange costs one test and branch, and saves no frame
//! record on its path on macOS, on `x86_64` Linux, and on `aarch64` Linux with `+lse`: a word
//! nested in others tests its pointer once against every tag of each, an exchange tests both its
//! pointers in one test, and a pointer enum's unit or data, or a pointer that fills a niche beside
//! no tag field, tests nothing. The branch is to one cold refusal, a panic, where the pointer has a
//! bit set that its tags take. On `x86_64` Windows, a function that can call the refusal reserves
//! its stack before the test. At Android's default CPU, without LSE, a compare-exchange saves its
//! frame record before the test too, and restores it after its LL/SC loop. A compare-exchange loop
//! tests nothing its decode already cleared: an [`update`](Atomic::update) that keeps the pointer
//! tests, calls and saves nothing, and a Treiber stack's pop tests only the next node's pointer,
//! which it reads from the node, and saves the frame record its cold refusal needs. The one
//! exception is a word that holds a pointer enum of several pointer variants: its update saves a
//! frame record on `aarch64`, and on `x86_64` keeps a branch to the cold refusal that it never
//! takes. A match of a pointer enum folds an arm's tag into the offset of the load through its
//! pointer; where LLVM merges arms that read through pointers of several tags, one mask takes the
//! tags off. A constant reads no pointer's address but null's, so it decodes only null, and tags
//! only a null pointer.
//!
//! Two words are one 16-byte atomic, with `u128`'s operations: a compare-exchange is
//! `lock cmpxchg16b`, or `caspal`; [`load`](Atomic::load) and [`store`](Atomic::store) are AVX's
//! `vmovdqa`, or LSE2's `ldp` and `stp`, and where [Platforms](#platforms) shows neither,
//! [`load_rmw`](Atomic::load_rmw) reads with a compare-exchange and
//! [`store_rmw`](Atomic::store_rmw) writes with a loop of them; and no target swaps 16 bytes in one
//! instruction, so there is no `swap`. A field of a two-word value has its loads and its loops
//! alone, as no target has a 16-byte bitwise instruction.
//!
//! No Rust operation keeps a provenance through a 16-byte atomic ([UCG #517][ucg-517]), so a
//! [`DoubleWord`] exposes each pointer it stores, and each pointer it loads takes back an exposed
//! provenance, which costs no instruction. It is the one place atomix exposes a provenance, chosen
//! per type by declaring one of two words: a pointer word or a pointer enum in one word keeps its
//! pointer's strictly. [`DoubleWord`]'s page says what that means for a `static`, for an atomic's
//! place and under Miri. A trait object's compare-exchange compares its vtable pointer beside its
//! data pointer, and one type may have several vtables, so it takes `current` from a load or a
//! failed exchange, as [`update`](Atomic::update) does, never from a pointer coerced afresh.
//!
//! A structure whose nodes share one arena keeps strict provenance in an integer word: it holds a
//! node's offset from the arena's base, beside a version where both fit, and rebuilds the node's
//! pointer from the base with `add` or `with_addr`, which keep the base's provenance.
//!
//! # Platforms
//!
//! atomix builds for every `aarch64` and `x86_64` target with 64-bit pointers, little-endian, on
//! any OS or none, and for `arm64ec`, the `aarch64` code Windows runs beside `x86_64` code; its
//! build script refuses any other target, where an operation it promises as one instruction could
//! be a compare-exchange loop, as a 64-bit add is on 32-bit x86. CI runs the tests on Linux, macOS
//! and Windows, and each night builds for every target rustup ships that the build script admits,
//! those for Android, Apple's other systems, FreeBSD, NetBSD, illumos, Solaris, Fuchsia, Redox,
//! UEFI and bare metal among them. It needs a nightly Rust, `nightly-2026-09-28` or newer, for
//! `const_trait_impl`, pattern types and the other unstable features its crates enable; that
//! nightly's version, 1.101, is its `rust-version`.
//!
//! What an operation lowers to depends on the features a target turns on, which
//! `rustc --print cfg --target <triple>` lists, not on its OS; what a target's default lacks, a
//! flag adds. For the targets rustup ships:
//!
//! | Target                             | Read-Modify-Write               | 128-Bit Atomics        | Their `Load`, `Store` |
//! | ---------------------------------- | ------------------------------- | ---------------------- | --------------------- |
//! | `aarch64` macOS                    | LSE                             | yes                    | yes, with LSE2        |
//! | `aarch64` simulators, Mac Catalyst | LSE                             | yes                    | a CPU with LSE2       |
//! | `aarch64` Linux gnu, musl          | an outline call; `+lse` for LSE | yes                    | a CPU with LSE2       |
//! | `aarch64` Windows, `arm64ec`       | an LL/SC loop; `+lse` for LSE   | yes                    | a CPU with LSE2       |
//! | `aarch64` elsewhere                | an LL/SC loop; `+lse` for LSE   | yes                    | a CPU with LSE2       |
//! | `x86_64` Apple, Windows, Fuchsia   | one instruction                 | yes, with `cmpxchg16b` | `x86-64-v3`, for AVX  |
//! | `x86_64` elsewhere                 | one instruction                 | `x86-64-v2`            | `x86-64-v3`, for AVX  |
//!
//! LSE2 (Armv8.4) comes with a CPU, not a flag: rustc warns that `-C target-feature=+lse2` is
//! unstable and will be refused. A CPU turns on more than LSE2, so `neoverse-n2`'s SVE2 faults on
//! Graviton 3: name the one the code runs on. `neoverse-v1` (Graviton 3), `neoverse-n2` (Azure
//! Cobalt 100), `neoverse-v2` (Graviton 4) and `apple-m1` have LSE2, as does `native` on any of
//! them; `neoverse-n1` (Graviton 2) does not.
//!
//! Where an OS requires a CPU, its flag costs nothing: `+lse` for Windows 11 24H2,
//! `-C target-cpu=apple-a12` for iOS 18 and `apple-a13`, with LSE2, for iOS 26, `x86-64-v2` for
//! RHEL 9 and `x86-64-v3` for RHEL 10.
//!
//! Every target has [`FetchAdd`] and [`MaskBitwise`] for each integer up to 64 bits, and
//! [`BitTest`] from 16 bits on `x86_64`, whose `lock bts` takes no byte, and from 8 on `aarch64`;
//! and [`MaskBitwise`] and [`BitTest`] for a pointer, whose tags they change. `aarch64` alone has
//! [`FetchBitwise`] and [`MinMax`]: on `x86_64`, an `and`, `or` or `xor` that returns the value
//! before, and a maximum or a minimum, are compare-exchange loops.
//!
//! On `aarch64` Linux gnu or musl without `+lse` (Armv8.1), the outline call runs LSE's one
//! instruction where the CPU has LSE, and an LL/SC loop where it does not.
//!
//! # Model Checking
//!
//! Under `--cfg loom` with the `loom` feature, every atomic, [`fence`](fn@fence), [`cell`] and
//! [`hint::spin_loop`] is loom's, so the same code is the model. [`compiler_fence`] stays core's:
//! it orders nothing between threads. There, `Atomic`'s `new`, `into_inner`, `get`, `set`, `From`
//! and `Default` are not `const`, nor are the cell's methods; `Atomic`'s `as_ptr`, `from_ptr`,
//! `get_mut`, `from_mut`, `get_mut_slice` and `from_mut_slice`, and the cell's `as_ptr` and
//! `raw_get`, do not exist.
//!
//! A crate models its own code with this dependency, under `RUSTFLAGS="--cfg loom"`:
//!
//! ```toml
//! [target.'cfg(loom)'.dependencies]
//! atomix-rs = { version = "0.1", features = ["loom"] }
//! ```
//!
//! Each model runs in `atomix::model::check`, and spawns with `atomix::model::{thread, Arc}`.
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
//! | `deranged-05`   | `Atom`, `AtomOrd` and `From` with atomix's own for deranged 0.5's integers |
//! | `loom`          | loom's types under `--cfg loom`; nothing without the cfg                   |
//!
//! atomix holds `atomix-derive`, which `derive` adds, at its own version whether the feature is
//! on or not, so the code a derive writes always calls the hidden items it was written against.
//!
//! [portable-atomic]: https://docs.rs/portable-atomic
//! [ucg-517]: https://github.com/rust-lang/unsafe-code-guidelines/issues/517

// Where no atomic holds two words, no `DoubleWord` exists: its links lead to where one does.
#![cfg_attr(
    not(any(
        target_arch = "aarch64",
        target_arch = "arm64ec",
        all(target_arch = "x86_64", target_feature = "cmpxchg16b")
    )),
    doc = "[`DoubleWord`]: #platforms"
)]
#![no_std]
#![feature(doc_cfg)]
// No badge names `loom` or atomix-core's aliases `aarch64_code` and `wide`: a loom build is a model
// of this one, not a target of its own, and an item writes out an alias's condition where its badge
// needs one.
#![doc(auto_cfg(hide(loom, aarch64_code, wide)))]

#[doc(hidden)]
pub use atomix_core::__private;
#[cfg(loom)]
#[doc(inline)]
pub use atomix_core::model;
pub use atomix_core::{
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
    target_arch = "arm64ec",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
))]
pub use atomix_core::{AtomicI128, AtomicU128, DoubleWord, VtablePointer};
#[doc(inline)]
pub use atomix_core::{cell, hint, ordering, validity};
#[cfg(feature = "derive")]
pub use atomix_derive::{Atom, AtomAdd, AtomBitwise, AtomOrd};
