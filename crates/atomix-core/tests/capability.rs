//! The read-modify-writes a capability allows, on the values that have it.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::num::Wrapping;

    #[cfg(target_arch = "aarch64")]
    use atomix_core::AtomicI64;
    use atomix_core::ordering::{AcqRel, Relaxed};
    use atomix_core::{
        Atomic, AtomicBool, AtomicI8, AtomicI16, AtomicU8, AtomicU16, AtomicU32, AtomicU64,
    };

    #[test]
    fn add_and_sub_wrap() {
        let count = AtomicU8::new(255);
        assert_eq!(count.fetch_add(1, AcqRel), 255, "the value before");
        assert_eq!(count.load(Relaxed), 0, "wrapped");
        assert_eq!(count.fetch_sub(1, AcqRel), 0, "the value before");
        assert_eq!(count.load(Relaxed), 255, "wrapped back");
        count.fetch_add(2, Relaxed);
        assert_eq!(count.load(Relaxed), 1, "up two, wrapping");
        count.fetch_sub(3, Relaxed);
        assert_eq!(count.load(Relaxed), 254, "down three, wrapping");
    }

    #[test]
    fn signed_add_and_sub_wrap_at_the_signed_bounds() {
        let level = AtomicI8::new(i8::MAX);
        assert_eq!(level.fetch_add(1, AcqRel), i8::MAX, "the value before");
        assert_eq!(level.load(Relaxed), i8::MIN, "past the greatest, the least");
        level.fetch_sub(1, Relaxed);
        assert_eq!(level.load(Relaxed), i8::MAX, "below the least, the greatest");
        assert_eq!(level.fetch_sub(-2, AcqRel), i8::MAX, "the value before");
        assert_eq!(level.load(Relaxed), -127, "less -2 is 2 more, wrapping");
    }

    #[test]
    fn bitwise_operations_combine_bits() {
        let bits = AtomicU32::new(0b1100);
        bits.or(0b1010, AcqRel);
        assert_eq!(bits.load(Relaxed), 0b1110, "or keeps the bits either has");
        bits.and(0b0111, AcqRel);
        assert_eq!(bits.load(Relaxed), 0b0110, "and keeps the bits both have");
        bits.xor(0b0101, AcqRel);
        assert_eq!(bits.load(Relaxed), 0b0011, "xor keeps the bits one has");
        bits.not(AcqRel);
        assert_eq!(bits.load(Relaxed), !0b0011, "every bit inverted");
    }

    #[test]
    fn a_bool_flips_and_combines() {
        let flag = AtomicBool::new(false);
        flag.not(AcqRel);
        assert!(flag.load(Relaxed), "not flips false");
        flag.or(false, AcqRel);
        assert!(flag.load(Relaxed), "or with false keeps true");
        flag.xor(true, AcqRel);
        assert!(!flag.load(Relaxed), "xor with true flips true");
        flag.and(true, AcqRel);
        assert!(!flag.load(Relaxed), "and with true keeps false");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn max_and_min_follow_the_values_sign() {
        let signed = AtomicI64::new(-5);
        assert_eq!(signed.fetch_max(3, AcqRel), -5, "the value before");
        assert_eq!(signed.load(Relaxed), 3, "3 is the larger signed value");
        signed.fetch_min(-7, Relaxed);
        assert_eq!(signed.load(Relaxed), -7, "-7 is the smaller signed value");
        let unsigned = AtomicU64::new(7);
        assert_eq!(unsigned.fetch_min(2, AcqRel), 7, "the value before");
        assert_eq!(unsigned.load(Relaxed), 2, "2 is the smaller");
        unsigned.fetch_max(u64::MAX, Relaxed);
        assert_eq!(unsigned.load(Relaxed), u64::MAX, "the top bit set is the larger, unsigned");
        let narrow = AtomicI8::new(-1);
        narrow.fetch_max(1, Relaxed);
        assert_eq!(narrow.load(Relaxed), 1, "1 is the larger signed value; unsigned, -1 is 0xFF");
        assert_eq!(narrow.fetch_min(-1, AcqRel), 1, "the value before");
        assert_eq!(narrow.load(Relaxed), -1, "-1 is the smaller signed value");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn the_fetch_forms_return_the_value_before() {
        let bits = AtomicU32::new(0b1100);
        assert_eq!(bits.fetch_or(0b1010, AcqRel), 0b1100, "before or");
        assert_eq!(bits.fetch_and(0b0111, AcqRel), 0b1110, "before and");
        assert_eq!(bits.fetch_xor(0b0101, AcqRel), 0b0110, "before xor");
        assert_eq!(bits.fetch_not(AcqRel), 0b0011, "before not");
        assert_eq!(bits.load(Relaxed), !0b0011, "after not");
        let flag = AtomicBool::new(false);
        assert!(!flag.fetch_or(true, AcqRel), "before or");
        assert!(flag.load(Relaxed), "after or");
    }

    #[test]
    fn a_bit_chosen_at_run_time_is_set_cleared_and_inverted_alone() {
        for bit in 0..u64::BITS {
            let mask = 1_u64 << bit;
            let bits = AtomicU64::new(0xA5A5_A5A5_A5A5_A5A5 & !mask);
            let others = bits.load(Relaxed);
            assert!(!bits.bit_set(bit, AcqRel), "{bit}: clear before the set");
            assert!(bits.bit_set(bit, AcqRel), "{bit}: set before the second set");
            assert_eq!(bits.load(Relaxed), others | mask, "{bit}: set, every other bit kept");
            assert!(bits.bit_toggle(bit, AcqRel), "{bit}: set before the toggle");
            assert!(!bits.bit_toggle(bit, AcqRel), "{bit}: clear before the toggle back");
            assert!(bits.bit_clear(bit, AcqRel), "{bit}: set before the clear");
            assert!(!bits.bit_clear(bit, AcqRel), "{bit}: clear before the second clear");
            assert_eq!(bits.load(Relaxed), others, "{bit}: clear again, every other bit kept");
        }
    }

    #[test]
    fn a_bit_counts_modulo_the_reprs_width() {
        let bits = AtomicU16::new(0);
        assert!(!bits.bit_set(16 + 3, AcqRel), "bit 19 of 16 is bit 3, clear");
        assert_eq!(bits.load(Relaxed), 0b1000, "bit 3 set");
        assert!(bits.bit_clear(u32::MAX - 12, AcqRel), "bit 2^32 - 13 of 16 is bit 3, set");
        assert_eq!(bits.load(Relaxed), 0, "bit 3 clear");
        assert!(!bits.bit_toggle(16, AcqRel), "bit 16 of 16, one past the top, is bit 0, clear");
        assert_eq!(bits.load(Relaxed), 0b1, "bit 0 set");
    }

    #[test]
    fn a_signed_values_top_bit_is_its_sign() {
        let level = AtomicI16::new(1);
        assert!(!level.bit_set(15, AcqRel), "a positive value's sign bit is clear");
        assert_eq!(level.load(Relaxed), i16::MIN + 1, "negative once set");
    }

    #[test]
    fn a_wrapping_value_takes_its_integers_bits() {
        let bits = Atomic::new(Wrapping(0_u32));
        assert!(!bits.bit_set(31, AcqRel), "clear before the set");
        assert_eq!(bits.load(Relaxed), Wrapping(1 << 31), "the top bit set");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn an_8_bit_value_has_its_bits_on_aarch64() {
        let bits = AtomicU8::new(0);
        assert!(!bits.bit_set(7, AcqRel), "clear before the set");
        assert_eq!(bits.load(Relaxed), 0x80, "the top bit set");
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn a_bools_one_bit_is_every_position_on_aarch64() {
        let flag = AtomicBool::new(false);
        assert!(!flag.bit_set(9, AcqRel), "clear before the set, at any position");
        assert!(flag.load(Relaxed), "and set after");
    }
}
