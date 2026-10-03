//! The `Atom` laws over every built-in: a value's repr lies in its range, which may wrap, and
//! decodes back to it; a repr that decodes re-encodes to itself, unchecked too; each validity's
//! promise holds; and `None` takes a repr outside its value's range.
//!
//! Each law runs on every repr of a byte, on the edges of each width and range, and on random bits.

// Loom's cells exist only inside a model; `model.rs` holds the loom tests.
#![cfg(not(loom))]
#![feature(f16, integer_casts)]
#![cfg_attr(wide, feature(f128))]

#[cfg(test)]
mod tests {
    use core::fmt::Debug;
    use core::marker::PhantomData;
    use core::num::{NonZero, Saturating, Wrapping};
    use core::ptr::{self, NonNull};

    #[cfg(target_arch = "aarch64")]
    use atomiks_core::ordering::Relaxed;
    use atomiks_core::validity::{Total, TotalZeroNiche, ZeroValid};
    use atomiks_core::{Atom, AtomOrd, ExactBits, Primitive};
    #[cfg(target_arch = "aarch64")]
    use atomiks_core::{Atomic, Load, MinMax};
    use proptest::prelude::{Strategy, any, prop_oneof};
    use proptest::sample::select;
    use proptest::test_runner::TestCaseError;
    use proptest::{prop_assert, prop_assert_eq, proptest};

    /// The last scalar value, and the first and last surrogates.
    const CHAR_EDGES: [u128; 3] = [0x10_FFFF, 0xD800, 0xDFFF];

    /// Edge bits, each with its neighbours.
    ///
    /// The edges are zero, each width's largest signed and unsigned values, and `extra`.
    fn edges(extra: &[u128]) -> Vec<u128> {
        let tops = [8_u32, 16, 32, 64, 128]
            .map(|width| u128::MAX.unbounded_shr(128_u32.wrapping_sub(width)));
        let mut edges = vec![0];
        edges.extend(tops.into_iter().flat_map(|top| [top.unbounded_shr(1), top]));
        edges.extend_from_slice(extra);
        edges
            .into_iter()
            .flat_map(|edge| [edge.wrapping_sub(1), edge, edge.wrapping_add(1)])
            .collect()
    }

    /// Bits, half from `edges` and half anywhere; each primitive takes their low bits.
    fn edge_or_random_bits(extra: &[u128]) -> impl Strategy<Value = u128> {
        prop_oneof![select(edges(extra)), any::<u128>()]
    }

    /// The range and round-trip laws for `value`, and the repr laws for its repr.
    fn round_trips<T: Atom + PartialEq + Debug>(value: T) -> Result<(), TestCaseError>
    where
        T::Repr: ExactBits + PartialEq + Debug,
    {
        let bits = value.to_repr().to_bits();
        prop_assert!(T::REPRS.contains(bits), "{value:?}'s repr {bits:#x} lies in {:?}", T::REPRS);
        prop_assert_eq!(T::from_repr(value.to_repr()), Some(value), "the value decodes back");
        canonical::<T>(value.to_repr())
    }

    /// The range and round-trip laws for `value`, whose repr is a pointer.
    fn pointer_round_trips<T: Atom<Repr = *mut P> + PartialEq + Debug, P>(
        value: T,
    ) -> Result<(), TestCaseError> {
        let address = value.to_repr().addr();
        prop_assert!(
            u128::try_from(address).is_ok_and(|bits| T::REPRS.contains(bits)),
            "{value:?}'s address {address:#x} lies in {:?}",
            T::REPRS
        );
        prop_assert_eq!(T::from_repr(value.to_repr()), Some(value), "the value decodes back");
        canonical::<T>(value.to_repr())
    }

    /// The float laws for `bits`: they lie in range and are the repr of the float they make, which
    /// decodes back to them, checked and unchecked, NaN included.
    fn keeps_every_bit<T: Atom<Validity = Total> + Debug>(
        bits: u128, from_bits: fn(T::Repr) -> T,
    ) -> Result<(), TestCaseError>
    where
        T::Repr: ExactBits + PartialEq + Debug,
    {
        let repr = <T::Repr as Primitive>::from_bits(bits);
        let unsigned = repr.to_bits();
        prop_assert!(T::REPRS.contains(unsigned), "the bits {unsigned:#x} lie in {:?}", T::REPRS);
        prop_assert_eq!(from_bits(repr).to_repr(), repr, "the repr is the bits");
        total::<T>(repr)
    }

