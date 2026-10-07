//! The `Atom` laws every value obeys, built-in or derived, and the bits they run on: a value's repr
//! lies in its range and decodes back to it; a repr that decodes re-encodes to itself, unchecked
//! too; each validity's promise holds; an ordered value's reprs order as it does, and an integer
//! held to a range decodes, as itself, exactly where its repr lies in it; `None` takes a spare
//! repr: zero where zero is the niche, else one outside its value's range; and a value of pointers
//! decodes to pointers that read what its own do.
//!
//! A repr's bits are an integer's own, or a pointer's address, which the laws read without
//! exposing its provenance: two pointer reprs compare by address, and a pointer decoded is read
//! through, which Miri refuses where it lost its provenance.

use core::any::type_name;
use core::fmt::Debug;

use atomiks_core::validity::{Partial, Total, TotalZeroNiche, ZeroNiche, ZeroValid};
use atomiks_core::{Atom, AtomOrd, ExactBits, Primitive};
use proptest::prelude::{Strategy, any, prop_oneof};
use proptest::sample::select;
use proptest::test_runner::TestCaseError;
use proptest::{prop_assert, prop_assert_eq};

/// Edge bits, each with its neighbours.
///
/// The edges are zero, each width's largest signed and unsigned values, and `extra`.
pub(crate) fn edges(extra: &[u128]) -> Vec<u128> {
    let tops =
        [8_u32, 16, 32, 64, 128].map(|width| u128::MAX.unbounded_shr(128_u32.wrapping_sub(width)));
    let mut edges = vec![0];
    edges.extend(tops.into_iter().flat_map(|top| [top.unbounded_shr(1), top]));
    edges.extend_from_slice(extra);
    edges.into_iter().flat_map(|edge| [edge.wrapping_sub(1), edge, edge.wrapping_add(1)]).collect()
}

/// Bits, half from `edges` and half anywhere; each primitive takes their low bits.
pub(crate) fn edge_or_random_bits(extra: &[u128]) -> impl Strategy<Value = u128> {
    prop_oneof![select(edges(extra)), any::<u128>()]
}

/// The range and round-trip laws for `value`, and the repr laws for its repr.
pub(crate) fn round_trips<T: Atom + PartialEq + Debug>(value: T) -> Result<(), TestCaseError>
where
    T::Repr: PartialEq + Debug,
{
    let bits = value.to_repr().packed_bits();
    prop_assert!(T::REPRS.contains(bits), "{value:?}'s repr {bits:#x} lies in {:?}", T::REPRS);
    prop_assert_eq!(T::from_repr(value.to_repr()), Some(value), "the value decodes back");
    canonical::<T>(value.to_repr())
}

/// The repr laws for `repr`: if it decodes, the value it decodes to, checked and unchecked,
/// re-encodes to it.
pub(crate) fn canonical<T: Atom + Debug>(repr: T::Repr) -> Result<(), TestCaseError>
where
    T::Repr: PartialEq + Debug,
{
    if let Some(value) = T::from_repr(repr) {
        prop_assert_eq!(value.to_repr(), repr, "{:?} re-encodes to the repr it came from", value);
        // SAFETY: `from_repr` decodes `repr`.
        #[expect(unsafe_code, reason = "the unchecked decode the `Atom` contract constrains")]
        let unchecked = unsafe { T::from_repr_unchecked(repr) };
        prop_assert_eq!(unchecked.to_repr(), repr, "{:?} decodes unchecked as checked", repr);
    }
    Ok(())
}

/// What a validity promises of whether a repr decodes, as its doc in `atomiks_core::validity`
/// says.
pub(crate) trait Promise {
    /// Whether the zero repr decodes: `None` where the validity promises nothing.
    const ZERO_DECODES: Option<bool>;
    /// Whether every other repr decodes: `None` where the validity promises nothing.
    const NONZERO_DECODES: Option<bool>;
}

/// Implements `Promise` for each validity: what it promises of zero, then of every other repr.
macro_rules! promise {
    ($($validity:ident => $zero:expr, $nonzero:expr;)+) => {$(
        impl Promise for $validity {
            const ZERO_DECODES: Option<bool> = $zero;
            const NONZERO_DECODES: Option<bool> = $nonzero;
        }
    )+};
}

promise! {
    Total => Some(true), Some(true);
    TotalZeroNiche => Some(false), Some(true);
    ZeroValid => Some(true), None;
    ZeroNiche => Some(false), None;
    Partial => None, None;
}

/// The validity law for `repr`: it decodes as `V`, `T`'s validity, promises; then the repr laws.
///
/// Naming `V` checks that `T`'s validity is `V`; `_` takes whichever it is.
pub(crate) fn decodes_as_promised<V: Promise, T: Atom<Validity = V> + Debug>(
    repr: T::Repr,
) -> Result<(), TestCaseError>
where
    T::Repr: PartialEq + Debug,
{
    let promise = if repr.is_bits(0) { V::ZERO_DECODES } else { V::NONZERO_DECODES };
    if let Some(decodes) = promise {
        prop_assert_eq!(
            T::from_repr(repr).is_some(),
            decodes,
            "{:?} decodes as `{}`'s validity promises",
            repr,
            type_name::<T>()
        );
    }
    canonical::<T>(repr)
}

