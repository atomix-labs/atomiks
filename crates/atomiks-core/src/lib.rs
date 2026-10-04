//! The typed atomic of atomiks, the values it holds, and the orderings and primitives under it.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out.
//!
//! # Examples
//! ```
//! use core::num::NonZero;
//!
//! use atomiks_core::ordering::{Acquire, Relaxed, Release};
//! use atomiks_core::{Atomic, AtomicU64};
//!
//! static NEXT: AtomicU64 = AtomicU64::new(1);
//! static OWNER: Atomic<Option<NonZero<u64>>> = Atomic::new(None);
//!
//! OWNER.store(NonZero::new(NEXT.fetch_add(1, Relaxed)), Release);
//! assert_eq!(OWNER.load(Acquire), NonZero::new(1), "the first id taken");
//! ```
//!
//! # Crate Features
//!
//! None is on by default.
//!
//! | Feature       | Adds                                                                       |
//! | ------------- | -------------------------------------------------------------------------- |
//! | `bytemuck`    | an atomic's `Zeroable`, off under loom, and a ranged integer's byte traits |
//! | `loom`        | loom's types under `--cfg loom`; nothing without the cfg                   |
//! | `serde`       | serde: an atomic by a `Relaxed` load; a ranged integer, held to its range  |
//! | `zerocopy-08` | an atomic's zerocopy traits: its cell's, never `Immutable`; off under loom |

#![no_std]
#![feature(
    associated_type_defaults,
    const_convert,
    const_destruct,
    const_index,
    const_option_ops,
    const_trait_impl,
    const_type_name,
    doc_cfg,
    f16,
    generic_pattern_types,
    impl_restriction,
    integer_casts,
    pattern_type_macro,
    pattern_types,
    step_trait,
    structural_match,
    transmute_neo
)]
#![cfg_attr(not(loom), feature(const_atomic))]
#![cfg_attr(wide, feature(f128))]
#![cfg_attr(core_atomic_u128, feature(integer_atomics))]
#![cfg_attr(all(target_arch = "x86_64", wide_load_store, not(loom)), feature(core_intrinsics))]
#![expect(
    internal_features,
    reason = "a ranged integer's field is a pattern type, and with AVX the 16-byte load and store \
              are core's atomic intrinsics"
)]
#![expect(
    incomplete_features,
    reason = "`generic_pattern_types` is the feature for a pattern type bounded by const \
              parameters, as a ranged integer's is"
)]
// No badge names `loom` or an alias `build.rs` declares: a loom build is a model of this one, not
// a target of its own, and an item writes out an alias's condition where its badge needs one.
#![doc(auto_cfg(hide(loom, wide, wide_load_store)))]

#[cfg(all(loom, not(feature = "loom")))]
compile_error!(concat!(
    "atomiks: `--cfg loom` needs the `loom` feature: depend with `[target.'cfg(loom)'.dependencies] \
     atomiks = { version = \"",
    env!("CARGO_PKG_VERSION"),
    "\", features = [\"loom\"] }`, or pass `--features atomiks/loom`, or `atomiks-core/loom` \
     where atomiks-core is the dependency"
));

// The loom model of a 128-bit cell keeps its table in `std`; `alloc` only where that model is.
#[cfg(all(loom, wide))]
extern crate alloc;
#[cfg(loom)]
extern crate std;

#[doc(hidden)]
pub mod __private;
mod atom;
mod atomic;
pub mod cell;
mod errors;
mod fence;
pub mod hint;
mod interop;
mod message;
#[cfg(loom)]
pub mod model;
pub mod ordering;
mod primitive;
mod range;
mod ranged;
pub mod validity;

pub use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd};
pub use crate::atomic::{
    Atomic, AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr,
    AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize,
};
#[cfg(wide)]
#[doc(cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
)))]
pub use crate::atomic::{AtomicI128, AtomicU128};
pub use crate::errors::{ParseRangeError, RangeError};
pub use crate::fence::{compiler_fence, fence};
pub use crate::primitive::{ExactBits, FetchBitwise, Load, MinMax, Primitive, Store, Swap};
pub use crate::range::ReprRange;
pub use crate::ranged::{
    RangedI8, RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize, RangedU8, RangedU16,
    RangedU32, RangedU64, RangedU128, RangedUsize,
};