    /// The repr laws for `repr`: if it decodes, the value it decodes to, checked and unchecked,
    /// re-encodes to it.
    fn canonical<T: Atom + Debug>(repr: T::Repr) -> Result<(), TestCaseError>
    where
        T::Repr: PartialEq + Debug,
    {
        if let Some(value) = T::from_repr(repr) {
            prop_assert_eq!(
                value.to_repr(),
                repr,
                "{:?} re-encodes to the repr it came from",
                value
            );
            // SAFETY: `from_repr` decodes `repr`.
            #[expect(unsafe_code, reason = "the unchecked decode the `Atom` contract constrains")]
            let unchecked = unsafe { T::from_repr_unchecked(repr) };
            prop_assert_eq!(unchecked.to_repr(), repr, "{:?} decodes unchecked as checked", repr);
        }
        Ok(())
    }

    /// The `None` law for `T`, given `none`, the bits of `None`'s repr: they lie outside `T`'s
    /// range and decode as no `T`, and `Option<T>`'s range holds them.
    fn none_lies_outside<T: Atom>(none: u128) -> Result<(), TestCaseError> {
        prop_assert!(!T::REPRS.contains(none), "`None`'s {none:#x} lies outside {:?}", T::REPRS);
        let repr = <T::Repr as Primitive>::from_bits(none);
        prop_assert!(T::from_repr(repr).is_none(), "and decodes as no value");
        let with_none = <Option<T> as Atom>::REPRS;
        prop_assert!(with_none.contains(none), "and lies in `Option`'s, {:?}", with_none);
        Ok(())
    }

    /// The `Total` law for `repr`: it decodes; then the repr laws.
    fn total<T: Atom<Validity = Total> + Debug>(repr: T::Repr) -> Result<(), TestCaseError>
    where
        T::Repr: PartialEq + Debug,
    {
        prop_assert!(T::from_repr(repr).is_some(), "every repr decodes, {:?} too", repr);
        canonical::<T>(repr)
    }

    /// The `TotalZeroNiche` law for `repr`: it decodes unless it is zero; then the repr laws.
    fn total_zero_niche<T: Atom<Validity = TotalZeroNiche> + Debug>(
        repr: T::Repr,
    ) -> Result<(), TestCaseError>
    where
        T::Repr: PartialEq + Debug,
    {
        let nonzero = !repr.is_bits(0);
        prop_assert_eq!(
            T::from_repr(repr).is_some(),
            nonzero,
            "every repr but zero decodes, {:?} too",
            repr
        );
        canonical::<T>(repr)
    }

    /// The `ZeroValid` law: zero decodes.
    fn zero_decodes<T: Atom<Validity = ZeroValid>>() -> Result<(), TestCaseError> {
        let zero = <T::Repr as Primitive>::from_bits(0);
        prop_assert!(T::from_repr(zero).is_some(), "zero decodes");
        Ok(())
    }

    /// The order law for `a` and `b`: the reprs' own order is the values'.
    fn reprs_order_as_ord<T: AtomOrd + Debug>(a: T, b: T) -> Result<(), TestCaseError>
    where
        T::Repr: Ord,
    {
        let (values, reprs) = (a.cmp(&b), a.to_repr().cmp(&b.to_repr()));
        prop_assert_eq!(reprs, values, "{:?} and {:?} order as their reprs do", a, b);
        Ok(())
    }

    /// The order law for `a` and `b` through the atomic maximum and minimum, which compare reprs:
    /// each keeps what `Ord`'s does, and returns the value before.
    #[cfg(target_arch = "aarch64")]
    fn atomics_order_as_ord<T: AtomOrd + Debug>(a: T, b: T) -> Result<(), TestCaseError>
    where
        T::Repr: Load + MinMax,
    {
        let larger = Atomic::from(a);
        prop_assert_eq!(larger.fetch_max(b, Relaxed), a, "fetch_max returns the value before");
        prop_assert_eq!(larger.load(Relaxed), a.max(b), "fetch_max keeps the larger");
        let smaller = Atomic::from(a);
        prop_assert_eq!(smaller.fetch_min(b, Relaxed), a, "fetch_min returns the value before");
        prop_assert_eq!(smaller.load(Relaxed), a.min(b), "fetch_min keeps the smaller");
        Ok(())
    }