/// The order law for `a` and `b`: the reprs' own order is the values'.
pub(crate) fn reprs_order_as_ord<T: AtomOrd + Debug>(a: T, b: T) -> Result<(), TestCaseError>
where
    T::Repr: Ord,
{
    let (values, reprs) = (a.cmp(&b), a.to_repr().cmp(&b.to_repr()));
    prop_assert_eq!(reprs, values, "{:?} and {:?} order as their reprs do", a, b);
    Ok(())
}

/// The ranged integer laws for the low bits of `bits` as the repr of `T`, an integer held to a
/// range.
///
/// The repr decodes, as the integer it is, exactly where it lies in `T`'s range, alike unchecked,
/// as `V`, `T`'s validity, promises; and where the low bits of `other` decode too, the two
/// round-trip and order as their reprs do.
pub(crate) fn ranged_integer_laws<V: Promise, T>(
    bits: u128, other: u128,
) -> Result<(), TestCaseError>
where
    T: AtomOrd<Validity = V> + Into<T::Repr> + Debug,
    T::Repr: ExactBits + Ord + Debug,
{
    let [repr, other] = [bits, other].map(<T::Repr as Primitive>::from_bits);
    let decoded = T::from_repr(repr);
    prop_assert_eq!(
        decoded.map(Into::into),
        T::REPRS.contains(repr.to_bits()).then_some(repr),
        "{:?} decodes as itself exactly where it lies in {:?}",
        repr,
        T::REPRS
    );
    decodes_as_promised::<V, T>(repr)?;
    if let (Some(a), Some(b)) = (decoded, T::from_repr(other)) {
        round_trips(a)?;
        reprs_order_as_ord(a, b)?;
    }
    Ok(())
}

/// The `None` law for `T`, given `none`, the bits of `None`'s repr: they are spare, zero where
/// `T`'s validity makes zero its niche, wherever `T`'s range lies, and outside that range
/// otherwise; they decode as no `T`; and `Option<T>`'s range holds them.
pub(crate) fn none_takes_a_spare_repr<T: Atom>(none: u128) -> Result<(), TestCaseError>
where
    T::Validity: Promise,
{
    if <T::Validity as Promise>::ZERO_DECODES == Some(false) {
        prop_assert_eq!(none, 0, "`None` takes the zero niche");
    } else {
        prop_assert!(!T::REPRS.contains(none), "`None`'s {none:#x} lies outside {:?}", T::REPRS);
    }
    let repr = <T::Repr as Primitive>::from_bits(none);
    prop_assert!(T::from_repr(repr).is_none(), "and decodes as no value");
    let with_none = <Option<T> as Atom>::REPRS;
    prop_assert!(with_none.contains(none), "and lies in `Option`'s, {:?}", with_none);
    Ok(())
}

/// The bits of `None`'s repr as an `Option<T>`.
pub(crate) fn none_bits<T: Atom>() -> u128 {
    None::<T>.to_repr().packed_bits()
}

/// The provenance law for `value`, which holds pointers: what `read` reads through the pointers
/// of the value its repr decodes to, checked and unchecked, is what it reads through `value`'s.
pub(crate) fn keeps_provenance<T, R, F>(value: T, read: F) -> Result<(), TestCaseError>
where
    T: Atom + Debug,
    R: PartialEq + Debug,
    F: Fn(T) -> R,
{
    let repr = value.to_repr();
    let Some(decoded) = T::from_repr(repr) else {
        return Err(TestCaseError::fail(format!("{value:?} decodes")));
    };
    prop_assert_eq!(read(decoded), read(value), "read through {:?}, decoded", decoded);
    // SAFETY: `repr` is `value`'s own, which decodes.
    #[expect(unsafe_code, reason = "the unchecked decode the `Atom` contract constrains")]
    let unchecked = unsafe { T::from_repr_unchecked(repr) };
    prop_assert_eq!(read(unchecked), read(value), "and decoded unchecked");
    Ok(())
}

/// Panics with the broken law's message, where `laws` broke one.
#[track_caller]
pub(crate) fn assert_holds(laws: Result<(), TestCaseError>) {
    if let Err(broken) = laws {
        panic!("{broken}");
    }
}

/// Checks the `None` law for each type.
macro_rules! none_laws {
    ($($value:ty),+ $(,)?) => {$(
        $crate::testing::law::assert_holds($crate::testing::law::none_takes_a_spare_repr::<$value>(
            $crate::testing::law::none_bits::<$value>(),
        ));
    )+};
}

pub(crate) use none_laws;
