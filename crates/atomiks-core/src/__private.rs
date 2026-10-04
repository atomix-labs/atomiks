//! What the code `#[derive(Atom)]` writes calls: hidden, since nothing else should call it.
//!
//! A derived impl for a type with no parameters calls the codecs here rather than `Atom`'s methods:
//! their bound is `const`, never `[const]`, so the call needs no `const_trait_impl` in the crate
//! that derives.

use core::any::type_name;

use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd};
use crate::message::{Message, refuse};
use crate::primitive::{CompareExchange, ExactBits, Primitive};
use crate::range::{FieldLayout, ReprRange};
pub use crate::range::{NicheLayout, PackedField, PackedLayout};
use crate::validity::{Partial, Total, TotalZeroNiche, Validity, ZeroNiche, ZeroValid};

/// `value`'s repr: [`Atom::to_repr`].
#[inline]
#[must_use]
pub const fn to_repr<T: const Atom>(value: T) -> T::Repr {
    value.to_repr()
}

/// The value `repr` encodes: [`Atom::from_repr`].
#[inline]
#[must_use]
pub const fn from_repr<T: const Atom>(repr: T::Repr) -> Option<T> {
    T::from_repr(repr)
}

/// The value `repr` encodes, without the check: [`Atom::from_repr_unchecked`].
///
/// # Safety
/// `repr` decodes: `T::from_repr(repr)` is `Some`.
#[expect(unsafe_code, reason = "forwards `Atom::from_repr_unchecked`, and its contract")]
#[inline]
#[must_use]
pub const unsafe fn from_repr_unchecked<T: const Atom>(repr: T::Repr) -> T {
    // SAFETY: the caller's repr decodes.
    unsafe { T::from_repr_unchecked(repr) }
}

/// A repr a packed value stores a field as: one whose bits are its whole value, never a pointer,
/// whose bits do not hold its provenance.
#[diagnostic::on_unimplemented(
    message = "a field stored as `{Self}` cannot be packed beside others",
    label = "a pointer, whose bits do not hold its provenance",
    note = "a newtype, a struct of one field beside any `PhantomData` markers, stores a pointer"
)]
pub impl(crate) const trait FieldRepr: [const] ExactBits {}

#[diagnostic::do_not_recommend]
const impl<R: [const] ExactBits> FieldRepr for R {}

/// The unsigned bits of `value`'s repr, as a packed value stores its field of `F`.
///
/// `F`'s repr must be a [`FieldRepr`]; the decodes need no more than a [`Primitive`], so a field
/// stored as a pointer is refused here alone.
#[inline]
#[must_use]
pub const fn to_bits<F: const Atom<Repr: const FieldRepr>>(value: F) -> u128 {
    value.to_repr().to_bits()
}

/// The value whose repr's unsigned bits are `bits`, or `None` where that repr does not decode: a
/// packed value's field of `F`, read back.
#[inline]
#[must_use]
pub const fn from_bits<F: const Atom>(bits: u128) -> Option<F> {
    F::from_repr(Primitive::from_bits(bits))
}

/// The value whose repr's unsigned bits are `bits`, without the check: [`from_bits`].
///
/// # Safety
/// The repr decodes: `from_bits::<F>(bits)` is `Some`.
#[expect(unsafe_code, reason = "forwards `Atom::from_repr_unchecked`, and its contract")]
#[inline]
#[must_use]
pub const unsafe fn from_bits_unchecked<F: const Atom>(bits: u128) -> F {
    // SAFETY: the caller's bits are a repr that decodes.
    unsafe { F::from_repr_unchecked(Primitive::from_bits(bits)) }
}

/// Compiles only where `T` is `Send` and `Sync`, as a derived value must be, or each marker beside
/// a newtype's pointer: an `Atomic` of it is both, whatever the value's own auto traits say.
#[inline]
pub const fn assert_send_and_sync<T: Send + Sync>() {}

/// Compiles only where `T`'s repr is `R`: the repr a derived value states, checked against the
/// field whose repr it takes.
#[inline]
pub const fn assert_repr<T: Atom<Repr = R>, R>() {}

