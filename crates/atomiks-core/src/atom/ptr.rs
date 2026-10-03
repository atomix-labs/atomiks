//! `Atom` for pointers, stored in an atomic pointer so they keep their provenance.

#![expect(
    unsafe_code,
    reason = "each item here is an `Atom` impl, which promises what loads rely on"
)]

use core::ptr::NonNull;

use super::Atom;
use crate::range::ReprRange;
use crate::validity::{Total, TotalZeroNiche};

// SAFETY: a pointer is its own repr and every repr a pointer, so its range is every repr, and its
// address may cross threads, as `AtomicPtr`'s does.
const unsafe impl<T> Atom for *mut T {
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

// SAFETY: the repr is the same pointer made mutable and every repr is one, so its range is every
// repr, and its address may cross threads, as `AtomicPtr`'s does.
const unsafe impl<T> Atom for *const T {
    type Repr = *mut T;
    type Validity = Total;
    const REPRS: ReprRange<*mut T> = ReprRange::FULL;
    #[inline]
    fn to_repr(self) -> *mut T {
        self.cast_mut()
    }
    #[inline]
    fn from_repr(repr: *mut T) -> Option<Self> {
        Some(repr.cast_const())
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: *mut T) -> Self {
        repr.cast_const()
    }
}

// SAFETY: the repr is the pointer, never null, so within every repr but zero; `new` decodes exactly
// the non-null ones; its address may cross threads, as `AtomicPtr`'s does.
const unsafe impl<T> Atom for NonNull<T> {
    type Repr = *mut T;
    type Validity = TotalZeroNiche;
    const REPRS: ReprRange<*mut T> = ReprRange::NONZERO;
    #[inline]
    fn to_repr(self) -> *mut T {
        self.as_ptr()
    }
    #[inline]
    fn from_repr(repr: *mut T) -> Option<Self> {
        Self::new(repr)
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: *mut T) -> Self {
        // SAFETY: the caller's repr decodes, so it is not null.
        unsafe { Self::new_unchecked(repr) }
    }
}
