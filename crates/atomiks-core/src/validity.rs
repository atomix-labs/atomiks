//! Which reprs decode, as a type.
//!
//! A trait bound on it, rather than a promise at each call, decides where a byte-level read, or a
//! read-modify-write that can leave any repr, is sound.
//!
//! | Validity           | Zero decodes | `Option` of it                                   |
//! | ------------------ | ------------ | ------------------------------------------------ |
//! | [`Total`]          | yes          | [`Partial`]; refused while every repr is a value |
//! | [`TotalZeroNiche`] | no           | [`Total`]: `None` takes repr 0                   |
//! | [`ZeroValid`]      | yes          | [`ZeroValid`]                                    |
//! | [`ZeroNiche`]      | no           | [`ZeroValid`]: `None` takes repr 0               |
//! | [`Partial`]        | not promised | [`Partial`]                                      |

use core::marker::Destruct;
use core::panic::RefUnwindSafe;

use self::opaque::Opaque;
use crate::primitive::CellAccess;

/// Which reprs of a value's primitive decode.
///
/// Only atomiks implements it.
pub impl(crate) const trait Validity {
    /// The cell an `Atomic` of such a value holds: the primitive's own for [`Total`], an opaque
    /// wrapper otherwise, so a trait that builds a value from bytes reaches only total values.
    ///
    /// Has `R::Cell`'s layout, which `Atomic::from_ptr` relies on. Its bounds repeat
    /// `CellAccess::Cell`'s, so generic code sees them; change both together.
    #[doc(hidden)]
    type Cell<R: CellAccess>: Send + Sync + RefUnwindSafe + Unpin + const Destruct;
    /// The validity of `Option` of such a value.
    #[doc(hidden)]
    type Optional: const Validity;
    /// Whether an `Option`'s `None` must take the zero repr, which no value takes.
    #[doc(hidden)]
    const NONE_TAKES_ZERO: bool;
    /// Whether this validity promises that every repr decodes: [`Total`] alone.
    #[doc(hidden)]
    const PROMISES_EVERY_REPR_DECODES: bool;
    /// Whether this validity promises that the zero repr decodes: [`Total`] and [`ZeroValid`].
    #[doc(hidden)]
    const PROMISES_ZERO_DECODES: bool;
    /// What this validity promises of the zero repr alone: [`ZeroValid`] where it decodes, else
    /// [`Partial`].
    #[doc(hidden)]
    type ZeroValidity: const Validity;
    /// What a value of a field with this validity, beside fields whose zero validity is `V`'s,
    /// promises of zero: `V`'s zero validity where this one's zero decodes, else [`Partial`]. So
    /// a generic derived value's validity nests it over its fields.
    #[doc(hidden)]
    type ZeroValidityWith<V: Validity>: const Validity;
    /// Wraps a primitive's cell.
    #[doc(hidden)]
    fn wrap<R: CellAccess>(cell: R::Cell) -> Self::Cell<R>;
    /// The primitive's cell, consuming the wrapper.
    #[doc(hidden)]
    fn into_inner<R: CellAccess>(cell: Self::Cell<R>) -> R::Cell;
    /// Borrows the primitive's cell.
    #[doc(hidden)]
    fn get_ref<R: CellAccess>(cell: &Self::Cell<R>) -> &R::Cell;
    /// Borrows the primitive's cell exclusively.
    #[doc(hidden)]
    fn get_mut<R: CellAccess>(cell: &mut Self::Cell<R>) -> &mut R::Cell;
}

/// Every repr decodes.
#[derive(Debug)]
pub enum Total {}

/// Every repr but zero decodes: zero is the niche, so `Option`'s `None` takes repr 0 and every repr
/// of the `Option` decodes.
#[derive(Debug)]
pub enum TotalZeroNiche {}

/// The zero repr decodes.
#[derive(Debug)]
pub enum ZeroValid {}

/// Zero does not decode, so `Option`'s `None` takes repr 0, wherever the value's range lies; other
/// reprs need not decode.
#[derive(Debug)]
pub enum ZeroNiche {}

/// No repr beyond those `to_repr` returns is promised to decode.
#[derive(Debug)]
pub enum Partial {}

