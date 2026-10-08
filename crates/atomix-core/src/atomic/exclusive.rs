//! Reading and writing the value through `&mut`, where no other thread can see the cell.

#[cfg(not(loom))]
use core::ptr;

use super::Atomic;
use crate::atom::Atom;
use crate::primitive::{CellAccess, RawAccess};
use crate::validity::{Total, Validity};

impl<T: Atom> Atomic<T> {
    /// The value, read without an atomic operation.
    #[cfg(not(loom))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[must_use]
    pub const fn get(&mut self) -> T
    where
        T: [const] Atom,
    {
        let repr = T::Repr::get(T::Validity::get_mut::<T::Repr>(&mut self.cell));
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(repr) }
    }

    /// The value, read without an atomic operation.
    #[cfg(loom)]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[must_use]
    pub fn get(&mut self) -> T {
        let repr = T::Repr::get(T::Validity::get_mut::<T::Repr>(&mut self.cell));
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(repr) }
    }

    /// Writes `value` without an atomic operation.
    #[cfg(not(loom))]
    #[inline]
    pub const fn set(&mut self, value: T)
    where
        T: [const] Atom,
    {
        T::Repr::set(T::Validity::get_mut::<T::Repr>(&mut self.cell), value.to_repr());
    }

    /// Writes `value` without an atomic operation.
    #[cfg(loom)]
    #[inline]
    pub fn set(&mut self, value: T) {
        T::Repr::set(T::Validity::get_mut::<T::Repr>(&mut self.cell), value.to_repr());
    }

    /// Lends the value to `f` and writes back what `f` leaves, unless `f` panics.
    #[inline]
    pub fn with_mut<R, U: FnOnce(&mut T) -> R>(&mut self, f: U) -> R {
        let mut value = self.get();
        let result = f(&mut value);
        self.set(value);
        result
    }
}

/// A value whose place an atomic's can be: a primitive that is its own repr, every one of which
/// decodes, in a cell that lends its place.
///
/// `get_mut`, `from_mut` and the slice conversions take it, and its message names the values that
/// have it.
#[diagnostic::on_unimplemented(
    message = "an atomic of `{Self}` shares no place with a `{Self}`",
    label = "expected `bool`, an integer or a `*mut` pointer to a sized value",
    note = "an atomic's place is its value's only where the value is its own repr, every one of which decodes",
    note = "an atomic of any other value reaches it through `&mut` with `get`, `set` and `with_mut`"
)]
pub impl(crate) const trait RawValue:
    Atom<Repr = Self, Validity = Total> + [const] RawAccess
{
}

// `do_not_recommend`, so a value without it reports this trait's message, not one of the impl's
// bounds.
#[diagnostic::do_not_recommend]
const impl<T: Atom<Repr = T, Validity = Total> + [const] RawAccess> RawValue for T {}

// Each bound sits on its method, so a value without it gets `RawValue`'s message.
#[cfg(not(loom))]
impl<T: Atom> Atomic<T> {
    /// The value's place, for a primitive (every repr of which is a value) whose cell holds it as
    /// its own type lays it out, as [`RawAccess`] says.
    ///
    /// [`with_mut`](Self::with_mut) lends any other value, and every value under loom.
    #[inline]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut T
    where
        T: const RawValue,
    {
        <T as RawAccess>::get_mut(Total::get_mut::<T>(&mut self.cell))
    }

    /// The atomic over `value`'s place, for as long as the borrow, for the primitives
    /// [`get_mut`](Self::get_mut) lends.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use std::thread;
    ///
    /// use atomix::AtomicU64;
    /// use atomix::ordering::Relaxed;
    ///
    /// // Fills counted by two threads, then read as a plain `u64`.
    /// let mut fills = 0_u64;
    /// let counter: &AtomicU64 = AtomicU64::from_mut(&mut fills);
    /// thread::scope(|scope| {
    ///     for thread_fills in [3, 4] {
    ///         scope.spawn(move || counter.fetch_add(thread_fills, Relaxed));
    ///     }
    /// });
    /// assert_eq!(fills, 7, "the plain `u64` holds what each thread added");
    /// ```
    #[expect(unsafe_code, reason = "a reference to a primitive as the atomic over its place")]
    #[inline]
    #[must_use]
    pub const fn from_mut(value: &mut T) -> &mut Self
    where
        T: const RawValue,
    {
        const { assert_same_layout::<T, Self>() };
        // SAFETY: `Atomic<T>` is `repr(transparent)` over `Total`'s cell, `T`'s own, which holds a
        // `T` as `T` lays it out, by `RawAccess`'s contract, and is aligned as `T` is, as the
        // assertion above checks; so `value`'s place is an atomic's. The borrow is exclusive for
        // its lifetime, as the atomic's is. Every `T` is a repr (`Repr = T`) that decodes
        // (`Total`), so the field INVARIANT holds of what `value` holds, and what the
        // atomic leaves is a `T`.
        unsafe { &mut *ptr::from_mut(value).cast::<Self>() }
    }

