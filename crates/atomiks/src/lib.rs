//! Typed atomics for any value that fits one atomic word.

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
