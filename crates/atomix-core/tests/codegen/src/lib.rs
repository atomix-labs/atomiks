//! One operation per function, unmangled, for `tests/codegen.rs` to read the assembly of.
//!
//! The operations only `aarch64` has build there, or anywhere with the `aarch64-only` feature, for
//! the test that `x86_64` refuses each; the `aarch64-refused` feature adds a probe of each
//! capability `aarch64` Linux's floor lacks for 128 bits (load, store, exchange, maximum), of a
//! `u128` and of a double word, for the tests that it refuses them and that macOS's floor, with
//! LSE2, refuses the exchange and maximum; and the `x86-64-refused` feature adds a pair of
//! pointers and a slice's pointer, for the test that `x86_64` without `cmpxchg16b` refuses them.
//! atomix-core's `deranged-05` feature is always on, for the probes of its conversions, and its
//! `arbitrary-int` feature, for those of a field of an arbitrary-int integer. The field operations'
//! probes are in `fields`, over packed structs `tests/testing/packed.rs` writes out, a pointer
//! word's in `pointer_words`, over words `tests/testing/pointer_word.rs` writes out, and a double
//! word's, a pair's and a slice pointer's, in `double_words`.
//!
//! The empty `[workspace]` in its manifest makes it a workspace of its own: the repository's does
//! not list it, and the test builds it alone.

#![no_std]
#![feature(const_trait_impl)]

#[cfg(any(target_arch = "aarch64", target_feature = "cmpxchg16b"))]
pub mod double_words;
pub mod fields;
#[path = "../../testing/packed.rs"]
mod packed;
#[path = "../../testing/pointer_word.rs"]
mod pointer_word;
pub mod pointer_words;
#[cfg(feature = "x86-64-refused")]
pub mod without_cmpxchg16b;

use core::cell::Cell;
use core::num::NonZero;

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
use atomix_core::AtomicI64;
#[cfg(any(target_arch = "aarch64", target_feature = "cmpxchg16b"))]
use atomix_core::AtomicU128;
use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst, StoreStore};
use atomix_core::{
    Atomic, AtomicBool, AtomicPtr, AtomicU64, RangedI16, RangedU32, RangedU64, compiler_fence,
    fence,
};
use deranged::{RangedI16 as DerangedI16, RangedU32 as DerangedU32};