mod opaque {
    //! [`Opaque`], in a module of its own so that no path outside `validity` names it.

    /// A primitive's cell behind a type that implements no trait building a value from bytes;
    /// `repr(transparent)`, so it has its cell's layout.
    #[repr(transparent)]
    #[derive(Debug)]
    pub struct Opaque<C>(pub(super) C);
}

const impl Validity for Total {
    type Cell<R: CellAccess> = R::Cell;
    // A canonical `Total` value leaves no repr for `None`; only a non-canonical impl reaches here.
    type Optional = Partial;
    const NONE_TAKES_ZERO: bool = false;
    const PROMISES_EVERY_REPR_DECODES: bool = true;
    const PROMISES_ZERO_DECODES: bool = true;
    type ZeroValidity = ZeroValid;
    type ZeroValidityWith<V: Validity> = V::ZeroValidity;
    #[inline]
    fn wrap<R: CellAccess>(cell: R::Cell) -> R::Cell {
        cell
    }
    #[inline]
    fn into_inner<R: CellAccess>(cell: R::Cell) -> R::Cell {
        cell
    }
    #[inline]
    fn get_ref<R: CellAccess>(cell: &R::Cell) -> &R::Cell {
        cell
    }
    #[inline]
    fn get_mut<R: CellAccess>(cell: &mut R::Cell) -> &mut R::Cell {
        cell
    }
}

/// Implements `Validity` for each kind whose cell is opaque, which some repr does not decode.
macro_rules! opaque {
    ($(
        $kind:ident => $optional:ident, $none_takes_zero:literal, $zero_decodes:literal,
        $zero_validity:ident, $zero_validity_with:ty
    );+ $(;)?) => {$(
        const impl Validity for $kind {
            type Cell<R: CellAccess> = Opaque<R::Cell>;
            type Optional = $optional;
            const NONE_TAKES_ZERO: bool = $none_takes_zero;
            const PROMISES_EVERY_REPR_DECODES: bool = false;
            const PROMISES_ZERO_DECODES: bool = $zero_decodes;
            type ZeroValidity = $zero_validity;
            type ZeroValidityWith<V: Validity> = $zero_validity_with;
            #[inline]
            fn wrap<R: CellAccess>(cell: R::Cell) -> Opaque<R::Cell> {
                Opaque(cell)
            }
            #[inline]
            fn into_inner<R: CellAccess>(cell: Opaque<R::Cell>) -> R::Cell {
                cell.0
            }
            #[inline]
            fn get_ref<R: CellAccess>(cell: &Opaque<R::Cell>) -> &R::Cell {
                &cell.0
            }
            #[inline]
            fn get_mut<R: CellAccess>(cell: &mut Opaque<R::Cell>) -> &mut R::Cell {
                &mut cell.0
            }
        }
    )+};
}

// Each kind: its `Option`'s validity, whether `None` takes zero, whether zero decodes, its
// `ZeroValidity`, and its `ZeroValidityWith<V>`.
opaque! {
    TotalZeroNiche => Total, true, false, Partial, Partial;
    ZeroValid => ZeroValid, false, true, ZeroValid, V::ZeroValidity;
    ZeroNiche => ZeroValid, true, false, Partial, Partial;
    Partial => Partial, false, false, Partial, Partial;
}

#[cfg(test)]
mod tests {
    use super::{Partial, Total, TotalZeroNiche, Validity, ZeroNiche, ZeroValid};

    /// Compiles only where `Option` of a `V` value has validity `O`.
    const fn optional_is<V: Validity<Optional = O>, O: Validity>() {}

    #[test]
    fn none_fills_a_zero_niche_and_keeps_a_valid_zero() {
        optional_is::<Total, Partial>();
        optional_is::<TotalZeroNiche, Total>();
        optional_is::<ZeroValid, ZeroValid>();
        optional_is::<ZeroNiche, ZeroValid>();
        optional_is::<Partial, Partial>();
    }

    /// Compiles only where `V` promises of zero what `Z` does.
    const fn zero_validity_is<V: Validity<ZeroValidity = Z>, Z: Validity>() {}

    /// Compiles only where a field of validity `V` beside fields of `W` promises of zero what `Z`
    /// does.
    const fn zero_validity_with_is<
        V: Validity<ZeroValidityWith<W> = Z>,
        W: Validity,
        Z: Validity,
    >() {
    }

