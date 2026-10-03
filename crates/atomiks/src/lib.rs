//! Typed atomics for any value that fits one atomic word.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out. The integer, `bool` and pointer
//! atomics are its aliases (`AtomicU64 = Atomic<u64>`), and any value implementing [`Atom`] is one
//! more: a `NonZero`, a `char`, a float, an `Option` that spends a spare repr on `None`.
//!
//! # Types
//!
//! - **The atomic.** [`Atomic<T>`](Atomic), with an alias per primitive, such as [`AtomicU64`].
//! - **Values.** [`Atom`] encodes a value as its repr and back, its [`ReprRange`] says which reprs
//!   it takes and its [`validity`] which decode; [`AtomAdd`], [`AtomOrd`] and [`AtomBitwise`] add
//!   the read-modify-writes that mean something for it.
//! - **Orderings.** The [`ordering`] types, each accepted only where it means something, and the
//!   [`fence`](fn@fence) and [`compiler_fence`] they order.
//! - **Primitives.** [`Primitive`], [`ExactBits`] where the bits are the whole value, and what the
//!   target runs without a loop: [`Load`], [`Store`], [`Swap`], [`FetchBitwise`], [`MinMax`].
//! - **Building blocks.** The loom-shaped [`cell`], the spin [`hint`], and `model` under loom.
//!
//! # Examples
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
//!   [`max`](Atomic::max) and [`min`](Atomic::min) need [`FetchBitwise`] and [`MinMax`], while
//!   [`or`](Atomic::or), which discards the value before, needs no loop anywhere in an optimized
//!   build.
//!
//! # Platforms
//!
//! atomiks builds for Linux and macOS, on `aarch64` and `x86_64`. What a target's default CPU
//! lacks, a flag adds, `-C target-feature` on `aarch64` and `-C target-cpu` on `x86_64`:
//!
//! | Target          | 128-Bit Atomics        | Their `Load`, `Store` | `fetch_or`, `max`, `min` |
//! | --------------- | ---------------------- | --------------------- | ------------------------ |
//! | `aarch64` macOS | yes                    | yes, with LSE2        | yes                      |
//! | `aarch64` Linux | yes                    | `+lse2`               | yes                      |
//! | `x86_64` macOS  | yes, with `cmpxchg16b` | `x86-64-v3`, for AVX  | no                       |
//! | `x86_64` Linux  | `x86-64-v2`            | `x86-64-v3`, for AVX  | no                       |
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
//! None is on by default.
//!
//! | Feature  | Adds                                                                        |
//! | -------- | --------------------------------------------------------------------------- |
//! | `derive` | `#[derive(Atom)]`, and `AtomAdd`, `AtomOrd` and `AtomBitwise` for a newtype |
//! | `loom`   | loom's types under `--cfg loom`; nothing without the cfg                    |
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
    Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, AtomicBool, AtomicI8, AtomicI16, AtomicI32,
    AtomicI64, AtomicIsize, AtomicPtr, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
    ExactBits, FetchBitwise, Load, MinMax, Primitive, ReprRange, Store, Swap, compiler_fence,
    fence,
};
#[cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
))]
pub use atomiks_core::{AtomicI128, AtomicU128};
#[doc(inline)]
pub use atomiks_core::{cell, hint, ordering, validity};
#[cfg(feature = "derive")]
pub use atomiks_derive::{Atom, AtomAdd, AtomBitwise, AtomOrd};
