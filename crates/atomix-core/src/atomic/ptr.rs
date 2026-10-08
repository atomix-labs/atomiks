//! Pointer arithmetic on an atomic pointer, keeping the pointer's provenance.

use super::Atomic;
use crate::ordering::RmwOrdering;
use crate::primitive::PtrOffset;

/// An atomic `*mut T`.
pub type AtomicPtr<T> = Atomic<*mut T>;

impl<P> Atomic<*mut P> {
    /// Offsets the pointer by `count` elements, as `wrapping_add`, and returns the pointer before.
    #[inline]
    pub fn fetch_ptr_add<O: RmwOrdering>(&self, count: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_ptr_add(self.primitive_cell(), count, O::CORE)
    }

    /// Offsets the pointer back by `count` elements, as `wrapping_sub`,
    /// and returns the pointer before.
    #[inline]
    pub fn fetch_ptr_sub<O: RmwOrdering>(&self, count: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_ptr_sub(self.primitive_cell(), count, O::CORE)
    }

    /// Offsets the pointer by `bytes`, as `wrapping_byte_add`, and returns the pointer before.
    #[inline]
    pub fn fetch_byte_add<O: RmwOrdering>(&self, bytes: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_byte_add(self.primitive_cell(), bytes, O::CORE)
    }

    /// Offsets the pointer back by `bytes`, as `wrapping_byte_sub`, and returns the pointer before.
    #[inline]
    pub fn fetch_byte_sub<O: RmwOrdering>(&self, bytes: usize, order: O) -> *mut P {
        let _ = order;
        <*mut P>::fetch_byte_sub(self.primitive_cell(), bytes, O::CORE)
    }
}