    #[test]
    fn zero_validity_keeps_only_whether_zero_decodes() {
        zero_validity_is::<Total, ZeroValid>();
        zero_validity_is::<TotalZeroNiche, Partial>();
        zero_validity_is::<ZeroValid, ZeroValid>();
        zero_validity_is::<ZeroNiche, Partial>();
        zero_validity_is::<Partial, Partial>();
    }

    /// Checks `zero_validity_with_is` for each row: a field's validity, then its zero validity
    /// beside fields of each kind, in the order of `Total`, `TotalZeroNiche`, `ZeroValid`,
    /// `ZeroNiche` and `Partial`.
    macro_rules! meet {
        ($($field:ident => $($zero:ident),+;)+) => {$(
            meet!(@row $field => [Total, TotalZeroNiche, ZeroValid, ZeroNiche, Partial] [$($zero),+]);
        )+};
        (@row $field:ident => [$($beside:ident),+] [$($zero:ident),+]) => {
            $(zero_validity_with_is::<$field, $beside, $zero>();)+
        };
    }

    #[test]
    fn a_zero_decodes_beside_others_only_where_each_decodes() {
        meet! {
            Total => ZeroValid, Partial, ZeroValid, Partial, Partial;
            TotalZeroNiche => Partial, Partial, Partial, Partial, Partial;
            ZeroValid => ZeroValid, Partial, ZeroValid, Partial, Partial;
            ZeroNiche => Partial, Partial, Partial, Partial, Partial;
            Partial => Partial, Partial, Partial, Partial, Partial;
        }
    }

    #[test]
    fn every_repr_decodes_for_total_alone_and_zero_for_total_and_zero_valid() {
        const {
            assert!(
                Total::PROMISES_EVERY_REPR_DECODES && Total::PROMISES_ZERO_DECODES,
                "Total: every repr"
            );
            assert!(
                !TotalZeroNiche::PROMISES_EVERY_REPR_DECODES
                    && !TotalZeroNiche::PROMISES_ZERO_DECODES,
                "TotalZeroNiche: every repr but zero"
            );
            assert!(
                !ZeroValid::PROMISES_EVERY_REPR_DECODES && ZeroValid::PROMISES_ZERO_DECODES,
                "ZeroValid: zero, and no other promised"
            );
            assert!(
                !ZeroNiche::PROMISES_EVERY_REPR_DECODES && !ZeroNiche::PROMISES_ZERO_DECODES,
                "ZeroNiche: not zero"
            );
            assert!(
                !Partial::PROMISES_EVERY_REPR_DECODES && !Partial::PROMISES_ZERO_DECODES,
                "Partial: nothing promised"
            );
        }
    }

    #[test]
    fn none_takes_zero_where_zero_is_the_niche() {
        const {
            assert!(
                TotalZeroNiche::NONE_TAKES_ZERO,
                "TotalZeroNiche: zero is no value, so `None` takes it"
            );
            assert!(ZeroNiche::NONE_TAKES_ZERO, "ZeroNiche: zero is no value, so `None` takes it");
            assert!(!Total::NONE_TAKES_ZERO, "Total: zero is a value");
            assert!(!ZeroValid::NONE_TAKES_ZERO, "ZeroValid: zero is a value");
            assert!(!Partial::NONE_TAKES_ZERO, "Partial: nothing is promised of zero");
        }
    }

    #[test]
    fn an_option_of_an_option_never_needs_none_to_take_zero() {
        const {
            assert!(!<Total as Validity>::Optional::NONE_TAKES_ZERO, "Option<Total>");
            assert!(
                !<TotalZeroNiche as Validity>::Optional::NONE_TAKES_ZERO,
                "Option<TotalZeroNiche>"
            );
            assert!(!<ZeroValid as Validity>::Optional::NONE_TAKES_ZERO, "Option<ZeroValid>");
            assert!(!<ZeroNiche as Validity>::Optional::NONE_TAKES_ZERO, "Option<ZeroNiche>");
            assert!(!<Partial as Validity>::Optional::NONE_TAKES_ZERO, "Option<Partial>");
        }
    }
}
