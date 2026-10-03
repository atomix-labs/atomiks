//! One operation per function, unmangled, for `tests/codegen.rs` to read the assembly of.
//!
//! The operations only `aarch64` has build there, or anywhere with the `aarch64-only` feature, for
//! the test that `x86_64` refuses each; the `aarch64-refused` feature adds a probe of each
//! capability `aarch64` Linux's floor lacks for 128 bits (load, store, exchange, maximum), for the
//! tests that it refuses them and that macOS's floor, with LSE2, refuses the exchange and maximum.
//!
//! The empty `[workspace]` in its manifest makes it a workspace of its own: the repository's does
//! not list it, and the test builds it alone.

#![no_std]

use core::cell::Cell;
use core::num::NonZero;

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
use atomiks_core::AtomicI64;
#[cfg(any(target_arch = "aarch64", target_feature = "cmpxchg16b"))]
use atomiks_core::AtomicU128;
use atomiks_core::ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst, StoreStore};
use atomiks_core::{Atomic, AtomicBool, AtomicPtr, AtomicU64, compiler_fence, fence};

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
pub fn u64_add(atomic: &AtomicU64, delta: u64) {
    atomic.add(delta, Relaxed);
}

#[unsafe(no_mangle)]
pub fn u64_sub(atomic: &AtomicU64, delta: u64) {
    atomic.sub(delta, Relaxed);
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
pub fn u64_max(atomic: &AtomicU64, value: u64) {
    atomic.max(value, Relaxed);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_min(atomic: &AtomicU64, value: u64) {
    atomic.min(value, Relaxed);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u64_fetch_max(atomic: &AtomicU64, value: u64) -> u64 {
    atomic.fetch_max(value, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn i64_max(atomic: &AtomicI64, value: i64) {
    atomic.max(value, Relaxed);
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
pub fn ptr_byte_add(atomic: &AtomicPtr<u64>, bytes: usize) {
    atomic.byte_add(bytes, Relaxed);
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
pub fn u128_max(atomic: &AtomicU128, value: u128) {
    atomic.max(value, Relaxed);
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
