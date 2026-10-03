//! `Atom` for core's value types.

use core::marker::PhantomData;
use core::num::{NonZero, Saturating, Wrapping};

use super::{Atom, AtomAdd, AtomBitwise, AtomOrd};
use crate::range::ReprRange;
use crate::validity::{Total, TotalZeroNiche, ZeroValid};

/// Implements `Atom` for integers, and the capabilities listed.
macro_rules! integers {
    (@one [$($capability:ident),*] $int:ty) => {
        // SAFETY: an integer is its own repr and every repr an integer, so its range is every
        // repr, and an integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl Atom for $int {
            type Repr = Self;
            type Validity = Total;
            const REPRS: ReprRange<Self> = ReprRange::FULL;
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

// SAFETY: a `bool` is its own repr and every repr a `bool`, so its range is every repr, and a
// `bool` may cross threads.
#[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
const unsafe impl Atom for bool {
    type Repr = Self;
    type Validity = Total;
    const REPRS: ReprRange<Self> = ReprRange::FULL;
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

// The capabilities above, checked at compile time.
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
};

/// Implements `Atom` and `AtomOrd` for the nonzero integers.
macro_rules! nonzero {
    ($($int:ty),+ $(,)?) => {$(
        // SAFETY: the repr is the integer, never zero, so within every repr but zero; `new`
        // decodes exactly the nonzero ones; a nonzero integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl Atom for NonZero<$int> {
            type Repr = $int;
            type Validity = TotalZeroNiche;
            const REPRS: ReprRange<$int> = ReprRange::NONZERO;
            #[inline]
            fn to_repr(self) -> $int {
                self.get()
            }
            #[inline]
            fn from_repr(repr: $int) -> Option<Self> {
                Self::new(repr)
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: $int) -> Self {
                // SAFETY: the caller's repr decodes, so it is not zero.
                unsafe { Self::new_unchecked(repr) }
            }
        }
        impl AtomOrd for NonZero<$int> {}
    )+};
}

nonzero!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

// SAFETY: the repr is the scalar value, from 0 to `char::MAX`; `from_u32` decodes exactly the
// scalar values, refusing the surrogates; a `char` may cross threads.
#[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
const unsafe impl Atom for char {
    type Repr = u32;
    type Validity = ZeroValid;
    const REPRS: ReprRange<u32> = ReprRange::new(0, 0x10_FFFF);
    #[inline]
    fn to_repr(self) -> u32 {
        u32::from(self)
    }
    #[inline]
    fn from_repr(repr: u32) -> Option<Self> {
        Self::from_u32(repr)
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: u32) -> Self {
        // SAFETY: the caller's repr decodes, so it is a scalar value.
        unsafe { Self::from_u32_unchecked(repr) }
    }
}

impl AtomOrd for char {}

/// Implements `Atom` for floats, as their bits.
macro_rules! floats {
    ($($float:ty: $bits:ty),+ $(,)?) => {$(
        // SAFETY: the repr is the float's bits, which span the repr; every bit pattern is a
        // float; a float may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl Atom for $float {
            type Repr = $bits;
            type Validity = Total;
            const REPRS: ReprRange<$bits> = ReprRange::FULL;
            #[inline]
            fn to_repr(self) -> $bits {
                self.to_bits()
            }
            #[inline]
            fn from_repr(repr: $bits) -> Option<Self> {
                Some(<$float>::from_bits(repr))
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: $bits) -> Self {
                <$float>::from_bits(repr)
            }
        }
    )+};
}

floats!(f16: u16, f32: u32, f64: u64);
#[cfg(wide)]
floats!(f128: u128);

/// Implements `Atom` for zero-width values: one value, whose repr is zero.
macro_rules! zero_width {
    ($([$($param:tt)*] $kind:ty => $value:expr);+ $(;)?) => {$(
        // SAFETY: the one value is repr zero, and only zero decodes; it holds no data, so it may
        // cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<$($param)*> Atom for $kind {
            type Repr = u8;
            type Validity = ZeroValid;
            const REPRS: ReprRange<u8> = ReprRange::new(0, 0);
            #[inline]
            fn to_repr(self) -> u8 {
                0
            }
            #[inline]
            fn from_repr(repr: u8) -> Option<Self> {
                if repr == 0 { Some($value) } else { None }
            }
            #[inline]
            unsafe fn from_repr_unchecked(_: u8) -> Self {
                $value
            }
        }
    )+};
}

zero_width! {
    [] () => ();
    [T: ?Sized] PhantomData<T> => PhantomData;
}

/// Implements `Atom` for a wrapper stored as the value it wraps, and the capabilities it keeps.
macro_rules! wrappers {
    ($($wrapper:ident: [$($capability:ident),*]);+ $(;)?) => {$(
        // SAFETY: the repr, its range, the validity and the threads rule are the wrapped value's.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<T: [const] Atom> Atom for $wrapper<T> {
            type Repr = T::Repr;
            type Validity = T::Validity;
            const REPRS: ReprRange<T::Repr> = T::REPRS;
            #[inline]
            fn to_repr(self) -> T::Repr {
                self.0.to_repr()
            }
            #[inline]
            fn from_repr(repr: T::Repr) -> Option<Self> {
                match T::from_repr(repr) {
                    Some(value) => Some($wrapper(value)),
                    None => None,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(repr: T::Repr) -> Self {
                // SAFETY: the caller's repr decodes as the wrapper, so as the wrapped value.
                $wrapper(unsafe { T::from_repr_unchecked(repr) })
            }
        }
        $(impl<T: $capability> $capability for $wrapper<T> {})*
    )+};
}

// `Saturating` keeps no `AtomAdd`: the repr's add wraps, the value's saturates.
wrappers! {
    Wrapping: [AtomAdd, AtomOrd, AtomBitwise];
    Saturating: [AtomOrd, AtomBitwise];
}

// The 128-bit integers and their `NonZero`s, where a 16-byte compare-exchange exists. atomiks
// has no 128-bit add, bitwise operation, max or min, so they take no `AtomAdd` or `AtomBitwise`,
// and `AtomOrd` brings them no `max` or `min`: `update` is each one's loop.
#[cfg(wide)]
integers! { [AtomOrd] u128, i128 }
#[cfg(wide)]
nonzero!(u128, i128);
