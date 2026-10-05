//! bytemuck's `Zeroable` on an atomic whose zero repr decodes, and its byte traits on a ranged
//! integer: its bytes, bytes checked against its range, and its integers as a contiguous range.

#![cfg(feature = "bytemuck")]

// Under loom a ranged integer's traits stay on, since none reads an atomic's memory as bytes;
// `check-loom` builds this.
#[cfg(loom)]
const _: () = {
    use atomiks_core::RangedU16;
    use bytemuck::{CheckedBitPattern, Contiguous, NoUninit};

    /// Compiles only where `T` has each of a ranged integer's byte traits.
    const fn has_byte_traits<T: NoUninit + CheckedBitPattern + Contiguous>() {}

    has_byte_traits::<RangedU16<1, 10>>();
};

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use core::num::NonZero;
    use core::ptr::{self, NonNull};

    use atomiks_core::{Atomic, AtomicBool, AtomicU64, RangedI16, RangedU16};
    use bytemuck::checked::{self, CheckedCastError};
    use bytemuck::{Contiguous, Zeroable, bytes_of};

    /// How many levels of an order book a feed sends.
    type Depth = RangedU16<1, 10>;

    #[test]
    fn an_atomic_whose_zero_decodes_is_zeroable() {
        assert_eq!(AtomicU64::zeroed().into_inner(), 0, "an integer: 0");
        assert!(!AtomicBool::zeroed().into_inner(), "a `bool`: false");
        assert_eq!(Atomic::<Option<NonZero<u64>>>::zeroed().into_inner(), None, "a niche: `None`");
        assert_eq!(Atomic::<char>::zeroed().into_inner(), '\0', "a `char`: NUL");
        assert_eq!(Atomic::<*mut u8>::zeroed().into_inner(), ptr::null_mut(), "a pointer: null");
        assert_eq!(Atomic::<Option<NonNull<u8>>>::zeroed().into_inner(), None, "and `None`");
        assert_eq!(Atomic::<()>::zeroed().into_inner(), (), "the unit");
        #[cfg(wide)]
        assert_eq!(Atomic::<u128>::zeroed().into_inner(), 0, "and 0, 128 bits wide");
    }

    #[test]
    fn a_ranged_integer_has_its_integers_bytes() {
        let depth = Depth::new(5).expect("5 is in 1..=10");
        assert_eq!(bytes_of(&depth), 5_u16.to_ne_bytes(), "the integer's bytes");
    }

    #[test]
    fn a_ranged_integer_reads_only_bytes_inside_its_range() {
        // Each borrows a `u16`, so its bytes are aligned as `Depth`'s, which the cast checks first.
        assert_eq!(
            checked::try_from_bytes::<Depth>(bytes_of(&1_u16)),
            Ok(&Depth::MIN),
            "1, the smallest"
        );
        assert_eq!(
            checked::try_from_bytes::<Depth>(bytes_of(&10_u16)),
            Ok(&Depth::MAX),
            "10, the largest"
        );
        assert_eq!(
            checked::try_from_bytes::<Depth>(bytes_of(&11_u16)),
            Err(CheckedCastError::InvalidBitPattern),
            "11, past the largest"
        );
        assert_eq!(
            checked::try_pod_read_unaligned::<RangedI16<-5, 5>>(&(-6_i16).to_ne_bytes()),
            Err(CheckedCastError::InvalidBitPattern),
            "and -6, below a signed range"
        );
    }

    #[test]
    fn a_ranged_integer_is_its_contiguous_integers() {
        assert_eq!(Depth::from_integer(1), Some(Depth::MIN), "the smallest");
        assert_eq!(Depth::from_integer(0), None, "below it");
        assert_eq!(Depth::from_integer(11), None, "above the largest");
        assert_eq!(Depth::MAX.into_integer(), 10, "and back to an integer");
        assert_eq!(
            (RangedI16::<-5, 5>::MIN_VALUE, RangedI16::<-5, 5>::MAX_VALUE),
            (-5, 5),
            "a signed range's bounds"
        );
    }
}
