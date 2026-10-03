//! The read-modify-writes that need a capability: add and subtract ([`AtomAdd`]), maximum and
//! minimum ([`AtomOrd`]), and the bitwise operations ([`AtomBitwise`]).
//!
//! Each bound sits on its method, so a value without the capability gets the capability's own
//! message. Every form of `max` and `min` also needs the target's [`MinMax`], and the `fetch_`
//! bitwise forms its [`FetchBitwise`]: `aarch64` has both, `x86_64` neither.

use super::Atomic;
use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd};
use crate::ordering::RmwOrdering;
use crate::primitive::{AddSub, Bitwise, FetchBitwise, MinMax};

impl<T: Atom> Atomic<T> {
    /// Adds `delta` to the repr, wrapping: [`fetch_add`](Self::fetch_add)
    /// with the value before discarded.
    #[inline]
    pub fn add<O: RmwOrdering>(&self, delta: T::Repr, order: O)
    where
        T: AtomAdd,
    {
        let _ = order;
        T::Repr::fetch_add(self.primitive_cell(), delta, O::CORE);
    }

    /// Subtracts `delta` from the repr, wrapping: [`fetch_sub`](Self::fetch_sub)
    /// with the value before discarded.
    #[inline]
    pub fn sub<O: RmwOrdering>(&self, delta: T::Repr, order: O)
    where
        T: AtomAdd,
    {
        let _ = order;
        T::Repr::fetch_sub(self.primitive_cell(), delta, O::CORE);
    }

    /// Adds `delta` to the repr, wrapping, and returns the value before.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `add`"]
    #[inline]
    pub fn fetch_add<O: RmwOrdering>(&self, delta: T::Repr, order: O) -> T
    where
        T: AtomAdd,
    {
        let _ = order;
        let before = T::Repr::fetch_add(self.primitive_cell(), delta, O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Subtracts `delta` from the repr, wrapping, and returns the value before.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `sub`"]
    #[inline]
    pub fn fetch_sub<O: RmwOrdering>(&self, delta: T::Repr, order: O) -> T
    where
        T: AtomAdd,
    {
        let _ = order;
        let before = T::Repr::fetch_sub(self.primitive_cell(), delta, O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Keeps the larger of `value` and the current value: [`fetch_max`](Self::fetch_max)
    /// with the value before discarded.
    #[doc(cfg(target_arch = "aarch64"))]
    #[inline]
    pub fn max<O: RmwOrdering>(&self, value: T, order: O)
    where
        T: AtomOrd,
        T::Repr: MinMax,
    {
        let _ = order;
        T::Repr::fetch_max(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Keeps the smaller of `value` and the current value: [`fetch_min`](Self::fetch_min)
    /// with the value before discarded.
    #[doc(cfg(target_arch = "aarch64"))]
    #[inline]
    pub fn min<O: RmwOrdering>(&self, value: T, order: O)
    where
        T: AtomOrd,
        T::Repr: MinMax,
    {
        let _ = order;
        T::Repr::fetch_min(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Keeps the larger of `value` and the current value, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `max`"]
    #[inline]
    pub fn fetch_max<O: RmwOrdering>(&self, value: T, order: O) -> T
    where
        T: AtomOrd,
        T::Repr: MinMax,
    {
        let _ = order;
        let before = T::Repr::fetch_max(self.primitive_cell(), value.to_repr(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Keeps the smaller of `value` and the current value, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `min`"]
    #[inline]
    pub fn fetch_min<O: RmwOrdering>(&self, value: T, order: O) -> T
    where
        T: AtomOrd,
        T::Repr: MinMax,
    {
        let _ = order;
        let before = T::Repr::fetch_min(self.primitive_cell(), value.to_repr(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Applies `& value`: [`fetch_and`](Self::fetch_and) with the value before discarded.
    ///
    /// Discarding it is what gives `and`, `or`, `xor` and `not` to every target: `x86_64` runs each
    /// as one `lock` instruction, which cannot return the value before. That needs an optimized
    /// build; unoptimized, `x86_64` compiles each as a compare-exchange loop.
    #[inline]
    pub fn and<O: RmwOrdering>(&self, value: T, order: O)
    where
        T: AtomBitwise,
    {
        let _ = order;
        T::Repr::fetch_and(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Applies `| value`: [`fetch_or`](Self::fetch_or) with the value before discarded, at
    /// [`and`](Self::and)'s cost.
    #[inline]
    pub fn or<O: RmwOrdering>(&self, value: T, order: O)
    where
        T: AtomBitwise,
    {
        let _ = order;
        T::Repr::fetch_or(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Applies `^ value`: [`fetch_xor`](Self::fetch_xor) with the value before discarded, at
    /// [`and`](Self::and)'s cost.
    #[inline]
    pub fn xor<O: RmwOrdering>(&self, value: T, order: O)
    where
        T: AtomBitwise,
    {
        let _ = order;
        T::Repr::fetch_xor(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Inverts every bit: [`fetch_not`](Self::fetch_not) with the value before discarded, at
    /// [`and`](Self::and)'s cost.
    #[inline]
    pub fn not<O: RmwOrdering>(&self, order: O)
    where
        T: AtomBitwise,
    {
        let _ = order;
        T::Repr::fetch_not(self.primitive_cell(), O::CORE);
    }

    /// Applies `& value`, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `and`, which every target has"]
    #[inline]
    pub fn fetch_and<O: RmwOrdering>(&self, value: T, order: O) -> T
    where
        T: AtomBitwise,
        T::Repr: FetchBitwise,
    {
        let _ = order;
        let before = T::Repr::fetch_and(self.primitive_cell(), value.to_repr(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Applies `| value`, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `or`, which every target has"]
    #[inline]
    pub fn fetch_or<O: RmwOrdering>(&self, value: T, order: O) -> T
    where
        T: AtomBitwise,
        T::Repr: FetchBitwise,
    {
        let _ = order;
        let before = T::Repr::fetch_or(self.primitive_cell(), value.to_repr(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Applies `^ value`, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `xor`, which every target has"]
    #[inline]
    pub fn fetch_xor<O: RmwOrdering>(&self, value: T, order: O) -> T
    where
        T: AtomBitwise,
        T::Repr: FetchBitwise,
    {
        let _ = order;
        let before = T::Repr::fetch_xor(self.primitive_cell(), value.to_repr(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Inverts every bit, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the value before, call `not`, which every target has"]
    #[inline]
    pub fn fetch_not<O: RmwOrdering>(&self, order: O) -> T
    where
        T: AtomBitwise,
        T::Repr: FetchBitwise,
    {
        let _ = order;
        let before = T::Repr::fetch_not(self.primitive_cell(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }
}
