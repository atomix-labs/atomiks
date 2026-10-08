//! The typed atomic of atomix, the values it holds, and the orderings and primitives under it.
//!
//! [`Atomic<T>`](Atomic) holds a `T` as its [`Repr`](Atom::Repr), one primitive an atomic
//! instruction reads and writes, and decodes it on the way out.
//!
//! An atomic packed struct lends each of its fields as an [`AtomicField`], which one instruction
//! on the whole word changes alone: `QUOTE.fields().live.set(Release)` sets the bit `live` of a
//! derived `Quote` with `lock or`, or `ldset`. [`Atomic::fields`] lends them, through the
//! [`ProjectFields`] atomix's derive implements for the struct.
//!
//! A pointer keeps small fields as tags in the low bits its pointee's alignment leaves clear, and
//! keeps its provenance through every operation: `HEAD.fields().closed.test_and_set(AcqRel)`
//! closes a derived Treiber stack's `Head` with `lock bts`, or `ldsetal`, on its top node's
//! pointer. [`PtrAtom`] is any value stored as a pointer.
//!
//! Two pointers, or one beside a counter or a slice's length, share a 16-byte atomic, a
//! [`DoubleWord`], which exposes each pointer's provenance, as no Rust operation keeps one through
//! 16 bytes: `HEAD.compare_exchange(pushed, popped, AcqRel, Acquire)` pops a derived Treiber
//! stack's `Head`, a top node beside a `u64` version, with `lock cmpxchg16b`, or `caspal`.
//!
//! atomix-core needs a nightly Rust, and builds for `aarch64` and `x86_64`, little-endian with
//! 64-bit pointers, on any OS: [atomix's Platforms][platforms] say which nightly, and what
//! each target has.
//!
//! # Examples
//! ```
//! use core::num::NonZero;
//!
//! use atomix_core::ordering::{Acquire, Relaxed, Release};
//! use atomix_core::{Atomic, AtomicU64};
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
//! None is on by default. Under `--cfg loom`, whose cells are not plain memory, an atomic has
//! neither zerocopy's traits nor `Zeroable`.
//!
//! | Feature         | Adds                                                                       |
//! | --------------- | -------------------------------------------------------------------------- |
//! | `serde`         | serde's traits for an atomic, and a ranged integer, held to its range      |
//! | `zerocopy-08`   | zerocopy's traits for an atomic, as its validity allows, never `Immutable` |
//! | `bytemuck`      | `Zeroable` for an atomic, and the bit-pattern traits for a ranged integer  |
//! | `arbitrary`     | `Arbitrary` for an atomic, and a ranged integer, held to its range         |
//! | `arbitrary-int` | `Atom`, `AtomOrd`, `FieldBitwise` and `FieldAdd` for `UInt` and `Int`      |
//! | `deranged-05`   | `Atom`, `AtomOrd` and `From` with atomix's own for deranged 0.5's integers |
//! | `loom`          | loom's types under `--cfg loom`; nothing without the cfg                   |
//!
//! [platforms]: https://docs.rs/atomix-rs/latest/atomix/#platforms

// Where no atomic holds two words, no `DoubleWord` exists: its links lead to where one does.
#![cfg_attr(
    not(wide),
    doc = "",
    doc = "[`DoubleWord`]: https://docs.rs/atomix-rs/latest/atomix/#platforms"
)]
#![no_std]
#![feature(
    associated_type_defaults,
    const_convert,
    const_destruct,
    const_eval_select,
    const_index,
    const_option_ops,
    const_trait_impl,
    const_type_name,
    core_intrinsics,
    doc_cfg,
    f16,
    generic_pattern_types,
    impl_restriction,
    integer_casts,
    pattern_type_macro,
    pattern_types,
    ptr_mask,
    ptr_metadata,
    step_trait,
    structural_match,
    transmute_neo
)]
#![cfg_attr(not(loom), feature(const_atomic, const_default))]
#![cfg_attr(wide, feature(f128))]
#![cfg_attr(core_atomic_u128, feature(integer_atomics))]
#![expect(
    internal_features,
    reason = "a ranged integer's field is a pattern type, with AVX the 16-byte load and store are \
              core's atomic intrinsics, and a pointer's address is read, and its tags cleared, \
              in a constant through `const_eval_select`"
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
    "atomix: `--cfg loom` needs the `loom` feature: depend with `[target.'cfg(loom)'.dependencies] \
     atomix-rs = { version = \"",
    env!("CARGO_PKG_VERSION"),
    "\", features = [\"loom\"] }`, or pass `--features atomix-rs/loom`, or `atomix-core/loom` \
     where atomix-core is the dependency"
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

#[cfg(wide)]
#[doc(cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
)))]
pub use crate::atom::VtablePointer;
pub use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd, FieldAdd, FieldBitwise, PtrAtom};
pub use crate::atomic::{
    Atomic, AtomicBool, AtomicField, AtomicI8, AtomicI16, AtomicI32, AtomicI64, AtomicIsize,
    AtomicPtr, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize, Field, FieldPath, Join,
    ProjectFields, Then, TopField, Whole,
};
#[cfg(wide)]
#[doc(cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
)))]
pub use crate::atomic::{AtomicI128, AtomicU128};
pub use crate::errors::{ParseRangeError, RangeError};
pub use crate::fence::{compiler_fence, fence};
#[cfg(wide)]
#[doc(cfg(any(
    target_arch = "aarch64",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
)))]
pub use crate::primitive::DoubleWord;
pub use crate::primitive::{
    BitTest, ExactBits, FetchAdd, FetchBitwise, Load, MaskBitwise, MinMax, Primitive, RawAccess,
    ReadByExchange, Store, Swap,
};
pub use crate::range::ReprRange;
pub use crate::ranged::{
    RangedI8, RangedI16, RangedI32, RangedI64, RangedI128, RangedIsize, RangedU8, RangedU16,
    RangedU32, RangedU64, RangedU128, RangedUsize,
};
