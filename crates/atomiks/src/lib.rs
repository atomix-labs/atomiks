//! Typed atomics for any value that fits one atomic word.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out. The integer, `bool` and pointer
//! atomics are its aliases (`AtomicU64 = Atomic<u64>`), and any value implementing [`Atom`] is one
//! more: a `NonZero`, a `char`, a float, an `Option` that spends a spare repr on `None`.
//!
//! # Types
//!
//! - **The atomic.** [`Atomic<T>`](Atomic), with an alias for each primitive, [`AtomicU64`] and the
//!   rest.
//! - **Values.** [`Atom`] encodes a value as its repr and back, and its [`validity`] says which
//!   reprs decode; [`AtomAdd`], [`AtomOrd`] and [`AtomBitwise`] add the read-modify-writes that
//!   mean something for it.
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
//! - an operation exists only where the target runs it without a compare-exchange loop: a 16-byte
//!   [`Load`] needs LSE2 on `aarch64` or AVX on `x86_64`, else [`load_rmw`](Atomic::load_rmw) reads
//!   with one compare-exchange, which writes, by name; [`fetch_or`](Atomic::fetch_or) and every
//!   form of [`max`](Atomic::max) and [`min`](Atomic::min) need `aarch64` ([`FetchBitwise`],
//!   [`MinMax`]), while [`or`](Atomic::or), which discards the value before, needs no loop anywhere
//!   in an optimized build.
//!
//! On `x86_64`, `AtomicU128` and `AtomicI128` need `cmpxchg16b` (`-C target-cpu=x86-64-v2`), and
//! their [`Load`] and [`Store`] need AVX (`-C target-cpu=x86-64-v3`); `x86_64-unknown-linux-gnu`'s
//! default CPU has neither.
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
//! | Feature | Adds                                                     |
//! | ------- | -------------------------------------------------------- |
//! | `loom`  | loom's types under `--cfg loom`; nothing without the cfg |

#![no_std]
#![feature(doc_cfg)]
// A loom build is a model of this one, not a target of its own: no badge names it. Nor does
// atomiks-core's `wide`, whose condition the 128-bit atomics write out.
#![doc(auto_cfg(hide(loom, wide)))]

#[cfg(loom)]
#[doc(inline)]
pub use atomiks_core::model;
pub use atomiks_core::{
    Atom, AtomAdd, AtomBitwise, AtomOrd, Atomic, AtomicBool, AtomicI8, AtomicI16, AtomicI32,
    AtomicI64, AtomicIsize, AtomicPtr, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
    ExactBits, FetchBitwise, Load, MinMax, Primitive, Store, Swap, compiler_fence, fence,
};
#[cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
))]
pub use atomiks_core::{AtomicI128, AtomicU128};
#[doc(inline)]
pub use atomiks_core::{cell, hint, ordering, validity};
