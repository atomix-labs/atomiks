//! Reading and writing the value through `&mut`, where no other thread can see the cell.

use super::Atomic;
use crate::atom::Atom;
use crate::primitive::CellAccess;
#[cfg(not(loom))]
use crate::primitive::Primitive;
#[cfg(not(loom))]
use crate::validity::Total;
use crate::validity::Validity;

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
        unsafe { Self::decode(repr) }
    }

    /// The value, read without an atomic operation.
    #[cfg(loom)]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[must_use]
    pub fn get(&mut self) -> T {
        let repr = T::Repr::get(T::Validity::get_mut::<T::Repr>(&mut self.cell));
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { Self::decode(repr) }
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

#[cfg(not(loom))]
impl<T: Atom<Repr = T, Validity = Total> + const Primitive> Atomic<T> {
    /// The value's place, for a primitive (every repr of which is a value).
    ///
    /// [`with_mut`](Self::with_mut) lends any other value, and every value under loom.
    #[inline]
    #[must_use]
    pub const fn get_mut(&mut self) -> &mut T {
        <T as CellAccess>::get_mut(Total::get_mut::<T>(&mut self.cell))
    }
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
