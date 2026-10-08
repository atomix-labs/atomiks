//! A double word's cell holds each pointer's address with its provenance exposed, and no constant
//! exposes a provenance, so a static builds a double word of pointers made of integers alone, null
//! or a null with tags set: a pair of pointers to a static is refused at the static, where the
//! constant uses its first pointer's address, which no constant knows.

use core::ptr::NonNull;

use atomix_core::Atomic;

/// A node the pair points to.
static NODE: u64 = 7;

/// Two pointers to `NODE`, whose provenance no constant exposes.
static PAIR: Atomic<(NonNull<u64>, NonNull<u64>)> =
    Atomic::new((NonNull::from_ref(&NODE), NonNull::from_ref(&NODE)));

fn main() {
    let _ = &PAIR;
}
