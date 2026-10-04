//! A crate under the workspace's lints, `unsafe_code` forbidden, with `const_trait_impl`, derives
//! `Atom` for generic types, a newtype and a struct of several fields, and stores an instance of
//! each in a static.
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

/// The next sequence number.
pub static NEXT: Atomic<Wrap<u32>> = Atomic::new(Wrap(0));

/// The best price and whether it is live.
pub static BEST: Atomic<Pair<u32, bool>> = Atomic::new(Pair { first: 0, second: false });

fn main() {
    assert_eq!(NEXT.swap(Wrap(1), Relaxed), Wrap(0), "one taken");
    BEST.store(Pair { first: 7, second: true }, Relaxed);
    assert_eq!(BEST.load(Acquire), Pair { first: 7, second: true }, "the best price");
}
