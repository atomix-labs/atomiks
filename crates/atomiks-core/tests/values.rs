//! The built-in values beyond the integers: each round-trips, and each `Option` takes its niche;
//! and values of a crate's own, one whose range runs through zero among them.

// Above the `cfg`, which under loom drops every attribute after it: the parser refuses a `const`
// impl without the feature before the `cfg` drops the impl.
#![feature(const_trait_impl)]
// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]

#[cfg(test)]
mod tests {
    use core::num::{NonZero, Saturating, Wrapping};
    use core::ptr::{self, NonNull};

    use atomiks_core::ordering::{AcqRel, Acquire, Relaxed, Release};
    use atomiks_core::validity::{TotalZeroNiche, ZeroNiche};
    use atomiks_core::{Atom, Atomic, AtomicPtr, ReprRange};

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

    /// A ticket number, never zero, whose `Atom` impl keeps the default `from_repr_unchecked`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Ticket(NonZero<u32>);

    // SAFETY: the repr is the ticket's number, never zero, so within `REPRS`; `from_repr` decodes
    // exactly the nonzero reprs, each as the ticket it numbers; the default unchecked decode
    // unwraps `from_repr`; `Partial` promises nothing; a number may cross threads.
    #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
    unsafe impl Atom for Ticket {
        type Repr = u32;
        const REPRS: ReprRange<u32> = ReprRange::NONZERO;
        fn to_repr(self) -> u32 {
            self.0.get()
        }
        fn from_repr(repr: u32) -> Option<Self> {
            NonZero::new(repr).map(Self)
        }
    }

    #[test]
    fn the_default_unchecked_decode_reads_back_each_ticket() {
        let ticket = |number| Ticket(NonZero::new(number).expect("ticket numbers start at 1"));
        let next = Atomic::from(ticket(1));
        assert_eq!(next.load(Acquire), ticket(1), "a load decodes the ticket built");
        next.store(ticket(2), Release);
        assert_eq!(next.swap(ticket(3), AcqRel), ticket(2), "a swap, the ticket stored");
        assert_eq!(
            next.compare_exchange(ticket(3), ticket(4), AcqRel, Acquire),
            Ok(ticket(3)),
            "a compare-exchange, the ticket swapped in"
        );
        assert_eq!(next.load(Relaxed), ticket(4), "and a load, the ticket exchanged in");
    }