    /// Checks every law on the low bits of `$bits` and `$other` as each integer type.
    ///
    /// The same laws run on its `NonZero`, the `Option` of that, `Wrapping` and `Saturating`.
    macro_rules! integer_laws {
        ($bits:expr, $other:expr; $($int:ty),+) => {$({
            let (a, b): ($int, $int) = ($bits.wrapping_cast(), $other.wrapping_cast());
            total::<$int>(a)?;
            total_zero_niche::<NonZero<$int>>(a)?;
            total::<Option<NonZero<$int>>>(a)?;
            round_trips(a)?;
            round_trips(Wrapping(a))?;
            round_trips(Saturating(a))?;
            round_trips(NonZero::new(a))?;
            reprs_order_as_ord(a, b)?;
            if let (Some(a), Some(b)) = (NonZero::new(a), NonZero::new(b)) {
                round_trips(a)?;
                reprs_order_as_ord(a, b)?;
            }
        })+};
    }

    /// Checks the atomic order law on the low bits of `$bits` and `$other` as each integer type.
    ///
    /// The same law runs on its `NonZero`, `Wrapping` and `Saturating`.
    #[cfg(target_arch = "aarch64")]
    macro_rules! atomic_order_laws {
        ($bits:expr, $other:expr; $($int:ty),+) => {$({
            let (a, b): ($int, $int) = ($bits.wrapping_cast(), $other.wrapping_cast());
            atomics_order_as_ord(a, b)?;
            atomics_order_as_ord(Wrapping(a), Wrapping(b))?;
            atomics_order_as_ord(Saturating(a), Saturating(b))?;
            if let (Some(a), Some(b)) = (NonZero::new(a), NonZero::new(b)) {
                atomics_order_as_ord(a, b)?;
            }
        })+};
    }

