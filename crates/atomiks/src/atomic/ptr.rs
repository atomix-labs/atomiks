//! Pointer arithmetic on an atomic pointer, keeping the pointer's provenance.

use super::Atomic;
use crate::ordering::Ordering;
use crate::primitive::PtrOffset;

/// An atomic `*mut T`.
pub type AtomicPtr<T> = Atomic<*mut T>;

impl<P> Atomic<*mut P> {
    /// Offsets the pointer by `count` elements, as `wrapping_add`:
    /// [`fetch_ptr_add`](Self::fetch_ptr_add) with the pointer before discarded.
    #[inline]
    pub fn ptr_add<O: Ordering>(&self, count: usize, order: O) {
        let _ = order;
        <*mut P>::fetch_ptr_add(self.primitive_cell(), count, O::CORE);
    }

    /// Offsets the pointer back by `count` elements, as `wrapping_sub`:
    /// [`fetch_ptr_sub`](Self::fetch_ptr_sub) with the pointer before discarded.
    #[inline]
    pub fn ptr_sub<O: Ordering>(&self, count: usize, order: O) {
        let _ = order;
        <*mut P>::fetch_ptr_sub(self.primitive_cell(), count, O::CORE);
    }

    /// Offsets the pointer by `bytes`, as `wrapping_byte_add`:
    /// [`fetch_byte_add`](Self::fetch_byte_add) with the pointer before discarded.
    #[inline]
    pub fn byte_add<O: Ordering>(&self, bytes: usize, order: O) {
        let _ = order;
        <*mut P>::fetch_byte_add(self.primitive_cell(), bytes, O::CORE);
    }

    /// Offsets the pointer back by `bytes`, as `wrapping_byte_sub`:
    /// [`fetch_byte_sub`](Self::fetch_byte_sub) with the pointer before discarded.
    #[inline]
    pub fn byte_sub<O: Ordering>(&self, bytes: usize, order: O) {
        let _ = order;
        <*mut P>::fetch_byte_sub(self.primitive_cell(), bytes, O::CORE);
    }

    /// Offsets the pointer by `count` elements, as `wrapping_add`, and returns the pointer before.
    #[must_use = "to discard the pointer before, call `ptr_add`"]
    #[inline]
    pub fn fetch_ptr_add<O: Ordering>(&self, count: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_ptr_add(self.primitive_cell(), count, O::CORE)
    }

    /// Offsets the pointer back by `count` elements, as `wrapping_sub`,
    /// and returns the pointer before.
    #[must_use = "to discard the pointer before, call `ptr_sub`"]
    #[inline]
    pub fn fetch_ptr_sub<O: Ordering>(&self, count: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_ptr_sub(self.primitive_cell(), count, O::CORE)
    }

    /// Offsets the pointer by `bytes`, as `wrapping_byte_add`, and returns the pointer before.
    #[must_use = "to discard the pointer before, call `byte_add`"]
    #[inline]
    pub fn fetch_byte_add<O: Ordering>(&self, bytes: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_byte_add(self.primitive_cell(), bytes, O::CORE)
    }

    /// Offsets the pointer back by `bytes`, as `wrapping_byte_sub`, and returns the pointer before.
    #[must_use = "to discard the pointer before, call `byte_sub`"]
    #[inline]
    pub fn fetch_byte_sub<O: Ordering>(&self, bytes: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_byte_sub(self.primitive_cell(), bytes, O::CORE)
    }
}