    /// Which way a price moved: -1, 0 or 1 as an `i8`, a range through zero.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Sign {
        Minus,
        Flat,
        Plus,
    }

    // SAFETY: `to_repr` gives -1, 0 or 1, within `REPRS`, and `from_repr` decodes each as the sign
    // it came from, by the repr alone; the default unchecked decode unwraps `from_repr`; `Partial`
    // promises nothing; a `Sign` holds no data, so it may cross threads.
    #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
    const unsafe impl Atom for Sign {
        type Repr = i8;
        const REPRS: ReprRange<i8> = ReprRange::from_signed(-1, 1);
        fn to_repr(self) -> i8 {
            match self {
                Self::Minus => -1,
                Self::Flat => 0,
                Self::Plus => 1,
            }
        }
        fn from_repr(repr: i8) -> Option<Self> {
            match repr {
                -1 => Some(Self::Minus),
                0 => Some(Self::Flat),
                1 => Some(Self::Plus),
                _ => None,
            }
        }
    }

    #[test]
    fn option_of_a_range_through_zero_takes_the_reprs_below_it() {
        static LAST_MOVE: Atomic<Option<Sign>> = Atomic::new(None);
        #[expect(clippy::option_option, reason = "an `Option` of an `Option` is what this tests")]
        static FIRST_MOVE: Atomic<Option<Option<Sign>>> = Atomic::new(None);
        assert_eq!(<Option<Sign> as Atom>::to_repr(None), -2, "`None` takes 0xFE, below -1");
        assert_eq!(
            <Option<Sign> as Atom>::REPRS,
            ReprRange::from_signed(-2, 1),
            "which its range holds"
        );
        assert_eq!(<Option<Option<Sign>> as Atom>::to_repr(None), -3, "the outer `None` 0xFD");
        assert_eq!(
            <Option<Option<Sign>> as Atom>::REPRS,
            ReprRange::from_signed(-3, 1),
            "and its range both"
        );
        assert_eq!(LAST_MOVE.load(Acquire), None, "a static built with `None`");
        LAST_MOVE.store(Some(Sign::Minus), Release);
        assert_eq!(LAST_MOVE.swap(Some(Sign::Plus), AcqRel), Some(Sign::Minus), "holds -1");
        assert_eq!(LAST_MOVE.load(Acquire), Some(Sign::Plus), "and 1");
        assert_eq!(FIRST_MOVE.load(Acquire), None, "the outer `None`");
        FIRST_MOVE.store(Some(None), Release);
        assert_eq!(FIRST_MOVE.load(Acquire), Some(None), "the inner `None`, apart from it");
        FIRST_MOVE.store(Some(Some(Sign::Flat)), Release);
        assert_eq!(FIRST_MOVE.load(Acquire), Some(Some(Sign::Flat)), "and zero, a value");
    }

    /// An even byte from 2 to 254: zero is no value, and lies apart from its range.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Even(u8);

    // SAFETY: `to_repr` gives the byte, even and from 2 to 254, so within `REPRS`; `from_repr`
    // decodes exactly the even bytes but zero, each as itself, so zero decodes as none, as
    // `ZeroNiche` promises; the default unchecked decode unwraps `from_repr`; a byte may cross
    // threads.
    #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
    unsafe impl Atom for Even {
        type Repr = u8;
        type Validity = ZeroNiche;
        const REPRS: ReprRange<u8> = ReprRange::new(2, 254);
        fn to_repr(self) -> u8 {
            self.0
        }
        fn from_repr(repr: u8) -> Option<Self> {
            (repr != 0 && repr.is_multiple_of(2)).then_some(Self(repr))
        }
    }

    #[test]
    fn option_of_a_zero_niche_apart_from_its_range_grows_the_range_to_zero() {
        assert_eq!(<Option<Even> as Atom>::to_repr(None), 0, "`None` takes zero");
        assert_eq!(<Option<Even> as Atom>::REPRS, ReprRange::new(0, 254), "the range grows to it");
        assert_eq!(<Option<Option<Even>> as Atom>::to_repr(None), 255, "the outer `None` past it");
        let even = Atomic::from(None::<Even>);
        even.store(Some(Even(254)), Release);
        assert_eq!(even.swap(None, AcqRel), Some(Even(254)), "254 decodes as itself");
        assert_eq!(even.load(Acquire), None, "and zero as `None`");
    }

    /// A nonzero byte whose range is every byte: zero lies inside it, but decodes as no value.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Count(NonZero<u8>);

    // SAFETY: every repr lies in `REPRS`; `from_repr` decodes every nonzero byte, as the count it
    // is, and zero as none, as `TotalZeroNiche` promises; the default unchecked decode unwraps
    // `from_repr`; a byte may cross threads.
    #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
    unsafe impl Atom for Count {
        type Repr = u8;
        type Validity = TotalZeroNiche;
        const REPRS: ReprRange<u8> = ReprRange::FULL;
        fn to_repr(self) -> u8 {
            self.0.get()
        }
        fn from_repr(repr: u8) -> Option<Self> {
            NonZero::new(repr).map(Self)
        }
    }

    #[test]
    fn option_of_a_zero_niche_in_a_full_range_takes_zero() {
        assert_eq!(<Option<Count> as Atom>::to_repr(None), 0, "`None` takes zero, inside");
        assert_eq!(<Option<Count> as Atom>::REPRS, ReprRange::FULL, "and the range stays full");
        let count = Atomic::from(NonZero::new(3).map(Count));
        assert_eq!(count.swap(None, AcqRel), NonZero::new(3).map(Count), "a count");
        assert_eq!(count.load(Acquire), None, "and `None`, apart from it");
    }

    #[test]
    fn exclusive_access_encodes_as_the_atomic_operations_do() {
        let mut owner = Atomic::<Option<NonZero<u64>>>::new(None);
        owner.set(NonZero::new(7));
        assert_eq!(owner.load(Acquire), NonZero::new(7), "a load reads what set wrote");
        let before = owner.with_mut(|seen| seen.replace(NonZero::<u64>::MAX));
        assert_eq!(before, NonZero::new(7), "with_mut lends what set wrote");
        assert_eq!(
            owner.load(Acquire),
            Some(NonZero::<u64>::MAX),
            "a load reads what with_mut left"
        );
        owner.store(None, Release);
        assert_eq!(owner.get(), None, "get reads what a store wrote");
        let mut letter = Atomic::new('a');
        letter.with_mut(|seen| *seen = 'λ');
        assert_eq!(letter.load(Relaxed), 'λ', "a load reads the char with_mut left");
    }

    #[test]
    fn a_pointer_keeps_its_provenance_through_exclusive_access() {
        let mut slots = [10_u32, 20];
        let base = slots.as_mut_ptr();
        let mut raw = AtomicPtr::new(ptr::null_mut());
        raw.set(base);
        raw.with_mut(|seen| *seen = seen.wrapping_add(1));
        // SAFETY: the pointer is one element into the live `slots`, with `base`'s provenance.
        #[expect(unsafe_code, reason = "reading through the pointer the cell holds")]
        let second = unsafe { raw.load(Acquire).read() };
        assert_eq!(second, 20, "a load reads the pointer with_mut moved, with its provenance");
        let mut non_null = Atomic::new(NonNull::dangling());
        non_null.set(NonNull::new(base).expect("an array's pointer is not null"));
        // SAFETY: the pointer is `base`, with its provenance, and `slots` is live.
        #[expect(unsafe_code, reason = "reading through the pointer the cell holds")]
        let first = unsafe { non_null.load(Acquire).read() };
        assert_eq!(first, 10, "a load reads the `NonNull` set wrote, with its provenance");
    }
}
