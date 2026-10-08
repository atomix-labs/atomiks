//! What a test of field operations checks of a packed value: where a path's bits lie, that the
//! container's repr stayed one that decodes, and a signed field to change.

use core::fmt::Debug;

use atomix_core::ordering::Relaxed;
use atomix_core::{Atom, Atomic, FieldPath, Load, RawAccess};

/// The offset and width of the field at the path `P`.
pub(crate) const fn bits<P: FieldPath>() -> (u32, u32) {
    (P::OFFSET, P::WIDTH)
}

/// The value `atomic` holds, read as its whole repr, which must decode: the operations kept it
/// canonical.
pub(crate) fn canonical<T: Atom + Debug + PartialEq>(atomic: &Atomic<T>) -> T
where
    T::Repr: Atom<Repr = T::Repr> + Load + RawAccess,
{
    // SAFETY: `as_ptr` points to the atomic's repr, aligned and live for the borrow; every access
    // to it is atomic and of its width, as this load is; and every repr of an integer, a `bool` or
    // a pointer decodes, as itself.
    #[expect(unsafe_code, reason = "reads the whole repr as itself, to check it decodes")]
    let whole = unsafe { Atomic::<T::Repr>::from_ptr(atomic.as_ptr()) };
    let repr = whole.load(Relaxed);
    let decoded = T::from_repr(repr).expect("the container's repr decodes: canonical");
    assert_eq!(decoded, atomic.load(Relaxed), "and is the value a load reads");
    decoded
}

/// Declares `Nibble`, a four-bit two's complement number that decodes from every `i8`, as its low
/// four bits: `Total`, and its bitwise operations combine values, but signed, as no built-in
/// `Total` value is. Its `const` impl needs `const_trait_impl` where it expands.
macro_rules! nibble {
    () => {
        /// A four-bit two's complement number, -8 to 7.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        struct Nibble(i8);

        // SAFETY: a `Nibble` is built only of -8 to 7, so `to_repr` lies in `REPRS`; `from_repr`
        // decodes every repr, by its low four bits alone, as the value `to_repr` came from;
        // `Total` is truthful; and an `i8` may cross threads.
        #[expect(unsafe_code, reason = "a signed field, which no built-in `Total` value is")]
        const unsafe impl ::atomix_core::Atom for Nibble {
            type Repr = i8;
            type Validity = ::atomix_core::validity::Total;
            const REPRS: ::atomix_core::ReprRange<i8> =
                ::atomix_core::ReprRange::from_signed(-8, 7);
            fn to_repr(self) -> i8 {
                self.0
            }
            fn from_repr(repr: i8) -> Option<Self> {
                Some(Self(repr.wrapping_shl(4).wrapping_shr(4)))
            }
        }

        impl ::atomix_core::AtomBitwise for Nibble {}
    };
}

pub(crate) use nibble;
