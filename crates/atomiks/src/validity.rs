//! Which reprs decode, as a type.
//!
//! A trait bound on it, rather than a promise at each call, decides where a byte-level read, or a
//! read-modify-write that can leave any repr, is sound.
//!
//! | Validity         | Zero decodes | `Option` of it                                     |
//! | ---------------- | ------------ | -------------------------------------------------- |
//! | [`Total`]        | yes          | [`Partial`]; refused while every repr is a value   |
//! | [`TotalButZero`] | no           | [`Total`]: `None` takes repr 0                     |
//! | [`ZeroValid`]    | yes          | [`ZeroValid`]                                      |
//! | [`ZeroNiche`]    | no           | [`ZeroValid`]: `None` takes repr 0                 |
//! | [`Partial`]      | not promised | [`Partial`]                                        |

use core::marker::Destruct;
use core::panic::RefUnwindSafe;

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

/// Every repr but zero decodes, and `MIN_REPR` is 1, so `Option`'s `None` takes repr 0 and every
/// repr of the `Option` decodes.
#[derive(Debug)]
pub enum TotalButZero {}

/// The zero repr decodes.
#[derive(Debug)]
pub enum ZeroValid {}

/// Zero does not decode and `MIN_REPR` is 1, so `Option`'s `None` takes repr 0; other reprs need
/// not decode.
#[derive(Debug)]
pub enum ZeroNiche {}

/// No repr beyond those `to_repr` returns is promised to decode.
#[derive(Debug)]
pub enum Partial {}

/// A primitive's cell behind a type that implements no trait building a value from bytes;
/// `repr(transparent)`, so it has its cell's layout.
#[doc(hidden)]
#[repr(transparent)]
#[derive(Debug)]
pub struct Opaque<C>(C);

const impl Validity for Total {
    type Cell<R: CellAccess> = R::Cell;
    // A canonical `Total` value leaves no repr for `None`; only a non-canonical impl reaches here.
    type Optional = Partial;
    const NONE_TAKES_ZERO: bool = false;
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

/// Implements `Validity` for each kind whose cell is opaque.
macro_rules! opaque {
    ($($kind:ident => $optional:ident, $none_takes_zero:literal);+ $(;)?) => {$(
        const impl Validity for $kind {
            type Cell<R: CellAccess> = Opaque<R::Cell>;
            type Optional = $optional;
            const NONE_TAKES_ZERO: bool = $none_takes_zero;
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

opaque! {
    TotalButZero => Total, true;
    ZeroValid => ZeroValid, false;
    ZeroNiche => ZeroValid, true;
    Partial => Partial, false;
}

#[cfg(test)]
mod tests {
    use super::{Partial, Total, TotalButZero, Validity, ZeroNiche, ZeroValid};

    /// Compiles only where `Option` of a `V` value has validity `O`.
    const fn optional_is<V: Validity<Optional = O>, O: Validity>() {}

    #[test]
    fn none_fills_a_zero_niche_and_keeps_a_valid_zero() {
        optional_is::<Total, Partial>();
        optional_is::<TotalButZero, Total>();
        optional_is::<ZeroValid, ZeroValid>();
        optional_is::<ZeroNiche, ZeroValid>();
        optional_is::<Partial, Partial>();
    }

    #[test]
    fn none_takes_zero_where_zero_is_the_niche() {
        const {
            assert!(
                TotalButZero::NONE_TAKES_ZERO,
                "TotalButZero: zero is no value, so `None` takes it"
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
            assert!(!<TotalButZero as Validity>::Optional::NONE_TAKES_ZERO, "Option<TotalButZero>");
            assert!(!<ZeroValid as Validity>::Optional::NONE_TAKES_ZERO, "Option<ZeroValid>");
            assert!(!<ZeroNiche as Validity>::Optional::NONE_TAKES_ZERO, "Option<ZeroNiche>");
            assert!(!<Partial as Validity>::Optional::NONE_TAKES_ZERO, "Option<Partial>");
        }
    }
}
