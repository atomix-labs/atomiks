//! zerocopy's traits on an atomic, each derived through its cell: which atomics have each, and an
//! atomic read from bytes, from zeros, and at any offset.

#![cfg(feature = "zerocopy-08")]
// The derives are absent under loom, whose cells are not plain memory.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::num::{NonZero, Wrapping};
    use core::ptr::{self, NonNull};

    use atomix_core::ordering::Release;
    use atomix_core::{Atomic, AtomicBool, AtomicI8, AtomicU8, AtomicU64, RangedU8, RangedU64};
    use zerocopy::{
        FromBytes, FromZeros, IntoBytes, KnownLayout, TryFromBytes, Unaligned, transmute,
    };

    /// Compiles only where zerocopy knows `T`'s layout.
    const fn has_known_layout<T: KnownLayout>() {}
    /// Compiles only where `T`'s bytes may be read.
    const fn has_into_bytes<T: IntoBytes>() {}
    /// Compiles only where `T` has alignment 1.
    const fn has_unaligned<T: Unaligned>() {}
    /// Compiles only where `T` reads from any bytes, and so from checked bytes and from zeros.
    const fn has_from_bytes<T: FromBytes>() {}
    /// Compiles only where `T` reads from zeros, and so from checked bytes.
    const fn has_from_zeros<T: FromZeros>() {}

    #[test]
    fn every_atomic_has_a_known_layout() {
        has_known_layout::<AtomicU64>();
        has_known_layout::<AtomicBool>();
        has_known_layout::<Atomic<char>>();
        has_known_layout::<Atomic<*mut u8>>();
        has_known_layout::<Atomic<NonNull<u8>>>();
        has_known_layout::<Atomic<RangedU64<3>>>();
        #[cfg(wide)]
        has_known_layout::<Atomic<NonZero<u128>>>();
    }

    #[test]
    fn every_atomic_but_a_pointer_has_its_bytes() {
        has_into_bytes::<AtomicU64>();
        has_into_bytes::<AtomicBool>();
        has_into_bytes::<Atomic<char>>();
        has_into_bytes::<Atomic<()>>();
        has_into_bytes::<Atomic<NonZero<u64>>>();
        has_into_bytes::<Atomic<RangedU64<3>>>();
        #[cfg(wide)]
        has_into_bytes::<Atomic<NonZero<u128>>>();
        let bytes: [u8; 4] = transmute!(Atomic::new('A'));
        assert_eq!(bytes, 65_u32.to_ne_bytes(), "a `char`'s bytes, by value");
    }

    #[test]
    fn an_atomic_whose_every_integer_decodes_reads_from_any_bytes() {
        has_from_bytes::<AtomicU64>();
        has_from_bytes::<AtomicI8>();
        has_from_bytes::<Atomic<f64>>();
        has_from_bytes::<Atomic<Wrapping<u32>>>();
        has_from_bytes::<Atomic<Option<NonZero<u64>>>>();
        #[cfg(wide)]
        has_from_bytes::<Atomic<u128>>();
        let seq = AtomicU64::read_from_bytes(&7_u64.to_ne_bytes()).ok().map(Atomic::into_inner);
        assert_eq!(seq, Some(7), "7, from its bytes");
        let owner = Atomic::<Option<NonZero<u64>>>::read_from_bytes(&[0; 8]);
        assert_eq!(owner.ok().map(Atomic::into_inner), Some(None), "zeros, `None`");
        assert_eq!(Atomic::<f64>::new_zeroed().into_inner().to_bits(), 0, "zeros, 0.0");
        #[cfg(wide)]
        assert_eq!(Atomic::<u128>::new_zeroed().into_inner(), 0, "and 0, 128 bits wide");
    }

    #[test]
    fn a_bool_and_a_pointer_read_only_from_zeros_and_checked_bytes() {
        has_from_zeros::<AtomicBool>();
        has_from_zeros::<Atomic<*mut u8>>();
        has_from_zeros::<Atomic<*const u8>>();
        has_from_zeros::<Atomic<Option<NonNull<u8>>>>();
        let one = AtomicBool::try_read_from_bytes(&[1]).ok().map(AtomicBool::into_inner);
        assert_eq!(one, Some(true), "1 is true");
        let two = AtomicBool::try_read_from_bytes(&[2]).ok().map(AtomicBool::into_inner);
        assert_eq!(two, None, "2 is no `bool`");
        let null = Atomic::<*mut u8>::try_read_from_bytes(&[0; size_of::<usize>()]);
        assert_eq!(null.ok().map(Atomic::into_inner), Some(ptr::null_mut()), "zeros, null");
        let address = Atomic::<*mut u8>::try_read_from_bytes(&1_usize.to_ne_bytes()).ok();
        assert_eq!(
            address.map(Atomic::into_inner),
            None,
            "any other address is refused: bytes carry no provenance"
        );
        assert!(!AtomicBool::new_zeroed().into_inner(), "a `bool` from zeros, false");
    }

    #[test]
    fn an_eight_bit_atomic_reads_from_bytes_at_any_offset() {
        has_unaligned::<AtomicU8>();
        has_unaligned::<AtomicI8>();
        has_unaligned::<AtomicBool>();
        has_unaligned::<Atomic<()>>();
        has_unaligned::<Atomic<RangedU8<1, 10>>>();
        has_unaligned::<Atomic<Option<NonZero<u8>>>>();
        let mut bytes = [0_u8, 7];
        let (_, last) = AtomicU8::mut_from_suffix(&mut bytes).expect("a byte, at any offset");
        last.store(9, Release);
        assert_eq!(bytes, [0, 9], "the store, in the last byte");
    }
}
