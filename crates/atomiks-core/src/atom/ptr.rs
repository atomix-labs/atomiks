//! `Atom` for pointers, stored in an atomic pointer so they keep their provenance, and
//! [`PtrAtom`], every value stored as one.

#![expect(unsafe_code, reason = "each `Atom` impl here promises what loads rely on")]

use core::ptr::NonNull;

use super::Atom;
use crate::range::{PointeeAlignment, ReprRange};
use crate::validity::{Total, TotalZeroNiche};

// SAFETY: a pointer is its own repr and every repr a pointer, so its range is every repr, and its
// address may cross threads, as `AtomicPtr`'s does.
const unsafe impl<T> Atom for *mut T {
    type Repr = Self;
    type Validity = Total;
    const REPRS: ReprRange<Self> = ReprRange::FULL;
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::of::<T>();
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
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::of::<T>();
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
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::of::<T>();
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

/// A value stored as a pointer to its [`Pointee`](Self::Pointee): a `NonNull`, an `Option` of one,
/// a raw pointer, or any value whose [`Atom::Repr`] is a `*mut`, such as a newtype of one, a
/// pointer word or a pointer enum.
///
/// A pointer word, `#[derive(Atom)]`'s struct of one such field beside tag fields, packs the tags
/// into the low bits the pointer's alignment leaves clear, and its derive bounds a generic pointer
/// field by this trait.
///
/// One impl gives it to every such [`Atom`], so it promises nothing of its own: it names the
/// pointee, so generic code knows the repr is `*mut Self::Pointee`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not stored as a pointer, so no tag can share its bits",
    label = "expected a value whose repr is a `*mut`",
    note = "a pointer word's pointer field, or a pointer enum variant's, is a `NonNull`, an `Option` of one, a raw pointer, or a value stored as one, such as a newtype of one"
)]
pub const trait PtrAtom: [const] Atom<Repr = *mut Self::Pointee> {
    /// The type the pointer points to.
    type Pointee;
}

// `do_not_recommend`, so a value whose repr is no pointer reports `PtrAtom`'s message.
#[diagnostic::do_not_recommend]
const impl<T: [const] Atom<Repr = *mut P>, P> PtrAtom for T {
    type Pointee = P;
}
