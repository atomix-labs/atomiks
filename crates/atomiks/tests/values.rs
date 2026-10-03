//! The built-in values beyond the integers: each round-trips, and each `Option` takes its niche.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::num::{NonZero, Saturating, Wrapping};
    use core::ptr::{self, NonNull};

    use atomiks::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomiks::{Atom, Atomic, AtomicPtr};

    #[test]
    fn an_opaque_cell_has_the_layout_of_its_primitive() {
        assert_eq!(size_of::<Atomic<NonZero<u64>>>(), 8, "`from_ptr` casts a `*mut u64`");
        assert_eq!(align_of::<Atomic<NonZero<u64>>>(), 8, "and needs `u64`'s alignment");
        assert_eq!(size_of::<Atomic<char>>(), 4, "and `char`'s `ZeroValid` cell");
    }

    #[test]
    fn option_of_a_nonzero_spends_zero_on_none() {
        let owner = Atomic::<Option<NonZero<u64>>>::new(None);
        assert_eq!(size_of_val(&owner), 8, "one word for the value and its absence");
        assert_eq!(<Option<NonZero<u64>> as Atom>::to_repr(None), 0, "None is zero");
        owner.store(NonZero::new(9), Release);
        assert_eq!(owner.load(Acquire), NonZero::new(9), "Some round-trips");
    }

    #[test]
    fn load_rmw_leaves_a_value_with_no_zero_unchanged() {
        let max = NonZero::<u64>::MAX;
        let value = Atomic::new(max);
        assert_eq!(value.load_rmw(Acquire), max, "the value");
        assert_eq!(value.load(Relaxed), max, "unchanged: the exchange fails on a nonzero cell");
    }

    #[test]
    fn a_range_from_zero_puts_none_above_it() {
        assert_eq!(<Option<char> as Atom>::to_repr(None), 0x11_0000, "just past char::MAX");
        let letter = Atomic::new(Some('λ'));
        assert_eq!(letter.swap(None, AcqRel), Some('λ'), "the value before");
        assert_eq!(letter.load(Relaxed), None, "and None after");
    }

    #[test]
    fn a_float_keeps_its_exact_bits() {
        let value = Atomic::new(-0.0_f64);
        assert_eq!(value.load(Relaxed).to_bits(), (-0.0_f64).to_bits(), "negative zero");
        let payload = f64::from_bits(0x7FF8_0000_0000_0001);
        value.store(payload, Relaxed);
        assert_eq!(value.load(Relaxed).to_bits(), payload.to_bits(), "and a NaN's payload");
        assert_eq!(Atomic::new(1.5_f32).into_inner(), 1.5, "f32 too");
    }

    #[test]
    fn a_zero_width_value_takes_a_byte_and_each_none_the_next_repr() {
        assert_eq!(size_of::<Atomic<()>>(), 1, "`()`'s repr is a `u8`");
        assert_eq!(<Option<()> as Atom>::to_repr(None), 1, "`None` takes the repr past `()`'s 0");
        assert_eq!(<Option<Option<()>> as Atom>::to_repr(None), 2, "and the outer `None` the next");
    }

    #[test]
    fn the_wrappers_operate_as_their_integers_do() {
        let wrapping = Atomic::new(Wrapping(u8::MAX));
        assert_eq!(wrapping.fetch_add(1, AcqRel), Wrapping(u8::MAX), "the value before");
        assert_eq!(wrapping.load(Relaxed), Wrapping(0), "wrapped");
        let saturating = Atomic::new(Saturating(0b01_i32));
        saturating.or(Saturating(0b10), AcqRel);
        assert_eq!(saturating.load(Relaxed), Saturating(0b11), "or");
    }

    #[test]
    fn a_pointer_keeps_its_provenance_through_the_cell() {
        let mut slots = [10_u64, 20, 30];
        let base = slots.as_mut_ptr();
        let cell = AtomicPtr::new(base);
        let before = cell.fetch_ptr_add(2, AcqRel);
        assert_eq!(before, base, "the pointer before");
        let after = cell.load(Acquire);
        // SAFETY: `after` is two elements into the live `slots`, with its provenance.
        #[expect(unsafe_code, reason = "reading through the pointer the cell holds")]
        let third = unsafe { after.read() };
        assert_eq!(third, 30, "the third slot");
    }

    #[test]
    fn a_pointer_keeps_its_provenance_through_address_zero() {
        let mut slot = 5_u64;
        let base = ptr::from_mut(&mut slot);
        let cell = AtomicPtr::new(base);
        cell.byte_sub(base.addr(), AcqRel);
        // `load_rmw` would write a null over it, without the provenance: it takes only integers.
        assert_eq!(cell.load(Acquire).addr(), 0, "the address is zero");
        cell.byte_add(base.addr(), AcqRel);
        // SAFETY: the pointer is `base` again, with its provenance, and `slot` is live.
        #[expect(unsafe_code, reason = "reading through the pointer the cell holds")]
        let value = unsafe { cell.load(Acquire).read() };
        assert_eq!(value, 5, "the slot, through the pointer walked back");
    }

    #[test]
    fn a_pointer_offsets_by_elements_or_by_bytes() {
        let base = ptr::null_mut::<u64>();
        let cell = AtomicPtr::new(base);
        cell.ptr_add(3, AcqRel);
        assert_eq!(cell.load(Relaxed), base.wrapping_add(3), "three elements on");
        cell.byte_sub(8, AcqRel);
        assert_eq!(cell.load(Relaxed), base.wrapping_add(2), "one 8-byte element back");
        cell.ptr_sub(2, AcqRel);
        assert_eq!(cell.load(Relaxed), base, "two elements back");
        cell.byte_add(24, AcqRel);
        assert_eq!(cell.load(Relaxed), base.wrapping_add(3), "three elements' bytes on");
        assert_eq!(cell.fetch_ptr_sub(1, AcqRel), base.wrapping_add(3), "the pointer before");
        assert_eq!(cell.fetch_byte_sub(8, AcqRel), base.wrapping_add(2), "the pointer before");
        assert_eq!(cell.fetch_byte_add(16, AcqRel), base.wrapping_add(1), "the pointer before");
        assert_eq!(cell.load(Relaxed), base.wrapping_add(3), "after the last offset");
    }

    #[test]
    fn option_of_a_non_null_pointer_is_null_for_none() {
        let mut slot = 1_u8;
        assert!(<Option<NonNull<u8>> as Atom>::to_repr(None).is_null(), "None is null");
        let cell = Atomic::<Option<NonNull<u8>>>::new(None);
        assert!(cell.load(Relaxed).is_none(), "and null is None");
        cell.store(Some(NonNull::from(&mut slot)), Release);
        assert_eq!(cell.load(Acquire), Some(NonNull::from(&mut slot)), "Some round-trips");
        let constant = Atomic::new(ptr::null::<u8>());
        assert!(constant.load(Relaxed).is_null(), "a *const too");
    }
}
