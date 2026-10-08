//! One field operation per function, over packed structs written out as the derive writes them:
//! each of a `bool`, a bitwise field, a field of a field and the field at the top bit, and a bit's
//! test at each end and at the middle of 8, 16, 32 and 64 bits; and the add of an arbitrary-int
//! `u61` at the top, and the or of a `u4`.

use core::num::NonZero;

use arbitrary_int::{u4, u12, u61};
use atomix_core::ordering::{AcqRel, Acquire, Relaxed, Release};
use atomix_core::{Atomic, RangedU8, RangedU16, RangedU32};

use crate::packed::packed;

packed! {
    /// 32 bits of quantity, an owner's 8, a bit of whether it is live (bit 40), then flags.
    pub struct Order in u64, projected as OrderFields {
        0 => quantity: u32,
        1 => owner: NonZero<u8>,
        2 => live: bool,
        3 => flags: u8,
    }
}

packed! {
    /// Flags and a bit, then a count in the top half.
    pub struct Counted in u64, projected as CountedFields {
        0 => flags: u8,
        1 => ready: bool,
        2 => spare: RangedU32<0, 0x7F_FFFF>,
        3 => count: u32,
    }
}

packed! {
    /// A packed struct a field of another holds: 9 bits.
    pub struct Inner in u16, projected as InnerFields {
        0 => ready: bool,
        1 => flags: u8,
    }
}

packed! {
    /// A byte, the inner struct at bit 8, then a bit.
    pub struct Outer in u32, projected as OuterFields {
        0 => low: u8,
        1 => inner: Inner,
        2 => done: bool,
    }
}

packed! {
    /// A bit at each end of a `u8`.
    pub struct Ends8 in u8, projected as Ends8Fields {
        0 => low: bool,
        1 => between: RangedU8<0, 0x3F>,
        2 => top: bool,
    }
}

packed! {
    /// A bit at each end of a `u16`, and one at bit 8.
    pub struct Ends16 in u16, projected as Ends16Fields {
        0 => low: bool,
        1 => below: RangedU8<0, 0x7F>,
        2 => middle: bool,
        3 => above: RangedU8<0, 0x3F>,
        4 => top: bool,
    }
}

packed! {
    /// A bit at each end of a `u32`, and one at bit 16.
    pub struct Ends32 in u32, projected as Ends32Fields {
        0 => low: bool,
        1 => below: RangedU16<0, 0x7FFF>,
        2 => middle: bool,
        3 => above: RangedU16<0, 0x3FFF>,
        4 => top: bool,
    }
}

packed! {
    /// A bit at each end of a `u64`, and one at bit 32.
    pub struct Ends64 in u64, projected as Ends64Fields {
        0 => low: bool,
        1 => below: RangedU32<0, 0x7FFF_FFFF>,
        2 => middle: bool,
        3 => above: RangedU32<0, 0x3FFF_FFFF>,
        4 => top: bool,
    }
}

packed! {
    /// Three bits of a task's state, then a count of its references, filling the word.
    pub struct Task in u64, projected as TaskFields {
        0 => queued: bool,
        1 => running: bool,
        2 => done: bool,
        3 => references: u61,
    }
}

packed! {
    /// Four bits of flags, then a count, filling the word.
    pub struct Flagged in u16, projected as FlaggedFields {
        0 => flags: u4,
        1 => count: u12,
    }
}

#[unsafe(no_mangle)]
pub fn field_set(atomic: &Atomic<Order>) {
    atomic.fields().live.set(Release);
}

#[unsafe(no_mangle)]
pub fn field_clear(atomic: &Atomic<Order>) {
    atomic.fields().live.clear(Release);
}

#[unsafe(no_mangle)]
pub fn field_toggle(atomic: &Atomic<Order>) {
    atomic.fields().live.toggle(Release);
}

#[unsafe(no_mangle)]
pub fn field_store(atomic: &Atomic<Order>, value: bool) {
    atomic.fields().live.store(value, Release);
}

