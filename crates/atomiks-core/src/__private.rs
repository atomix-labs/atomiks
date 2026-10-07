//! What the code `#[derive(Atom)]` writes calls: hidden, since nothing else should call it.
//!
//! A derived impl for a type with no parameters calls the codecs here rather than `Atom`'s methods:
//! their bound is `const`, never `[const]`, so the call needs no `const_trait_impl` in the crate
//! that derives.

use core::any::type_name;
use core::ptr;

use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd, PtrAtom};
pub use crate::atomic::{HasPackedField, Reach, project_field};
use crate::message::{Message, refuse};
use crate::primitive::{CompareExchange, ExactBits, Primitive};
#[cfg(wide)]
use crate::primitive::{DoubleWord, Word};
pub use crate::range::{
    EnumLayout, PackedField, PackedLayout, PointeeAlignment, PointerEnumLayout, PointerEnumVariant,
    PointerWordLayout, PointerWordRepr, Tags, assert_aligned,
};
use crate::range::{FieldLayout, ReprRange};
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

/// `value`'s repr with `tags` set in its low bits: [`Atom::to_tagged_repr`].
///
/// A pointer word passes its tags on to its pointer field through it, and a newtype to its field.
#[inline]
#[must_use]
pub const fn to_tagged_repr<F: const Atom>(value: F, tags: Tags) -> (F::Repr, usize) {
    value.to_tagged_repr(tags)
}

/// The pointer `value`, a pointer enum's pointer field, is stored as, with `tags` set in its low
/// bits, as [`to_tagged_repr`] sets them.
///
/// Its bound refuses a field marked a pointer that is none, at its type.
#[inline]
#[must_use]
pub const fn to_tagged_pointer<F: const PtrAtom>(value: F, tags: Tags) -> (*mut (), usize) {
    let (pointer, misaligned) = value.to_tagged_repr(tags);
    (pointer.cast(), misaligned)
}

/// The value of a pointer enum's pointer field stored as `pointer`: [`Atom::from_repr`].
#[inline]
#[must_use]
pub const fn from_pointer<F: const PtrAtom>(pointer: *mut ()) -> Option<F> {
    F::from_repr(pointer.cast())
}

/// The value of a pointer enum's pointer field stored as `pointer`, without the check:
/// [`Atom::from_repr_unchecked`].
///
/// # Safety
/// `pointer` decodes: `F::from_repr(pointer.cast())` is `Some`.
#[expect(unsafe_code, reason = "forwards `Atom::from_repr_unchecked`, and its contract")]
#[inline]
#[must_use]
pub const unsafe fn from_pointer_unchecked<F: const PtrAtom>(pointer: *mut ()) -> F {
    // SAFETY: the caller's pointer decodes.
    unsafe { F::from_repr_unchecked(pointer.cast()) }
}

/// A repr a packed value stores a field as: one whose bits are its whole value, never a pointer,
/// whose bits do not hold its provenance.
#[diagnostic::on_unimplemented(
    message = "a field stored as `{Self}` cannot be packed beside others",
    label = "a pointer, whose bits do not hold its provenance",
    note = "a pointer is stored alone, in a newtype or an `Option`, or as the one pointer field of a pointer word or of a pointer enum's variant, marked `#[atom(ptr)]` where its type shows no pointer"
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

/// Compiles only where `F` is stored as a pointer, which a newtype claims of its field where it is
/// written as one or marked `#[atom(ptr)]`.
#[inline]
pub const fn assert_pointer<F: PtrAtom>() {}

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

/// Whether `field`, of a packed value in the repr `R`, ends at the repr's top bit: the constant
/// that selects a field's [`Reach`].
#[inline]
#[must_use]
pub const fn reaches_top<R: Primitive>(field: PackedField) -> bool {
    field.layout().width() != 0 && field.next_offset() == R::BITS
}

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

/// A count of words, one or two, which [`SelectPointerWordRepr`] maps to the repr of a value of a
/// pointer field and tag fields that many words wide.
#[derive(Debug)]
pub struct Words<const COUNT: u32>;

/// The repr of a value of a pointer field, stored as `P`, and tag fields, a [`Words`] wide.
///
/// It is `P` in one word, the tags in its low bits, or, in two, a `DoubleWord` of `P` beside an
/// integer word of the tags, as [`PointerWordLayout::word_count`] counts them. Where no atomic
/// holds two words, two words select `P` too, so the selection itself never fails, and
/// [`PointerWordLayout::assert_tags_fit`] refuses the value once, naming the CPU it needs.
pub impl(crate) trait SelectPointerWordRepr<P> {
    /// The repr.
    type Repr: const Primitive + CompareExchange + const PointerWordRepr<P>;
}