/// Compiles only where `T` has [`AtomAdd`], as the field a derived `AtomAdd` takes it from must.
#[inline]
pub const fn assert_atom_add<T: AtomAdd>() {}

/// Compiles only where `T` has [`AtomOrd`], as the field a derived `AtomOrd` takes it from must.
#[inline]
pub const fn assert_atom_ord<T: AtomOrd>() {}

/// Compiles only where `T` has [`AtomBitwise`], as the field a derived `AtomBitwise` takes it from
/// must.
#[inline]
pub const fn assert_atom_bitwise<T: AtomBitwise>() {}

/// The widest integer an atomic cell holds on this target, in bits.
const WIDEST: u32 = if cfg!(wide) { u128::BITS } else { u64::BITS };

/// A width in bits, 8 to 128, which [`SelectRepr`] maps to the unsigned integer that wide.
#[derive(Debug)]
pub struct Width<const BITS: u32>;

/// The repr a derived value is stored as: the unsigned integer of a [`Width`], or the integer a
/// `#[repr]` or `#[atom(repr = …)]` names.
///
/// Where no atomic cell on this target holds that integer, the widest one that is held, so the
/// selection never fails and [`assert_width`] refuses the value once.
pub impl(crate) trait SelectRepr {
    /// The integer.
    type Repr: const ExactBits + CompareExchange;
}

/// Implements `SelectRepr` for each width or integer, mapping it to its repr.
macro_rules! select {
    ($($from:ty => $repr:ty),+ $(,)?) => {$(
        impl SelectRepr for $from {
            type Repr = $repr;
        }
    )+};
}

select! {
    Width<8> => u8, Width<16> => u16, Width<32> => u32, Width<64> => u64,
    u8 => u8, u16 => u16, u32 => u32, u64 => u64, usize => usize,
    i8 => i8, i16 => i16, i32 => i32, i64 => i64, isize => isize,
}
#[cfg(wide)]
select!(Width<128> => u128, u128 => u128, i128 => i128);
#[cfg(not(wide))]
select!(u128 => u64, i128 => i64);

/// The [`Width`] of the narrowest integer that holds `bits` bits: 8, 16, 32, 64 or 128.
///
/// Where none an atomic cell holds on this target is that wide, the widest, which [`assert_width`]
/// refuses, so the selection itself never fails.
#[inline]
#[must_use]
pub const fn narrowest_width(bits: u32) -> u32 {
    if bits <= u8::BITS {
        u8::BITS
    } else if bits <= WIDEST {
        bits.next_power_of_two()
    } else {
        WIDEST
    }
}

/// Refuses the build of `T`, a value `bits` bits wide, where no integer an atomic cell holds on
/// this target is that wide.
///
/// # Panics
/// Where `bits` is past the widest such integer; in a constant, the build fails instead.
#[inline]
#[track_caller]
pub const fn assert_width<T: ?Sized>(bits: u32) {
    if bits <= WIDEST {
        return;
    }
    let advice = if bits > u128::BITS {
        " bits, but an atomic word holds at most 128: narrow a field, or split the value"
    } else {
        " bits, but an atomic word holds at most 64: build with `-C target-cpu=x86-64-v2` or newer for 128"
    };
    refuse_width::<T>(bits, &Message::new().text(advice));
}

/// Refuses the build of `T`, a value `bits` bits wide, where the integer `R` it states as its repr
/// is narrower.
///
/// `R` is the integer stated, whether or not an atomic cell on this target holds it: where none
/// does, [`assert_width`] refuses the type, and this, which reads `R`'s width alone, does not
/// again.
///
/// # Panics
/// Where `bits` is past `R`'s width; in a constant, the build fails instead.
#[inline]
#[track_caller]
pub const fn assert_stated_width<T: ?Sized, R>(bits: u32) {
    // An integer's bits are its bytes', of which it has at most 16.
    let repr_bits = match u32::try_from(size_of::<R>()) {
        Ok(bytes) => bytes.saturating_mul(u8::BITS),
        Err(_never) => u32::MAX,
    };
    if bits <= repr_bits {
        return;
    }
    let has = Message::new().text("` has ").number(u128::from(repr_bits));
    let has = has.as_str();
    refuse_width::<T>(
        bits,
        &Message::new().text(" bits, but its repr `").name(type_name::<R>(), has.len()).text(has),
    );
}

