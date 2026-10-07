//! One double word operation per function, each one of core's 16-byte atomic instructions: a pair
//! of pointers' load and store where the target has a 16-byte one (LSE2, AVX), its read by a
//! compare-exchange everywhere, its exchange and update, and a slice pointer's exchange. The
//! `aarch64-refused` feature adds the load and store anywhere, and an exchange of the whole value,
//! which no floor runs without a loop, for the tests that `aarch64` refuses them.

use core::ptr::NonNull;

use atomiks_core::Atomic;
#[cfg(any(target_feature = "avx", target_feature = "lse2", feature = "aarch64-refused"))]
use atomiks_core::ordering::Release;
use atomiks_core::ordering::{AcqRel, Acquire};

/// A node a pair points to.
pub struct Node {
    /// What it holds.
    pub value: u64,
}

/// Two pointers to nodes.
pub type Pair = (NonNull<Node>, NonNull<Node>);

#[cfg(any(target_feature = "avx", target_feature = "lse2", feature = "aarch64-refused"))]
#[unsafe(no_mangle)]
pub fn pair_load(atomic: &Atomic<Pair>) -> Pair {
    atomic.load(Acquire)
}

#[cfg(any(target_feature = "avx", target_feature = "lse2", feature = "aarch64-refused"))]
#[unsafe(no_mangle)]
pub fn pair_store(atomic: &Atomic<Pair>, value: Pair) {
    atomic.store(value, Release);
}

#[cfg(feature = "aarch64-refused")]
#[unsafe(no_mangle)]
pub fn pair_swap(atomic: &Atomic<Pair>, value: Pair) -> Pair {
    atomic.swap(value, AcqRel)
}

#[unsafe(no_mangle)]
pub fn pair_load_rmw(atomic: &Atomic<Pair>) -> Pair {
    atomic.load_rmw(Acquire)
}

#[unsafe(no_mangle)]
pub fn pair_compare_exchange(
    atomic: &Atomic<Pair>, current: Pair, new: Pair,
) -> Result<Pair, Pair> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[unsafe(no_mangle)]
pub fn pair_update(atomic: &Atomic<Pair>) -> Pair {
    atomic.update(AcqRel, Acquire, |(first, second)| (second, first))
}

#[unsafe(no_mangle)]
pub fn slice_compare_exchange(
    atomic: &Atomic<NonNull<[u64]>>, current: NonNull<[u64]>, new: NonNull<[u64]>,
) -> Result<NonNull<[u64]>, NonNull<[u64]>> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}