impl<P: const Primitive + CompareExchange> SelectPointerWordRepr<P> for Words<1> {
    type Repr = P;
}

#[cfg(wide)]
impl<P: const Word> SelectPointerWordRepr<P> for Words<2> {
    type Repr = DoubleWord<P, usize>;
}

#[cfg(not(wide))]
impl<P: const Primitive + CompareExchange> SelectPointerWordRepr<P> for Words<2> {
    type Repr = P;
}

/// Writes the items of a value two words wide whatever its fields, `{ items }`; or, where no
/// atomic holds two words, refuses the value named `name` once, naming the CPU it needs, and writes
/// `else { stub }`, an `Atom` impl that keeps each use of the value from a refusal of its own.
///
/// atomiks' own `cfg` chooses, as it chooses whether a `DoubleWord` exists, so the code a derive
/// writes agrees with the atomiks it builds against, whatever flags read the deriving crate.
#[cfg(wide)]
#[doc(hidden)]
#[macro_export]
macro_rules! __in_two_words {
    ($name:literal { $($items:tt)* } else { $($stub:tt)* }) => {
        $($items)*
    };
}

/// The refusal of the value `name` and its stub, where no atomic holds two words: as the macro
/// where one does says.
#[cfg(not(wide))]
#[doc(hidden)]
#[macro_export]
macro_rules! __in_two_words {
    ($name:literal { $($items:tt)* } else { $($stub:tt)* }) => {
        ::core::compile_error!(::core::concat!(
            "`",
            ::core::module_path!(),
            "::",
            $name,
            "` needs two words, but an atomic holds at most one: build with `-C target-cpu=x86-64-v2` or newer for two"
        ));
        $($stub)*
    };
}

pub use crate::__in_two_words as in_two_words;

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
    pub PARTIAL = 0 => Partial;
    pub ZERO_VALID = 1 => ZeroValid;
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

/// What an enum with fields promises of its reprs, folded variant by variant from
/// [`new`](Self::new), from which a concrete derived impl picks its validity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnumValidity {
    /// How the enum lays out its variants.
    layout: EnumLayout,
    /// The discriminant of the variant the zero repr holds, as unsigned bits.
    zero_discriminant: u128,
    /// How many variants are folded in.
    variant_count: u128,
    /// What the variants folded in promise of the bits below the selector, and of zero.
    promises: PackedValidity,
}

impl EnumValidity {
    /// The promises of an enum laid out in `layout`, before any variant: zero, until its variant
    /// is folded in, never decodes.
    #[inline]
    #[must_use]
    pub const fn new(layout: EnumLayout) -> Self {
        let promises = PackedValidity {
            every_pattern_decodes: true,
            zero_decodes: false,
            none_takes_zero: true,
        };
        Self { layout, zero_discriminant: layout.discriminant_bits(0), variant_count: 0, promises }
    }

    /// The promises with the unit variant of `discriminant` added.
    #[inline]
    #[must_use]
    pub const fn with_unit<D: const ExactBits>(self, discriminant: D) -> Self {
        self.with_variant(discriminant, PackedLayout::new(&[]), PackedValidity::EMPTY)
    }

    /// The promises with the variant of `discriminant` added, whose fields lie in `variant` and
    /// promise `fields`: every pattern of the bits below the selector decodes as it only where
    /// they fill those bits, and zero decodes as the variant the zero repr holds does.
    #[inline]
    #[must_use]
    pub const fn with_variant<D: const ExactBits>(
        self, discriminant: D, variant: PackedLayout, fields: PackedValidity,
    ) -> Self {
        let every_pattern =
            fields.every_pattern_decodes && self.layout.is_full_below_selector(variant);
        let zero =
            if discriminant.to_bits() == self.zero_discriminant { fields } else { self.promises };
        let promises = PackedValidity {
            every_pattern_decodes: self.promises.every_pattern_decodes && every_pattern,
            zero_decodes: zero.zero_decodes,
            none_takes_zero: zero.none_takes_zero,
        };
        Self { promises, variant_count: self.variant_count.saturating_add(1), ..self }
    }

    /// The number of the strongest validity these promises give the enum in a repr `repr_width`
    /// bits wide, as [`PackedValidity::code`] picks it: every pattern decodes only where each
    /// pattern of the selector says a variant, and each variant's fields decode every pattern of
    /// the bits below it.
    #[inline]
    #[must_use]
    pub const fn code(self, repr_width: u32) -> u8 {
        let every_pattern_decodes =
            self.promises.every_pattern_decodes && self.layout.is_selector_full(self.variant_count);
        PackedValidity { every_pattern_decodes, ..self.promises }
            .code(self.layout.width(), repr_width)
    }
}