/// Refuses the build of `T`, a value `bits` bits wide, with `advice` after the bits: "`Wide` needs
/// 129 bits, but …".
///
/// Cuts `T`'s name short where the rest would not fit after it.
#[track_caller]
const fn refuse_width<T: ?Sized>(bits: u32, advice: &Message) -> ! {
    let rest = Message::new().text("` needs ").number(u128::from(bits)).text(advice.as_str());
    let rest = rest.as_str();
    refuse(&Message::new().text("`").name(type_name::<T>(), rest.len()).text(rest))
}

/// How many bits a fieldless enum needs for its `discriminants`, read as `i128`s.
///
/// The width of the narrowest field that holds each, unsigned or two's complement, by which an enum
/// without a `#[repr]` selects its repr.
#[inline]
#[must_use]
pub const fn discriminant_width<const COUNT: usize>(discriminants: [i128; COUNT]) -> u32 {
    // Zero widens neither field, unsigned or two's complement, so both extremes start there.
    let (mut smallest, mut largest) = (0, 0);
    let mut rest: &[i128] = &discriminants;
    while let [discriminant, after @ ..] = rest {
        smallest = if *discriminant < smallest { *discriminant } else { smallest };
        largest = if *discriminant > largest { *discriminant } else { largest };
        rest = after;
    }
    FieldLayout::from_signed(smallest, largest).width()
}

/// The number of a fieldless enum's validity, whose `discriminants` in its repr `R` are distinct.
///
/// [`Total`]'s where they are every repr, [`TotalZeroNiche`]'s every one but zero, else
/// [`ZeroValid`]'s where one is zero and [`ZeroNiche`]'s where none is.
#[inline]
#[must_use]
pub const fn discriminant_validity_code<R: const ExactBits, const COUNT: usize>(
    discriminants: [R; COUNT],
) -> u8 {
    let mut zero_decodes = false;
    let mut rest: &[R] = &discriminants;
    while let [discriminant, after @ ..] = rest {
        zero_decodes |= discriminant.to_bits() == 0;
        rest = after;
    }
    // How many reprs `R` has, where a `usize` counts them; an enum of more variants cannot exist.
    let reprs = 1_usize.checked_shl(R::BITS);
    if zero_decodes {
        if matches!(reprs, Some(reprs) if reprs == COUNT) { TOTAL } else { ZERO_VALID }
    } else if matches!(reprs, Some(reprs) if reprs.wrapping_sub(1) == COUNT) {
        TOTAL_ZERO_NICHE
    } else {
        ZERO_NICHE
    }
}

/// The range of a fieldless enum whose discriminants in its repr `R` are `discriminants`: the
/// narrowest that holds each, every repr but the longest run, counted round through zero, that
/// holds none of them.
///
/// # Panics
/// Where `discriminants` is empty, since a range holds at least one repr; in a constant,
/// the build fails instead.
#[inline]
#[must_use]
#[track_caller]
pub const fn discriminant_range<R: const ExactBits, const COUNT: usize>(
    discriminants: [R; COUNT],
) -> ReprRange<R> {
    let mut bits = [0; COUNT];
    let (mut slots, mut rest): (&mut [u128], &[R]) = (&mut bits, &discriminants);
    while let ([slot, slots_after @ ..], [discriminant, rest_after @ ..]) = (slots, rest) {
        *slot = discriminant.to_bits();
        (slots, rest) = (slots_after, rest_after);
    }
    ReprRange::enclosing(&mut bits)
}

/// A validity named by a number, which a constant computes and [`SelectValidity`] maps to the
/// validity's type.
#[derive(Debug)]
pub struct ValidityCode<const CODE: u8>;