#[unsafe(no_mangle)]
pub fn u64_load(atomic: &AtomicU64) -> u64 {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn u64_store(atomic: &AtomicU64, value: u64) {
    atomic.store(value, Release);
}

#[unsafe(no_mangle)]
pub fn u64_swap(atomic: &AtomicU64, value: u64) -> u64 {
    atomic.swap(value, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_fetch_add(atomic: &AtomicU64, delta: u64) -> u64 {
    atomic.fetch_add(delta, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_fetch_sub(atomic: &AtomicU64, delta: u64) -> u64 {
    atomic.fetch_sub(delta, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_compare_exchange(atomic: &AtomicU64, current: u64, new: u64) -> Result<u64, u64> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[unsafe(no_mangle)]
pub fn u64_fetch_add_discarded(atomic: &AtomicU64, delta: u64) {
    atomic.fetch_add(delta, Relaxed);
}

#[unsafe(no_mangle)]
pub fn u64_fetch_sub_discarded(atomic: &AtomicU64, delta: u64) {
    atomic.fetch_sub(delta, Relaxed);
}

#[unsafe(no_mangle)]
pub fn u64_or(atomic: &AtomicU64, value: u64) {
    atomic.or(value, Release);
}

#[unsafe(no_mangle)]
pub fn u64_and(atomic: &AtomicU64, value: u64) {
    atomic.and(value, Release);
}

#[unsafe(no_mangle)]
pub fn u64_xor(atomic: &AtomicU64, value: u64) {
    atomic.xor(value, Release);
}

#[unsafe(no_mangle)]
pub fn u64_not(atomic: &AtomicU64) {
    atomic.not(Release);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_and(atomic: &AtomicU64, value: u64) -> u64 {
    atomic.fetch_and(value, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_or(atomic: &AtomicU64, value: u64) -> u64 {
    atomic.fetch_or(value, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_xor(atomic: &AtomicU64, value: u64) -> u64 {
    atomic.fetch_xor(value, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_not(atomic: &AtomicU64) -> u64 {
    atomic.fetch_not(AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_max_discarded(atomic: &AtomicU64, value: u64) {
    atomic.fetch_max(value, Relaxed);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_min_discarded(atomic: &AtomicU64, value: u64) {
    atomic.fetch_min(value, Relaxed);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_max(atomic: &AtomicU64, value: u64) -> u64 {
    atomic.fetch_max(value, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn i64_fetch_max_discarded(atomic: &AtomicI64, value: i64) {
    atomic.fetch_max(value, Relaxed);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn i64_fetch_min(atomic: &AtomicI64, value: i64) -> i64 {
    atomic.fetch_min(value, AcqRel)
}

#[unsafe(no_mangle)]
pub fn bool_or(atomic: &AtomicBool, value: bool) {
    atomic.or(value, Release);
}

#[unsafe(no_mangle)]
pub fn ptr_fetch_byte_add_discarded(atomic: &AtomicPtr<u64>, bytes: usize) {
    atomic.fetch_byte_add(bytes, Relaxed);
}

#[unsafe(no_mangle)]
pub fn ptr_fetch_ptr_sub(atomic: &AtomicPtr<u64>, count: usize) -> *mut u64 {
    atomic.fetch_ptr_sub(count, AcqRel)
}

#[unsafe(no_mangle)]
pub fn char_load(atomic: &Atomic<char>) -> char {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn option_load(atomic: &Atomic<Option<NonZero<u64>>>) -> Option<NonZero<u64>> {
    atomic.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn ranged_load(atomic: &Atomic<RangedU64<3>>) -> RangedU64<3> {
    atomic.load(Acquire)
}

/// Whether a value from 3 lies below 3: never, as LLVM knows from the pattern type's range, and
/// must still know should a plain integer replace the pattern type.
#[unsafe(no_mangle)]
pub fn ranged_below_min(value: RangedU64<3>) -> bool {
    value.get() < 3
}

/// Converts to deranged's ranged integer of the same bounds: the saturation never moves the
/// integer, which LLVM knows from the range `get` tells it.
#[unsafe(no_mangle)]
pub fn ranged_to_deranged(value: RangedU32<3, 100>) -> DerangedU32<3, 100> {
    value.into()
}

/// Converts back from deranged's, for a signed integer.
#[unsafe(no_mangle)]
pub fn ranged_from_deranged(value: DerangedI16<-5, 5>) -> RangedI16<-5, 5> {
    value.into()
}

#[cfg(any(target_feature = "avx", target_feature = "lse2", feature = "aarch64-refused"))]
#[unsafe(no_mangle)]
pub fn u128_load(atomic: &AtomicU128) -> u128 {
    atomic.load(Acquire)
}

#[cfg(any(target_feature = "avx", target_feature = "lse2"))]
#[unsafe(no_mangle)]
pub fn u128_load_relaxed(atomic: &AtomicU128) -> u128 {
    atomic.load(Relaxed)
}

#[cfg(any(target_feature = "avx", target_feature = "lse2"))]
#[unsafe(no_mangle)]
pub fn u128_load_seq_cst(atomic: &AtomicU128) -> u128 {
    atomic.load(SeqCst)
}

#[cfg(any(target_feature = "avx", target_feature = "lse2", feature = "aarch64-refused"))]
#[unsafe(no_mangle)]
pub fn u128_store(atomic: &AtomicU128, value: u128) {
    atomic.store(value, Release);
}

#[cfg(any(target_feature = "avx", target_feature = "lse2"))]
#[unsafe(no_mangle)]
pub fn u128_store_seq_cst(atomic: &AtomicU128, value: u128) {
    atomic.store(value, SeqCst);
}

#[cfg(any(target_arch = "aarch64", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn u128_compare_exchange(atomic: &AtomicU128, current: u128, new: u128) -> Result<u128, u128> {
    atomic.compare_exchange(current, new, AcqRel, Acquire)
}

#[cfg(any(target_arch = "aarch64", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn u128_load_rmw(atomic: &AtomicU128) -> u128 {
    atomic.load_rmw(Acquire)
}

#[cfg(any(target_arch = "aarch64", target_feature = "cmpxchg16b"))]
#[unsafe(no_mangle)]
pub fn u128_update(atomic: &AtomicU128) -> u128 {
    atomic.update(AcqRel, Acquire, |value| value.wrapping_add(1))
}

#[cfg(feature = "aarch64-refused")]
#[unsafe(no_mangle)]
pub fn u128_swap(atomic: &AtomicU128, value: u128) -> u128 {
    atomic.swap(value, AcqRel)
}

#[cfg(feature = "aarch64-refused")]
#[unsafe(no_mangle)]
pub fn u128_fetch_max(atomic: &AtomicU128, value: u128) -> u128 {
    atomic.fetch_max(value, AcqRel)
}

/// Two writes to a shared place around the fence: without `nomem`, the barrier keeps both.
#[unsafe(no_mangle)]
pub fn store_store_fence(cell: &Cell<u64>) {
    cell.set(1);
    fence(StoreStore);
    cell.set(2);
}

#[unsafe(no_mangle)]
pub fn seq_cst_compiler_fence() {
    compiler_fence(SeqCst);
}

#[unsafe(no_mangle)]
pub fn acquire_fence() {
    fence(Acquire);
}

#[unsafe(no_mangle)]
pub fn release_fence() {
    fence(Release);
}

#[unsafe(no_mangle)]
pub fn seq_cst_fence() {
    fence(SeqCst);
}
