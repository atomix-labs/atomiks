//! `Atom` for core's value types.

use super::{Atom, AtomAdd, AtomBitwise, AtomOrd};
use crate::validity::Total;

/// Implements `Atom` for integers, and the capabilities listed.
macro_rules! integers {
    (@one [$($capability:ident),*] $int:ty) => {
        // SAFETY: an integer is its own repr and every repr an integer, its unsigned bits span
        // `0..=MAX_REPR`, and an integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl Atom for $int {
            type Repr = Self;
            type Validity = Total;
            const MIN_REPR: u128 = 0;
            const MAX_REPR: u128 = u128::MAX.unbounded_shr(128_u32.wrapping_sub(<$int>::BITS));
            #[inline]
            fn to_repr(self) -> Self {
                self
            }
            #[inline]
            fn from_repr(repr: Self) -> Option<Self> {
                Some(repr)
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: Self) -> Self {
                repr
            }
        }
        $(impl $capability for $int {})*
    };
    ($capabilities:tt $($int:ty),+ $(,)?) => {$(
        integers!(@one $capabilities $int);
    )+};
}

integers! {
    [AtomAdd, AtomOrd, AtomBitwise]
    u8, u16, u32, u64, usize, i8, i16, i32, i64, isize,
}

// SAFETY: a `bool` is its own repr and every repr a `bool`, its bits span `0..=1`, and a `bool` may
// cross threads.
#[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
const unsafe impl Atom for bool {
    type Repr = Self;
    type Validity = Total;
    const MIN_REPR: u128 = 0;
    const MAX_REPR: u128 = 1;
    #[inline]
    fn to_repr(self) -> Self {
        self
    }
    #[inline]
    fn from_repr(repr: Self) -> Option<Self> {
        Some(repr)
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: Self) -> Self {
        repr
    }
}

impl AtomBitwise for bool {}

// The ranges and capabilities above, checked at compile time.
const _: () = {
    /// Compiles only for a value with every capability.
    const fn has_every_capability<T: AtomAdd + AtomOrd + AtomBitwise>() {}
    has_every_capability::<u8>();
    has_every_capability::<u16>();
    has_every_capability::<u32>();
    has_every_capability::<u64>();
    has_every_capability::<usize>();
    has_every_capability::<i8>();
    has_every_capability::<i16>();
    has_every_capability::<i32>();
    has_every_capability::<i64>();
    has_every_capability::<isize>();
    assert!(<u8 as Atom>::MAX_REPR == 0xFF, "u8's repr spans 8 bits");
    assert!(<i8 as Atom>::MAX_REPR == 0xFF, "i8's spans 8, as unsigned bits");
    assert!(<i16 as Atom>::MAX_REPR == 0xFFFF, "i16's spans 16");
    assert!(<i32 as Atom>::MAX_REPR == 0xFFFF_FFFF, "i32's spans 32");
    assert!(<u64 as Atom>::MAX_REPR == 0xFFFF_FFFF_FFFF_FFFF, "u64's spans 64");
    assert!(<isize as Atom>::MAX_REPR == <usize as Atom>::MAX_REPR, "isize's spans usize's");
    assert!(<bool as Atom>::MAX_REPR == 1, "bool's spans 1 bit");
};