    /// The values of `atomics`, each its atomic's place, for as long as the borrow.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::ordering::Relaxed;
    /// use atomix::{Atomic, AtomicU32};
    ///
    /// // Per-venue order counts, reset between sessions while no thread shares them.
    /// let mut counts = [const { AtomicU32::new(5) }; 3];
    /// Atomic::get_mut_slice(&mut counts).fill(0);
    /// assert!(counts.iter().all(|count| count.load(Relaxed) == 0), "every count reset");
    /// ```
    #[expect(unsafe_code, reason = "a slice of atomics as the primitives they hold")]
    #[inline]
    #[must_use]
    pub const fn get_mut_slice(atomics: &mut [Self]) -> &mut [T]
    where
        T: const RawValue,
    {
        const { assert_same_layout::<T, Self>() };
        let length = atomics.len();
        // SAFETY: each atomic's cell holds a `T` as `T` lays it out, by `RawAccess`'s contract,
        // and the assertion above gives an atomic a `T`'s size and alignment, so the pointer,
        // aligned for `T`, reaches `length` `T`s, each its atomic's whole cell; the borrow is
        // exclusive for its lifetime, and every `T` written is a repr that decodes.
        unsafe { &mut *ptr::slice_from_raw_parts_mut(atomics.as_mut_ptr().cast::<T>(), length) }
    }

    /// The atomics over the places of `values`, for as long as the borrow.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use std::thread;
    ///
    /// use atomix::Atomic;
    /// use atomix::ordering::Relaxed;
    ///
    /// // Per-venue order counts, a plain array between sessions, counted by threads during one.
    /// let mut counts = [0_u32; 3];
    /// let atomics: &[Atomic<u32>] = Atomic::from_mut_slice(&mut counts);
    /// thread::scope(|scope| {
    ///     for venue in [0, 2, 2] {
    ///         scope.spawn(move || atomics[venue].fetch_add(1, Relaxed));
    ///     }
    /// });
    /// assert_eq!(counts, [1, 0, 2], "each count, in its place");
    /// ```
    #[expect(unsafe_code, reason = "a slice of primitives as the atomics over their places")]
    #[inline]
    #[must_use]
    pub const fn from_mut_slice(values: &mut [T]) -> &mut [Self]
    where
        T: const RawValue,
    {
        const { assert_same_layout::<T, Self>() };
        let length = values.len();
        // SAFETY: as in `from_mut`, for each element; the assertion above makes the `length`
        // atomics span exactly the `length` values the pointer reaches.
        unsafe { &mut *ptr::slice_from_raw_parts_mut(values.as_mut_ptr().cast::<Self>(), length) }
    }
}

/// Refuses the build unless an `A` has a `T`'s size and alignment: the alignment `RawAccess` does
/// not promise, which an atomic and its value share on every target atomix builds for, `u128`'s 16
/// bytes included.
#[cfg(not(loom))]
const fn assert_same_layout<T, A>() {
    assert!(
        size_of::<T>() == size_of::<A>() && align_of::<T>() == align_of::<A>(),
        "an atomic is laid out as its value"
    );
}

// Under loom, `new` is not `const`.
#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use crate::atomic::AtomicU64;

    #[test]
    fn get_and_set_are_const() {
        const {
            let mut value = AtomicU64::new(1);
            value.set(2);
            assert!(value.get() == 2, "get reads what set wrote, in const");
        }
    }
}