/// The validity a [`ValidityCode`] names.
pub impl(crate) trait SelectValidity {
    /// The validity.
    type Validity: const Validity;
}

/// Names each validity by a number, and implements `SelectValidity` for it: public where derived
/// code names the number.
macro_rules! validity_codes {
    ($($visibility:vis $name:ident = $number:literal => $validity:ident;)+) => {$(
        #[doc = concat!("The number of [`", stringify!($validity), "`].")]
        $visibility const $name: u8 = $number;

        impl SelectValidity for ValidityCode<$number> {
            type Validity = $validity;
        }
    )+};
}

validity_codes! {
    PARTIAL = 0 => Partial;
    ZERO_VALID = 1 => ZeroValid;
    ZERO_NICHE = 2 => ZeroNiche;
    TOTAL = 3 => Total;
    TOTAL_ZERO_NICHE = 4 => TotalZeroNiche;
}

/// What a packed value's fields promise of its reprs, folded field by field from
/// [`EMPTY`](Self::EMPTY), from which a concrete derived impl picks its validity.
///
/// A field's bits are read back zero- or sign-extended to a repr of its own primitive, so its
/// validity speaks for every pattern of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedValidity {
    /// Every pattern of each field's bits decodes.
    every_pattern_decodes: bool,
    /// The zero repr decodes: each field's does.
    zero_decodes: bool,
    /// The zero repr does not decode, so `Option`'s `None` must take it: some field's does not.
    none_takes_zero: bool,
}

impl PackedValidity {
    /// The promises of a value of no fields, whose one repr, zero, decodes.
    pub const EMPTY: Self =
        Self { every_pattern_decodes: true, zero_decodes: true, none_takes_zero: false };

    /// The promises with a field of validity `V`, stored in `layout`, added.
    #[inline]
    #[must_use]
    pub const fn with_field<V: Validity>(self, layout: FieldLayout) -> Self {
        // A field of no bits reads back zero alone.
        let every_pattern =
            V::PROMISES_EVERY_REPR_DECODES || (layout.width() == 0 && V::PROMISES_ZERO_DECODES);
        Self {
            every_pattern_decodes: self.every_pattern_decodes && every_pattern,
            zero_decodes: self.zero_decodes && V::PROMISES_ZERO_DECODES,
            none_takes_zero: self.none_takes_zero || V::NONE_TAKES_ZERO,
        }
    }

    /// The number of the strongest validity these promises give a packed value `width` bits wide,
    /// in a repr `repr_width` bits wide: [`Total`]'s where every pattern decodes and no bit lies
    /// above the fields, then [`ZeroValid`]'s, [`ZeroNiche`]'s or [`Partial`]'s, by what is
    /// promised of zero.
    #[inline]
    #[must_use]
    pub const fn code(self, width: u32, repr_width: u32) -> u8 {
        if self.every_pattern_decodes && width == repr_width {
            TOTAL
        } else if self.zero_decodes {
            ZERO_VALID
        } else if self.none_takes_zero {
            ZERO_NICHE
        } else {
            PARTIAL
        }
    }
}

#[cfg(test)]
mod tests {
    use core::array;
    use core::num::NonZero;

    use super::{
        FieldLayout, PARTIAL, PackedField, PackedValidity, SelectRepr, SelectValidity, TOTAL,
        TOTAL_ZERO_NICHE, ValidityCode, Width, ZERO_NICHE, ZERO_VALID, assert_stated_width,
        assert_width, discriminant_range, discriminant_validity_code, discriminant_width,
        from_bits, from_bits_unchecked, narrowest_width, to_bits,
    };
    use crate::range::ReprRange;
    use crate::validity::{Partial, Total, TotalZeroNiche, Validity, ZeroNiche, ZeroValid};

