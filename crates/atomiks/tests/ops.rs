//! The read-modify-writes a capability allows, on the values that have it.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use atomiks::ordering::{AcqRel, Relaxed};
    use atomiks::{AtomicBool, AtomicI8, AtomicU8, AtomicU32};
    #[cfg(target_arch = "aarch64")]
    use atomiks::{AtomicI64, AtomicU64};

    #[test]
    fn add_and_sub_wrap() {
        let count = AtomicU8::new(255);
        assert_eq!(count.fetch_add(1, AcqRel), 255, "the value before");
        assert_eq!(count.load(Relaxed), 0, "wrapped");
        assert_eq!(count.fetch_sub(1, AcqRel), 0, "the value before");
        assert_eq!(count.load(Relaxed), 255, "wrapped back");
        count.add(2, Relaxed);
        assert_eq!(count.load(Relaxed), 1, "up two, wrapping");
        count.sub(3, Relaxed);
        assert_eq!(count.load(Relaxed), 254, "down three, wrapping");
    }

    #[test]
    fn signed_add_and_sub_wrap_at_the_signed_bounds() {
        let level = AtomicI8::new(i8::MAX);
        assert_eq!(level.fetch_add(1, AcqRel), i8::MAX, "the value before");
        assert_eq!(level.load(Relaxed), i8::MIN, "past the greatest, the least");
        level.sub(1, Relaxed);
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
        signed.min(-7, Relaxed);
        assert_eq!(signed.load(Relaxed), -7, "-7 is the smaller signed value");
        let unsigned = AtomicU64::new(7);
        assert_eq!(unsigned.fetch_min(2, AcqRel), 7, "the value before");
        assert_eq!(unsigned.load(Relaxed), 2, "2 is the smaller");
        unsigned.max(u64::MAX, Relaxed);
        assert_eq!(unsigned.load(Relaxed), u64::MAX, "the top bit set is the larger, unsigned");
        let narrow = AtomicI8::new(-1);
        narrow.max(1, Relaxed);
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
}