/// What an enum whose variants hold a pointer promises of its zero repr, from which a concrete
/// derived impl picks its validity and range.
///
/// It is what the variant zero holds promises, folded variant by variant from
/// [`new`](Self::new). Nothing else is promised: no pointer's repr is read as bits, so none is
/// [`Total`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerEnumValidity {
    /// The discriminant of the variant the zero repr holds, as unsigned bits: the unit that fills
    /// the niche, or the variant of tag 0.
    zero_discriminant: u128,
    /// What that variant promises of zero, once it is folded in.
    promises: PackedValidity,
}

impl PointerEnumValidity {
    /// The promises of an enum laid out in `layout`, before any variant: zero, until its variant
    /// is folded in, never decodes.
    #[inline]
    #[must_use]
    pub const fn new(layout: PointerEnumLayout) -> Self {
        let promises = PackedValidity {
            every_pattern_decodes: false,
            zero_decodes: false,
            none_takes_zero: true,
        };
        Self { zero_discriminant: layout.discriminant_bits(ptr::null_mut()), promises }
    }

    /// The promises with the variant of `discriminant` added, whose fields promise `fields`: a
    /// unit's are [`PackedValidity::EMPTY`], a pointer variant's its pointer's and its tags'.
    #[inline]
    #[must_use]
    pub const fn with_variant<D: const ExactBits>(
        self, discriminant: D, fields: PackedValidity,
    ) -> Self {
        if discriminant.to_bits() == self.zero_discriminant {
            Self { promises: fields, ..self }
        } else {
            self
        }
    }

    /// The number of the strongest validity the promises give: [`ZeroValid`]'s where the variant
    /// zero holds promises its zero decodes, [`ZeroNiche`]'s where it promises it does not, or
    /// where no variant's tag is zero, else [`Partial`]'s.
    #[inline]
    #[must_use]
    pub const fn code(self) -> u8 {
        self.promises.code(0, usize::BITS)
    }

    /// The enum's range, of its repr `R`: every repr but zero where zero never decodes, else every
    /// repr.
    #[inline]
    #[must_use]
    pub const fn range<R: Primitive>(self) -> ReprRange<R> {
        if self.promises.none_takes_zero { ReprRange::NONZERO } else { ReprRange::FULL }
    }
}

#[cfg(test)]
mod tests {
    use core::array;
    use core::num::NonZero;
    use core::ptr::NonNull;

    use super::{
        EnumLayout, EnumValidity, FieldLayout, PARTIAL, PackedField, PackedLayout, PackedValidity,
        PointerEnumLayout, PointerEnumValidity, PointerEnumVariant, SelectRepr, SelectValidity,
        TOTAL, TOTAL_ZERO_NICHE, ValidityCode, Width, ZERO_NICHE, ZERO_VALID, assert_stated_width,
        assert_width, discriminant_range, discriminant_validity_code, discriminant_width,
        from_bits, from_bits_unchecked, narrowest_width, to_bits,
    };
    use crate::primitive::Primitive;
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

    /// The layout of a variant of `COUNT` fields of `reprs`, each of validity `V`, and what they
    /// promise.
    fn fields<F: Primitive, V: Validity, const COUNT: usize>(
        reprs: ReprRange<F>,
    ) -> (PackedLayout, PackedValidity) {
        let mut offset = 0;
        let placed: [PackedField; COUNT] = array::from_fn(|_| {
            let field = PackedField::new(reprs, offset);
            offset = field.next_offset();
            field
        });
        let promises = placed.iter().fold(PackedValidity::EMPTY, |promises, field| {
            promises.with_field::<V>(field.layout())
        });
        (PackedLayout::new(&placed), promises)
    }

    #[test]
    fn a_tagged_enum_promises_what_the_variant_of_tag_zero_does() {
        let (lap, laps) = fields::<u32, Total, 1>(ReprRange::FULL);
        let slot = EnumLayout::tagged(ReprRange::<u8>::new(0, 2), &[lap, lap]);
        let slot = EnumValidity::new(slot).with_unit(0_u8).with_variant(1_u8, lap, laps);
        assert_eq!(slot.with_variant(2_u8, lap, laps).code(64), ZERO_VALID, "a unit at tag 0");
        let (id, ids) = fields::<u8, TotalZeroNiche, 1>(ReprRange::NONZERO);
        let owned = EnumLayout::tagged(ReprRange::<u8>::new(0, 1), &[id]);
        let owned = EnumValidity::new(owned).with_variant(0_u8, id, ids).with_unit(1_u8);
        assert_eq!(owned.code(16), ZERO_NICHE, "a field at tag 0 whose zero never decodes");
        let late = EnumLayout::tagged(ReprRange::<u8>::new(1, 2), &[]);
        let late = EnumValidity::new(late).with_unit(1_u8).with_unit(2_u8);
        assert_eq!(late.code(8), ZERO_NICHE, "and no variant at tag 0");
    }