    #[test]
    fn a_fields_bits_are_its_reprs_and_decode_as_it_does() {
        assert_eq!(to_bits(-1_i8), 0xFF, "an `i8`'s -1, unsigned");
        assert_eq!(to_bits(NonZero::<u16>::MAX), 0xFFFF, "a `NonZero`'s, its integer's");
        assert_eq!(from_bits::<NonZero<u8>>(7), NonZero::new(7), "a repr that decodes");
        assert_eq!(from_bits::<NonZero<u8>>(0), None, "and not one that does not");
        // SAFETY: 7 is a `NonZero<u8>`'s repr.
        #[expect(unsafe_code, reason = "the unchecked decode under test")]
        let seven = unsafe { from_bits_unchecked::<NonZero<u8>>(7) };
        assert_eq!(Some(seven), NonZero::new(7), "alike unchecked");
    }

    /// How a field of eight bits is stored.
    const BYTE: FieldLayout = PackedField::new(ReprRange::<u8>::FULL, 0).layout();
    /// How a field of no bits, whose one repr is zero, is stored.
    const NO_BITS: FieldLayout = PackedField::new(ReprRange::<u8>::new(0, 0), 0).layout();

    /// Compiles only where `ValidityCode<CODE>` names `V`.
    const fn names<const CODE: u8, V: Validity>()
    where
        ValidityCode<CODE>: SelectValidity<Validity = V>,
    {
    }

    #[test]
    fn each_code_names_its_validity() {
        names::<PARTIAL, Partial>();
        names::<ZERO_VALID, ZeroValid>();
        names::<ZERO_NICHE, ZeroNiche>();
        names::<TOTAL, Total>();
        names::<TOTAL_ZERO_NICHE, TotalZeroNiche>();
    }

    /// The code of a value of two byte fields, of validities `V` and `W`, in a repr `repr_width`
    /// bits wide.
    const fn code_of_two_bytes<V: Validity, W: Validity>(repr_width: u32) -> u8 {
        PackedValidity::EMPTY.with_field::<V>(BYTE).with_field::<W>(BYTE).code(16, repr_width)
    }

    /// Checks `code_of_two_bytes` for each row: a field's validity, then the code of a value of it
    /// and a field of each kind, in the order of `Total`, `TotalZeroNiche`, `ZeroValid`,
    /// `ZeroNiche` and `Partial`, filling its repr and narrower than it.
    macro_rules! meet {
        ($($first:ident => $($full:ident / $narrower:ident),+;)+) => {$(
            meet!(
                @row $first => [Total, TotalZeroNiche, ZeroValid, ZeroNiche, Partial]
                [$($full / $narrower),+]
            );
        )+};
        (@row $first:ident => [$($second:ident),+] [$($full:ident / $narrower:ident),+]) => {$(
            assert_eq!(
                code_of_two_bytes::<$first, $second>(16),
                $full,
                concat!(stringify!($first), " and ", stringify!($second), " filling their repr")
            );
            assert_eq!(
                code_of_two_bytes::<$first, $second>(32),
                $narrower,
                concat!(stringify!($first), " and ", stringify!($second), " in a wider repr")
            );
        )+};
    }

    #[test]
    fn a_packed_value_promises_what_each_field_does() {
        meet! {
            Total => TOTAL / ZERO_VALID, ZERO_NICHE / ZERO_NICHE, ZERO_VALID / ZERO_VALID,
                ZERO_NICHE / ZERO_NICHE, PARTIAL / PARTIAL;
            TotalZeroNiche => ZERO_NICHE / ZERO_NICHE, ZERO_NICHE / ZERO_NICHE,
                ZERO_NICHE / ZERO_NICHE, ZERO_NICHE / ZERO_NICHE, ZERO_NICHE / ZERO_NICHE;
            ZeroValid => ZERO_VALID / ZERO_VALID, ZERO_NICHE / ZERO_NICHE, ZERO_VALID / ZERO_VALID,
                ZERO_NICHE / ZERO_NICHE, PARTIAL / PARTIAL;
            ZeroNiche => ZERO_NICHE / ZERO_NICHE, ZERO_NICHE / ZERO_NICHE,
                ZERO_NICHE / ZERO_NICHE, ZERO_NICHE / ZERO_NICHE, ZERO_NICHE / ZERO_NICHE;
            Partial => PARTIAL / PARTIAL, ZERO_NICHE / ZERO_NICHE, PARTIAL / PARTIAL,
                ZERO_NICHE / ZERO_NICHE, PARTIAL / PARTIAL;
        }
    }

