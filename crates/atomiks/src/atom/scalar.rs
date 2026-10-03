//! `Atom` for core's value types.

use core::marker::PhantomData;
use core::num::{NonZero, Saturating, Wrapping};

use super::{Atom, AtomAdd, AtomBitwise, AtomOrd};
use crate::validity::{Total, TotalButZero, ZeroValid};

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

/// Implements `Atom` and `AtomOrd` for the nonzero integers.
macro_rules! nonzero {
    ($($int:ty),+ $(,)?) => {$(
        // SAFETY: the repr is the integer, never zero, and its unsigned bits span `1..=MAX_REPR`;
        // `new` decodes exactly the nonzero ones; a nonzero integer may cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl Atom for NonZero<$int> {
            type Repr = $int;
            type Validity = TotalButZero;
            const MIN_REPR: u128 = 1;
            const MAX_REPR: u128 = <$int as Atom>::MAX_REPR;
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

// SAFETY: the repr is the scalar value, within `0..=char::MAX`; `from_u32` decodes exactly the
// scalar values, refusing the surrogates; a `char` may cross threads.
#[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
const unsafe impl Atom for char {
    type Repr = u32;
    type Validity = ZeroValid;
    const MIN_REPR: u128 = 0;
    const MAX_REPR: u128 = 0x10_FFFF;
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
            const MIN_REPR: u128 = 0;
            const MAX_REPR: u128 = <$bits as Atom>::MAX_REPR;
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

/// Implements `Atom` for zero-width values: one value, whose repr is zero.
macro_rules! zero_width {
    ($([$($param:tt)*] $kind:ty => $value:expr);+ $(;)?) => {$(
        // SAFETY: the one value is repr zero, and only zero decodes; it holds no data, so it may
        // cross threads.
        #[expect(unsafe_code, reason = "an `Atom` impl promises what loads rely on")]
        const unsafe impl<$($param)*> Atom for $kind {
            type Repr = u8;
            type Validity = ZeroValid;
            const MIN_REPR: u128 = 0;
            const MAX_REPR: u128 = 0;
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
            const MIN_REPR: u128 = T::MIN_REPR;
            const MAX_REPR: u128 = T::MAX_REPR;
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
