//! Two words where no atomic holds them, `x86_64` without `cmpxchg16b`: a pair of pointers' and a
//! slice pointer's exchange, which the `x86-64-refused` feature adds, each refused as a 128-bit
//! value is, naming the CPU it needs.

use core::ptr::NonNull;

use atomiks_core::Atomic;
use atomiks_core::ordering::{AcqRel, Acquire};

/// A node a pair points to.
pub struct Node {
    /// What it holds.
    pub value: u64,
}

/// Two pointers to nodes.
pub type Pair = (NonNull<Node>, NonNull<Node>);

#[unsafe(no_mangle)]
pub fn pair_compare_exchange_without_cmpxchg16b(
    atomic: &Atomic<Pair>, current: Pair, new: Pair,
) -> Result<Pair, Pair> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[unsafe(no_mangle)]
pub fn slice_compare_exchange_without_cmpxchg16b(
    atomic: &Atomic<NonNull<[u64]>>, current: NonNull<[u64]>, new: NonNull<[u64]>,
) -> Result<NonNull<[u64]>, NonNull<[u64]>> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}