    /// Whether `V` and `W` together promise that zero decodes, by their zero validity.
    fn zero_meet_decodes<V: Validity, W: Validity>() -> bool {
        <V::ZeroValidityWith<W> as Validity>::PROMISES_ZERO_DECODES
    }

    /// Checks, for each pair of validities, that the fold and the zero validity agree on zero.
    macro_rules! agree {
        ($($first:ident),+) => {$(
            agree!(@row $first => Total, TotalZeroNiche, ZeroValid, ZeroNiche, Partial);
        )+};
        (@row $first:ident => $($second:ident),+) => {$(
            assert_eq!(
                matches!(code_of_two_bytes::<$first, $second>(32), ZERO_VALID),
                zero_meet_decodes::<$first, $second>(),
                concat!(stringify!($first), " and ", stringify!($second))
            );
        )+};
    }

    #[test]
    fn the_fold_and_the_zero_validity_agree_on_zero() {
        agree!(Total, TotalZeroNiche, ZeroValid, ZeroNiche, Partial);
    }

    #[test]
    fn a_field_of_no_bits_decodes_its_every_pattern_where_zero_does() {
        let marked =
            PackedValidity::EMPTY.with_field::<Total>(BYTE).with_field::<ZeroValid>(NO_BITS);
        assert_eq!(marked.code(8, 8), TOTAL, "a byte and a marker fill a `u8`");
        let unknown =
            PackedValidity::EMPTY.with_field::<Total>(BYTE).with_field::<Partial>(NO_BITS);
        assert_eq!(unknown.code(8, 8), PARTIAL, "but not beside a field that promises nothing");
    }

    #[test]
    fn a_value_of_no_fields_promises_only_that_zero_decodes() {
        assert_eq!(PackedValidity::EMPTY.code(0, 8), ZERO_VALID, "zero alone, in a `u8`");
    }

    #[test]
    fn a_selected_validity_is_its_type() {
        names::<
            {
                PackedValidity::EMPTY
                    .with_field::<Total>(BYTE)
                    .with_field::<TotalZeroNiche>(BYTE)
                    .code(16, 16)
            },
            ZeroNiche,
        >();
        names::<{ PackedValidity::EMPTY.with_field::<Total>(BYTE).code(8, 8) }, Total>();
    }

    /// Compiles only where `S` selects the repr `R`.
    const fn selects<S: SelectRepr<Repr = R>, R>() {}

    #[test]
    fn a_width_selects_the_unsigned_integer_that_wide_and_an_integer_itself() {
        selects::<Width<8>, u8>();
        selects::<Width<16>, u16>();
        selects::<Width<32>, u32>();
        selects::<Width<64>, u64>();
        selects::<i8, i8>();
        selects::<usize, usize>();
        selects::<isize, isize>();
    }

    #[test]
    fn the_selected_width_is_the_narrowest_that_holds_the_bits() {
        assert_eq!(narrowest_width(0), 8, "no bits, in a byte");
        assert_eq!(narrowest_width(8), 8, "a byte");
        assert_eq!(narrowest_width(9), 16, "one bit more, in two");
        assert_eq!(narrowest_width(33), 64, "past 32, in 64");
        assert_eq!(narrowest_width(64), 64, "64 bits");
    }

    /// The value whose width each test checks, and which a refusal names.
    struct Wide;

    #[cfg(wide)]
    #[test]
    fn a_target_with_128_bit_atomics_selects_and_holds_128_bits() {
        selects::<Width<128>, u128>();
        selects::<i128, i128>();
        assert_eq!(narrowest_width(65), 128, "past 64, in 128");
        assert_eq!(narrowest_width(200), 128, "and past 128, the widest, which is refused");
        assert_width::<Wide>(128);
    }

