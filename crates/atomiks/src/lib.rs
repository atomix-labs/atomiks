//! Typed atomics for any value that fits one atomic word.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out. The integer, `bool` and pointer
//! atomics are its aliases (`AtomicU64 = Atomic<u64>`), and any value implementing [`Atom`] is one
//! more: a `NonZero`, a `char`, a float, an `Option` that spends a spare repr on `None`.
//!
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
//! assert_eq!(OWNER.load(Acquire), NonZero::new(1));
//! ```
//!
//! # What each operation costs
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
//! # Model checking
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
//! # Crate features
//!
//! | Feature | Enables |
//! | ------- | ------- |
//! | `loom`  | loom's types under `--cfg loom`; nothing without the cfg |

#![no_std]
#![feature(
    associated_type_defaults,
    const_convert,
    const_destruct,
    const_index,
    const_trait_impl,
    const_type_name,
    doc_cfg,
    f16,
    impl_restriction,
    integer_casts
)]
#![cfg_attr(not(loom), feature(const_atomic))]
#![cfg_attr(
    any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "cmpxchg16b")),
    feature(f128)
)]
#![cfg_attr(all(target_arch = "aarch64", not(loom)), feature(integer_atomics))]
#![cfg_attr(
    all(target_arch = "x86_64", target_feature = "cmpxchg16b", target_feature = "avx", not(loom)),
    feature(core_intrinsics)
)]
#![cfg_attr(
    all(target_arch = "x86_64", target_feature = "cmpxchg16b", target_feature = "avx", not(loom)),
    expect(
        internal_features,
        reason = "the AVX 16-byte load and store are core's atomic intrinsics"
    )
)]
// A loom build is a model of this one, not a target of its own: no badge names it.
#![doc(auto_cfg(hide(loom)))]

#[cfg(all(loom, not(feature = "loom")))]
compile_error!(concat!(
    "atomiks: `--cfg loom` needs the `loom` feature: pass `--features atomiks/loom`, or depend \
     with `[target.'cfg(loom)'.dependencies] atomiks = { version = \"",
    env!("CARGO_PKG_VERSION"),
    "\", features = [\"loom\"] }`"
));

// The loom model of a 128-bit cell keeps its table in `std`; `alloc` only where that model is.
#[cfg(all(
    loom,
    any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "cmpxchg16b"))
))]
extern crate alloc;
#[cfg(loom)]
extern crate std;

mod atom;
mod atomic;
pub mod cell;
mod fence;
pub mod hint;
mod message;
#[cfg(loom)]
pub mod model;
pub mod ordering;
mod primitive;
pub mod validity;

pub use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd};
pub use crate::atomic::{
    Atomic, AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr,
    AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
};
#[cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
))]
pub use crate::atomic::{AtomicI128, AtomicU128};
pub use crate::fence::{compiler_fence, fence};
#[doc(hidden)]
pub use crate::primitive::{Bitwise, CellAccess, CellOps, FetchAdd};
pub use crate::primitive::{FetchBitwise, Integer, Load, MinMax, Primitive, Store, Swap};

/// The loom this crate models with, so a downstream model uses the same copy.
#[cfg(loom)]
#[doc(hidden)]
pub mod __private {
    pub use loom;
}
