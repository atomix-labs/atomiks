//! A bit chosen at run time, or by a constant at either end, in the middle or past the width, set,
//! cleared or inverted, its value before returned, discarded, or put in an `Option`, where LLVM's
//! own test of a `fetch_or` would be a compare-exchange loop.

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", feature = "aarch64-only"))]
use atomix_core::AtomicU8;
use atomix_core::ordering::AcqRel;
use atomix_core::{AtomicI32, AtomicI64, AtomicIsize, AtomicU16, AtomicU32, AtomicU64};

#[unsafe(no_mangle)]
pub fn u64_bit_set(atomic: &AtomicU64, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_clear(atomic: &AtomicU64, bit: u32) -> bool {
    atomic.bit_clear(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_toggle(atomic: &AtomicU64, bit: u32) -> bool {
    atomic.bit_toggle(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_0(atomic: &AtomicU64) -> bool {
    atomic.bit_set(0, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_5(atomic: &AtomicU64) -> bool {
    atomic.bit_set(5, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_63(atomic: &AtomicU64) -> bool {
    atomic.bit_set(63, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_clear_63(atomic: &AtomicU64) -> bool {
    atomic.bit_clear(63, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_64(atomic: &AtomicU64) -> bool {
    atomic.bit_set(64, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_discarded(atomic: &AtomicU64, bit: u32) {
    let _ = atomic.bit_set(bit, AcqRel);
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_in_some(atomic: &AtomicU64, bit: u32) -> Option<bool> {
    Some(atomic.bit_set(bit, AcqRel))
}

#[unsafe(no_mangle)]
pub fn u64_bit_clear_in_some(atomic: &AtomicU64, bit: u32) -> Option<bool> {
    Some(atomic.bit_clear(bit, AcqRel))
}

#[unsafe(no_mangle)]
pub fn u64_bit_toggle_in_some(atomic: &AtomicU64, bit: u32) -> Option<bool> {
    Some(atomic.bit_toggle(bit, AcqRel))
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_63_in_some(atomic: &AtomicU64) -> Option<bool> {
    Some(atomic.bit_set(63, AcqRel))
}

#[unsafe(no_mangle)]
pub fn u64_bit_set_through_map(atomic: &AtomicU64, bit: Option<u32>) -> Option<bool> {
    bit.map(|bit| atomic.bit_set(bit, AcqRel))
}

#[unsafe(no_mangle)]
pub fn u32_bit_set(atomic: &AtomicU32, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u32_bit_set_31(atomic: &AtomicU32) -> bool {
    atomic.bit_set(31, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u16_bit_set(atomic: &AtomicU16, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn u16_bit_set_in_some(atomic: &AtomicU16, bit: u32) -> Option<bool> {
    Some(atomic.bit_set(bit, AcqRel))
}

#[unsafe(no_mangle)]
pub fn i32_bit_set(atomic: &AtomicI32, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn i64_bit_set(atomic: &AtomicI64, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}

#[unsafe(no_mangle)]
pub fn isize_bit_set(atomic: &AtomicIsize, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn u8_bit_set(atomic: &AtomicU8, bit: u32) -> bool {
    atomic.bit_set(bit, AcqRel)
}