#[unsafe(no_mangle)]
pub fn field_test_and_set(atomic: &Atomic<Order>) -> bool {
    atomic.fields().live.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn field_load(atomic: &Atomic<Order>) -> bool {
    atomic.fields().live.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn flags_load(atomic: &Atomic<Order>) -> u8 {
    atomic.fields().flags.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn quantity_load(atomic: &Atomic<Order>) -> u32 {
    atomic.fields().quantity.load(Acquire)
}

#[unsafe(no_mangle)]
pub fn quantity_update(atomic: &Atomic<Order>) -> Order {
    atomic.fields().quantity.update(AcqRel, Acquire, |quantity| quantity.wrapping_mul(3))
}

#[unsafe(no_mangle)]
pub fn flags_or(atomic: &Atomic<Order>, flags: u8) {
    atomic.fields().flags.or(flags, Release);
}

#[unsafe(no_mangle)]
pub fn flags_and(atomic: &Atomic<Order>, flags: u8) {
    atomic.fields().flags.and(flags, Release);
}

#[unsafe(no_mangle)]
pub fn flags_xor(atomic: &Atomic<Order>, flags: u8) {
    atomic.fields().flags.xor(flags, Release);
}

#[unsafe(no_mangle)]
pub fn flags_not(atomic: &Atomic<Order>) {
    atomic.fields().flags.not(Release);
}

#[unsafe(no_mangle)]
pub fn flags_or_constant(atomic: &Atomic<Order>) {
    atomic.fields().flags.or(0b100, Release);
}

#[unsafe(no_mangle)]
pub fn nested_clear(atomic: &Atomic<Outer>) {
    atomic.fields().inner.fields().ready.clear(Release);
}

#[unsafe(no_mangle)]
pub fn nested_flags_or(atomic: &Atomic<Outer>, flags: u8) {
    atomic.fields().inner.fields().flags.or(flags, Release);
}

#[unsafe(no_mangle)]
pub fn top_fetch_add(atomic: &Atomic<Counted>, delta: u32) -> Counted {
    atomic.fields().count.fetch_add(delta, AcqRel)
}

#[unsafe(no_mangle)]
pub fn top_fetch_sub(atomic: &Atomic<Counted>, delta: u32) -> Counted {
    atomic.fields().count.fetch_sub(delta, AcqRel)
}

#[unsafe(no_mangle)]
pub fn top_fetch_add_discarded(atomic: &Atomic<Counted>, delta: u32) {
    atomic.fields().count.fetch_add(delta, Relaxed);
}

#[unsafe(no_mangle)]
pub fn top_fetch_add_count(atomic: &Atomic<Counted>) -> u32 {
    atomic.fields().count.fetch_add(1, AcqRel).count
}

#[unsafe(no_mangle)]
pub fn u61_top_fetch_add_references(atomic: &Atomic<Task>) -> u61 {
    atomic.fields().references.fetch_add(1, AcqRel).references
}

#[unsafe(no_mangle)]
pub fn u4_flags_or(atomic: &Atomic<Flagged>, flags: u4) {
    atomic.fields().flags.or(flags, Release);
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn field_fetch_or(atomic: &Atomic<Order>) -> Order {
    atomic.fields().live.fetch_or(true, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn flags_fetch_and(atomic: &Atomic<Order>, flags: u8) -> Order {
    atomic.fields().flags.fetch_and(flags, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn flags_fetch_or(atomic: &Atomic<Order>, flags: u8) -> Order {
    atomic.fields().flags.fetch_or(flags, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn flags_fetch_xor(atomic: &Atomic<Order>, flags: u8) -> Order {
    atomic.fields().flags.fetch_xor(flags, AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn flags_fetch_not(atomic: &Atomic<Order>) -> Order {
    atomic.fields().flags.fetch_not(AcqRel)
}

// `x86_64` has no 8-bit `lock bts`.
#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn ends8_low_test_and_set(atomic: &Atomic<Ends8>) -> bool {
    atomic.fields().low.test_and_set(AcqRel)
}

#[cfg(any(target_arch = "aarch64", feature = "aarch64-only"))]
#[unsafe(no_mangle)]
pub fn ends8_top_test_and_set(atomic: &Atomic<Ends8>) -> bool {
    atomic.fields().top.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends16_low_test_and_set(atomic: &Atomic<Ends16>) -> bool {
    atomic.fields().low.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends16_middle_test_and_set(atomic: &Atomic<Ends16>) -> bool {
    atomic.fields().middle.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends16_top_test_and_set(atomic: &Atomic<Ends16>) -> bool {
    atomic.fields().top.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends32_low_test_and_set(atomic: &Atomic<Ends32>) -> bool {
    atomic.fields().low.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends32_middle_test_and_set(atomic: &Atomic<Ends32>) -> bool {
    atomic.fields().middle.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends32_top_test_and_set(atomic: &Atomic<Ends32>) -> bool {
    atomic.fields().top.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_low_test_and_set(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().low.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_middle_test_and_set(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().middle.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_middle_test_and_clear(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().middle.test_and_clear(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_middle_test_and_toggle(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().middle.test_and_toggle(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_top_test_and_set(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().top.test_and_set(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_top_test_and_clear(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().top.test_and_clear(AcqRel)
}

#[unsafe(no_mangle)]
pub fn ends64_top_test_and_toggle(atomic: &Atomic<Ends64>) -> bool {
    atomic.fields().top.test_and_toggle(AcqRel)
}