    /// Every law on `bits` and `other` for each integer of 64 bits or fewer.
    fn narrow_integer_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        integer_laws!(bits, other; u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
        #[cfg(target_arch = "aarch64")]
        atomic_order_laws!(bits, other; u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
        Ok(())
    }

    /// Every law on `bits` as the repr of the 128-bit values.
    #[cfg(wide)]
    fn wide_laws(bits: u128, other: u128) -> Result<(), TestCaseError> {
        integer_laws!(bits, other; u128, i128);
        keeps_every_bit(bits, f128::from_bits)
    }

    /// Every law on the low bits of `bits` as the repr of a `char` and of its `Option`s.
    ///
    /// The order laws compare the `char` they make, if any, with `other`.
    fn char_laws(bits: u128, other: char) -> Result<(), TestCaseError> {
        let repr = u32::from_bits(bits);
        zero_decodes::<char>()?;
        zero_decodes::<Option<char>>()?;
        canonical::<char>(repr)?;
        canonical::<Option<char>>(repr)?;
        canonical::<Option<Option<char>>>(repr)?;
        round_trips(None::<char>)?;
        round_trips(None::<Option<char>>)?;
        if let Some(letter) = char::from_u32(repr) {
            round_trips(letter)?;
            round_trips(Some(letter))?;
            reprs_order_as_ord(letter, other)?;
            #[cfg(target_arch = "aarch64")]
            atomics_order_as_ord(letter, other)?;
        }
        Ok(())
    }

    /// Every law on the low bits of `bits` as each float's.
    fn float_laws(bits: u128) -> Result<(), TestCaseError> {
        keeps_every_bit(bits, f16::from_bits)?;
        keeps_every_bit(bits, f32::from_bits)?;
        keeps_every_bit(bits, f64::from_bits)
    }

    /// Every law on the low bits of `bits` as an address, for each pointer and `Option<NonNull>`.
    fn pointer_laws(bits: u128) -> Result<(), TestCaseError> {
        let pointer = ptr::without_provenance_mut::<u8>(bits.wrapping_cast());
        pointer_round_trips(pointer)?;
        pointer_round_trips(pointer.cast_const())?;
        pointer_round_trips(NonNull::new(pointer))?;
        if let Some(value) = NonNull::new(pointer) {
            pointer_round_trips(value)?;
        }
        total::<*mut u8>(pointer)?;
        total::<*const u8>(pointer)?;
        total_zero_niche::<NonNull<u8>>(pointer)?;
        total::<Option<NonNull<u8>>>(pointer)
    }

    /// Panics with the broken law's message, where `laws` broke one.
    #[track_caller]
    fn assert_holds(laws: Result<(), TestCaseError>) {
        if let Err(broken) = laws {
            panic!("{broken}");
        }
    }

    #[test]
    fn every_byte_obeys_the_laws() {
        for bits in 0..=u128::from(u8::MAX) {
            assert_holds(narrow_integer_laws(bits, bits.wrapping_add(1)));
            let byte = u8::from_bits(bits);
            assert_holds(canonical::<()>(byte));
            assert_holds(canonical::<PhantomData<str>>(byte));
            assert_holds(canonical::<Option<()>>(byte));
            assert_holds(canonical::<Option<Option<()>>>(byte));
            assert_holds(canonical::<Option<PhantomData<str>>>(byte));
        }
        for value in [false, true] {
            assert_holds(total::<bool>(value));
            assert_holds(round_trips(value));
        }
        assert_holds(zero_decodes::<()>());
        assert_holds(zero_decodes::<PhantomData<str>>());
        assert_holds(zero_decodes::<Option<()>>());
        assert_holds(round_trips(()));
        assert_holds(round_trips(PhantomData::<str>));
        assert_holds(round_trips(Some(())));
        assert_holds(round_trips(None::<()>));
        assert_holds(round_trips(None::<Option<()>>));
    }

    #[test]
    fn every_edge_obeys_the_laws() {
        for bits in edges(&CHAR_EDGES) {
            assert_holds(narrow_integer_laws(bits, bits.wrapping_sub(1)));
            assert_holds(char_laws(bits, char::MAX));
            assert_holds(float_laws(bits));
            assert_holds(pointer_laws(bits));
            #[cfg(wide)]
            assert_holds(wide_laws(bits, bits.wrapping_sub(1)));
        }
    }

    /// The bits of `None`'s repr as an `Option<T>`.
    fn none_bits<T: Atom>() -> u128
    where
        T::Repr: ExactBits,
    {
        None::<T>.to_repr().to_bits()
    }

    /// Checks the `None` law for each integer's `NonZero`.
    macro_rules! nonzero_none_laws {
        ($($int:ty),+) => {$(
            assert_holds(none_lies_outside::<NonZero<$int>>(none_bits::<NonZero<$int>>()));
        )+};
    }

    #[test]
    fn none_lies_outside_each_built_in_range() {
        nonzero_none_laws!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);
        #[cfg(wide)]
        nonzero_none_laws!(u128, i128);
        assert_holds(none_lies_outside::<char>(none_bits::<char>()));
        assert_holds(none_lies_outside::<Option<char>>(none_bits::<Option<char>>()));
        assert_holds(none_lies_outside::<()>(none_bits::<()>()));
        assert_holds(none_lies_outside::<Option<()>>(none_bits::<Option<()>>()));
        assert_holds(none_lies_outside::<PhantomData<str>>(none_bits::<PhantomData<str>>()));
        let null = None::<NonNull<u8>>.to_repr().addr();
        assert_holds(none_lies_outside::<NonNull<u8>>(
            u128::try_from(null).expect("an address fits a `u128`"),
        ));
    }

    #[test]
    fn an_option_of_a_nonzero_or_non_null_is_total() {
        /// Compiles only where `T` declares every repr decodes.
        const fn is_total<T: Atom<Validity = Total>>() {}
        is_total::<Option<NonZero<u8>>>();
        is_total::<Option<NonZero<i64>>>();
        is_total::<Option<NonNull<u8>>>();
    }

    proptest! {
        #[test]
        fn integers_obey_the_laws(bits in edge_or_random_bits(&[]), other: u128) {
            narrow_integer_laws(bits, other)?;
        }

        #[test]
        fn chars_obey_the_laws(
            bits in prop_oneof![edge_or_random_bits(&CHAR_EDGES), 0..=0x11_0001_u128], other: char,
        ) {
            char_laws(bits, other)?;
        }

        #[test]
        fn floats_keep_every_bit(bits in edge_or_random_bits(&[])) {
            float_laws(bits)?;
        }

        #[test]
        fn pointers_obey_the_laws(bits in edge_or_random_bits(&[])) {
            pointer_laws(bits)?;
        }
    }

    #[cfg(wide)]
    proptest! {
        #[test]
        fn wide_values_obey_the_laws(bits in edge_or_random_bits(&[]), other: u128) {
            wide_laws(bits, other)?;
        }
    }
}