    #[cfg(not(wide))]
    #[test]
    fn a_target_without_128_bit_atomics_selects_the_widest_it_holds() {
        selects::<u128, u64>();
        selects::<i128, i64>();
        assert_eq!(narrowest_width(65), 64, "past 64, the widest, which is refused");
    }

    #[cfg(not(wide))]
    #[test]
    #[should_panic(
        expected = "Wide` needs 65 bits, but an atomic word holds at most 64: build with `-C target-cpu=x86-64-v2` or newer for 128"
    )]
    fn a_value_wider_than_this_targets_atomic_words_is_refused() {
        assert_width::<Wide>(65);
    }

    #[test]
    #[should_panic(
        expected = "Wide` needs 129 bits, but an atomic word holds at most 128: narrow a field, or split the value"
    )]
    fn a_value_wider_than_any_atomic_word_is_refused() {
        assert_width::<Wide>(129);
    }

    #[test]
    fn a_value_as_wide_as_its_stated_repr_is_held() {
        assert_stated_width::<Wide, u64>(64);
    }

    #[test]
    #[should_panic(expected = "Wide` needs 65 bits, but its repr `u64` has 64")]
    fn a_value_wider_than_its_stated_repr_is_refused() {
        assert_stated_width::<Wide, u64>(65);
    }

    #[test]
    fn discriminants_need_the_narrower_of_an_unsigned_and_a_signed_field() {
        assert_eq!(discriminant_width([0, 1]), 1, "0 and 1, unsigned");
        assert_eq!(discriminant_width([-1, 0, 1]), 2, "-1 to 1, signed");
        assert_eq!(discriminant_width([200, 201]), 8, "200 and 201, unsigned");
        assert_eq!(discriminant_width([-128, 127]), 8, "a byte's signed extremes");
        assert_eq!(discriminant_width([-129]), 9, "one below");
        assert_eq!(discriminant_width([-5, -3]), 4, "negatives alone, signed");
        assert_eq!(discriminant_width([0, 1 << 40]), 41, "past 32 bits");
        assert_eq!(discriminant_width([i64::MIN.into()]), 64, "the lowest `isize`");
        assert_eq!(discriminant_width([0]), 0, "zero alone, in no bits");
    }

    /// Every byte, as a discriminant.
    fn every_byte() -> [u8; 256] {
        array::from_fn(|byte| u8::try_from(byte).expect("an index below 256 is a byte"))
    }

    #[test]
    fn discriminants_are_total_where_they_take_every_repr_and_else_say_whether_zero_decodes() {
        assert_eq!(discriminant_validity_code(every_byte()), TOTAL, "every byte");
        let nonzero: [u8; 255] = array::from_fn(|index| every_byte()[index + 1]);
        assert_eq!(discriminant_validity_code(nonzero), TOTAL_ZERO_NICHE, "every byte but zero");
        let below_the_top: [u8; 255] = array::from_fn(|index| every_byte()[index]);
        assert_eq!(discriminant_validity_code(below_the_top), ZERO_VALID, "every byte but 255");
        assert_eq!(discriminant_validity_code([0_u8, 1]), ZERO_VALID, "zero among a few");
        assert_eq!(discriminant_validity_code([-1_i8, 0, 1]), ZERO_VALID, "and signed");
        assert_eq!(discriminant_validity_code([1_u16, 2]), ZERO_NICHE, "no zero");
        assert_eq!(discriminant_validity_code([1_u64]), ZERO_NICHE, "no zero in a wide repr");
    }

    #[test]
    fn discriminants_take_the_narrowest_range_that_holds_their_bits() {
        assert_eq!(discriminant_range([2_u8, 0, 1]), ReprRange::new(0, 2), "unsorted");
        assert_eq!(discriminant_range([-1_i8, 0, 1]), ReprRange::from_signed(-1, 1), "signed");
        assert_eq!(discriminant_range([0_u8, 255]), ReprRange::from_signed(-1, 0), "through zero");
        assert_eq!(discriminant_range(every_byte()), ReprRange::FULL, "every byte");
    }
}
