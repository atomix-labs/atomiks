//! A crate under the workspace's lints, `unsafe_code` forbidden, with `const_trait_impl`, derives
//! `Atom` for generic types: a newtype, a struct of several fields, and enums with fields,
//! niche-filling and tagged; and stores an instance of each in a static.
//!
//! trybuild runs rustc alone, so the clippy lints here hold only where clippy builds the same code,
//! as it does in `tests/derive_generic.rs`.

#![feature(const_trait_impl)]
#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    single_use_lifetimes,
    unreachable_pub,
    unused,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::absolute_paths,
    clippy::allow_attributes,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::indexing_slicing,
    clippy::missing_docs_in_private_items,
    clippy::multiple_unsafe_ops_per_block,
    clippy::std_instead_of_core,
    clippy::undocumented_unsafe_blocks,
    clippy::wildcard_enum_match_arm
)]

use core::num::NonZero;

use atomiks::ordering::{Acquire, Relaxed};
use atomiks::{Atom, Atomic};

/// A value of any atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
pub struct Wrap<T>(T);

/// Two values of any atoms, packed in a `u64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
pub struct Pair<A, B> {
    /// The low one.
    first: A,
    /// The one above it.
    second: B,
}

/// A lock, free or held by its owner: `Free` fills a niche beside the owner's reprs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
pub enum Lock<O> {
    /// Free to take.
    Free,
    /// Held by its owner.
    Held(O),
}

/// A ring buffer's slot, tagged by the discriminants it states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
#[atom(repr = u64)]
#[repr(u8)]
pub enum Slot<T> {
    /// Nothing written.
    Empty      = 0,
    /// Being written.
    Writing(T) = 1,
    /// Written.
    Ready(T)   = 2,
}

/// The next sequence number.
pub static NEXT: Atomic<Wrap<u32>> = Atomic::new(Wrap(0));

/// The best price and whether it is live.
pub static BEST: Atomic<Pair<u32, bool>> = Atomic::new(Pair { first: 0, second: false });

/// The lock's owner, or `Free`.
pub static LOCK: Atomic<Lock<NonZero<u32>>> = Atomic::new(Lock::Free);

/// The last slot written, or `None` before the first.
pub static SLOT: Atomic<Option<Slot<u32>>> = Atomic::new(None);

fn main() {
    assert_eq!(NEXT.swap(Wrap(1), Relaxed), Wrap(0), "one taken");
    BEST.store(Pair { first: 7, second: true }, Relaxed);
    assert_eq!(BEST.load(Acquire), Pair { first: 7, second: true }, "the best price");
    assert_eq!(LOCK.load(Acquire), Lock::Free, "free");
    assert_eq!(Lock::<NonZero<u32>>::Free.to_repr(), 0, "at zero, which no owner is");
    SLOT.store(Some(Slot::Writing(3)), Relaxed);
    assert_eq!(SLOT.load(Acquire), Some(Slot::Writing(3)), "and a slot");
}
