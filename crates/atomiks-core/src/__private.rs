//! What the code `#[derive(Atom)]` writes calls: hidden, since nothing else should call it.

pub use crate::range::{FieldLayout, NicheLayout};
use crate::validity::{Partial, Total, TotalZeroNiche, Validity, ZeroNiche, ZeroValid};

/// A validity named by a number, which a constant computes and [`SelectValidity`] maps to the
/// validity's type.
#[derive(Debug)]
pub struct ValidityCode<const CODE: u8>;

/// The validity a [`ValidityCode`] names.
pub impl(crate) trait SelectValidity {
    /// The validity.
    type Validity: const Validity;
}

/// Names each validity by a number, and implements `SelectValidity` for it.
macro_rules! validity_codes {
    ($($name:ident = $number:literal => $validity:ident;)+) => {$(
        #[doc = concat!("The number of [`", stringify!($validity), "`].")]
        pub const $name: u8 = $number;

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
    use super::{
        FieldLayout, PARTIAL, PackedValidity, SelectValidity, TOTAL, TOTAL_ZERO_NICHE,
        ValidityCode, ZERO_NICHE, ZERO_VALID,
    };
    use crate::range::ReprRange;
    use crate::validity::{Partial, Total, TotalZeroNiche, Validity, ZeroNiche, ZeroValid};

    /// How a field of eight bits is stored.
    const BYTE: FieldLayout = ReprRange::<u8>::FULL.field_layout();
    /// How a field of no bits, whose one repr is zero, is stored.
    const NO_BITS: FieldLayout = ReprRange::<u8>::new(0, 0).field_layout();

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
}