    #[test]
    fn a_tagged_enum_is_total_where_its_tags_and_fields_fill_its_repr() {
        let (flags, every) = fields::<bool, Total, 7>(ReprRange::FULL);
        let both = EnumLayout::tagged(ReprRange::<u8>::new(0, 1), &[flags, flags]);
        let both = EnumValidity::new(both).with_variant(0_u8, flags, every);
        assert_eq!(both.with_variant(1_u8, flags, every).code(8), TOTAL, "seven flags and a tag");
        let (fewer, six) = fields::<bool, Total, 6>(ReprRange::FULL);
        assert_eq!(both.with_variant(1_u8, fewer, six).code(8), ZERO_VALID, "not one flag fewer");
        let tags = EnumLayout::tagged(ReprRange::<u8>::new(0, 2), &[flags, flags]);
        let tags = EnumValidity::new(tags).with_variant(0_u8, flags, every);
        let tags = tags.with_variant(1_u8, flags, every).with_variant(2_u8, flags, every);
        assert_eq!(tags.code(16), ZERO_VALID, "nor three tags of four");
        let ends = EnumLayout::tagged(ReprRange::<u16>::new(0, 255), &[]);
        let ends = EnumValidity::new(ends).with_unit(0_u16).with_unit(255_u16);
        assert_eq!(ends.code(8), ZERO_VALID, "nor two tags whose range fills a byte");
    }

    #[test]
    fn a_niche_filling_enum_promises_what_its_zero_repr_holds() {
        let (owner, owners) = fields::<u64, ZeroNiche, 1>(ReprRange::new(3, u128::from(u64::MAX)));
        let lock = EnumValidity::new(EnumLayout::niche_or_tagged(2, owner, 2_u8));
        let lock = lock.with_unit(0_u8).with_unit(1_u8).with_variant(2_u8, owner, owners);
        assert_eq!(lock.code(64), ZERO_NICHE, "zero is the payload's, which never decodes");
        let (id, ids) = fields::<u8, TotalZeroNiche, 1>(ReprRange::NONZERO);
        let maybe = EnumValidity::new(EnumLayout::niche_or_tagged(1, id, 1_u8));
        let maybe = maybe.with_unit(0_u8).with_variant(1_u8, id, ids);
        assert_eq!(maybe.code(8), ZERO_VALID, "zero is the unit's");
    }

    #[test]
    fn an_enum_of_pointers_promises_what_the_variant_at_tag_zero_does() {
        let no_tags = PackedLayout::new(&[]);
        let node_first = PointerEnumLayout::tagged(&[
            PointerEnumVariant::pointer::<_, NonNull<u64>>(0_u8, no_tags),
            PointerEnumVariant::unit(1_u8),
        ]);
        let null_node =
            PackedValidity::EMPTY.with_field::<TotalZeroNiche>(node_first.pointer_layout());
        let zero_is_no_node = PointerEnumValidity::new(node_first).with_variant(0_u8, null_node);
        assert_eq!(zero_is_no_node.code(), ZERO_NICHE, "a null node at tag 0 decodes as nothing");
        assert_eq!(zero_is_no_node.range::<*mut ()>(), ReprRange::NONZERO, "so zero lies outside");
        let no_tag_zero = PointerEnumLayout::tagged(&[
            PointerEnumVariant::unit(1_u8),
            PointerEnumVariant::pointer::<_, *mut u64>(2_u8, no_tags),
        ]);
        let no_variant = PointerEnumValidity::new(no_tag_zero);
        assert_eq!(no_variant.code(), ZERO_NICHE, "nor where no variant's tag is 0");
        let digit = PackedField::new(ReprRange::<u8>::new(0, 9), 0).layout();
        let data_first = PointerEnumLayout::tagged(&[
            PointerEnumVariant::data(0_u8),
            PointerEnumVariant::pointer::<_, *mut u64>(1_u8, no_tags),
        ]);
        let unsure = PointerEnumValidity::new(data_first)
            .with_variant(0_u8, PackedValidity::EMPTY.with_field::<Partial>(digit));
        assert_eq!(unsure.code(), PARTIAL, "and a value at tag 0 that promises nothing, nothing");
        assert_eq!(unsure.range::<*mut ()>(), ReprRange::FULL, "every repr");
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
        assert_stated_width::<Wide, u128>(128);
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
