//! The read-modify-writes that need a capability: add and subtract ([`AtomAdd`]), maximum and
//! minimum ([`AtomOrd`]), and the bitwise operations, a bit's among them ([`AtomBitwise`]).
//!
//! Each bound sits on its method, so a value without the capability gets the capability's own
//! message. `fetch_max` and `fetch_min` also need the target's [`MinMax`], and the `fetch_` bitwise
//! forms its [`FetchBitwise`]: `aarch64` has both, `x86_64` neither. A bit's set, clear and toggle
//! need the target's [`BitTest`]: both have it from 16 bits, and `aarch64` from 8.

use super::Atomic;
use super::field::{bit_mask, has_bit};
use crate::atom::{Atom, AtomAdd, AtomBitwise, AtomOrd};
use crate::ordering::RmwOrdering;
use crate::primitive::{
    BitTest, Bitwise, ExactBits, FetchAdd, FetchBitwise, MaskBitwise, MinMax, Primitive,
};

impl<T: Atom> Atomic<T> {
    /// Adds `delta` to the repr, wrapping, and returns the value before.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
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

    /// Keeps the larger of `value` and the current value, and returns the value before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
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
    ///
    /// These four alone have such a form, under [portable-atomic]'s names, since no other
    /// read-modify-write needs one: a [`fetch_add`](Self::fetch_add) whose value goes unused is
    /// already `lock add` on `x86_64`.
    ///
    /// [portable-atomic]: https://docs.rs/portable-atomic
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

    /// Turns bit `bit` of the repr on, and returns it before: `lock bts` on `x86_64`, `ldset` on
    /// `aarch64`.
    ///
    /// `bit` counts from the repr's lowest bit, modulo its width, as a shift by
    /// [`wrapping_shl`](u64::wrapping_shl) does, so bit 67 of a `u64` is bit 3, and every bit of a
    /// `bool` is its one bit. `x86_64` has no 8-bit `lock bts`, so an 8-bit repr or a `bool` has it
    /// on `aarch64` alone.
    ///
    /// It takes its name from [portable-atomic]'s `bit_set`, and so returns the bit before as that
    /// does. The atomic's own [`set`](Self::set) writes a whole value through `&mut`. A `bool`
    /// field's [`set`](crate::AtomicField::set) returns nothing, since the field's form that
    /// returns the bit is [`test_and_set`](crate::AtomicField::test_and_set); an atomic's form that
    /// discards it is [`or`](Self::or).
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU64;
    /// use atomix::ordering::AcqRel;
    ///
    /// // A bit per worker of a pool, set while the worker is busy.
    /// static BUSY: AtomicU64 = AtomicU64::new(0);
    ///
    /// let worker = 5;
    /// assert!(!BUSY.bit_set(worker, AcqRel), "worker 5 was idle, and this thread took it");
    /// assert!(BUSY.bit_set(worker, AcqRel), "so a second claim finds it busy");
    /// ```
    ///
    /// [portable-atomic]: https://docs.rs/portable-atomic
    #[must_use = "to discard the bit before, call `or`, which every target has"]
    #[inline]
    pub fn bit_set<O: RmwOrdering>(&self, bit: u32, order: O) -> bool
    where
        T: AtomBitwise,
        T::Repr: BitTest,
    {
        let _ = order;
        let mask = bit_mask::<T::Repr>(bit);
        has_bit(T::Repr::fetch_or_mask(self.primitive_cell(), mask, O::CORE), mask)
    }

    /// Turns bit `bit` of the repr off, and returns it before: `lock btr` on `x86_64`, `ldclr` on
    /// `aarch64`.
    ///
    /// As [`bit_set`](Self::bit_set), on the reprs it takes; [`and`](Self::and) is the form that
    /// discards the bit.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU64;
    /// use atomix::ordering::{AcqRel, Acquire};
    ///
    /// // A bit per worker of a pool, set while the worker is busy: worker 5 is.
    /// static BUSY: AtomicU64 = AtomicU64::new(1 << 5);
    ///
    /// assert!(BUSY.bit_clear(5, AcqRel), "worker 5 was busy, and this thread freed it");
    /// assert_eq!(BUSY.load(Acquire), 0, "so every worker is idle");
    /// ```
    #[must_use = "to discard the bit before, call `and`, which every target has"]
    #[inline]
    pub fn bit_clear<O: RmwOrdering>(&self, bit: u32, order: O) -> bool
    where
        T: AtomBitwise,
        T::Repr: BitTest,
    {
        let _ = order;
        let mask = bit_mask::<T::Repr>(bit);
        let others = <T::Repr as MaskBitwise>::Mask::from_bits(!mask.to_bits());
        has_bit(T::Repr::fetch_and_mask(self.primitive_cell(), others, O::CORE), mask)
    }

    /// Inverts bit `bit` of the repr, and returns it before: `lock btc` on `x86_64`, `ldeor` on
    /// `aarch64`.
    ///
    /// As [`bit_set`](Self::bit_set), on the reprs it takes; [`xor`](Self::xor) is the form that
    /// discards the bit.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU32;
    /// use atomix::ordering::AcqRel;
    ///
    /// // A bit per shard of a cache, the parity of the shard's epoch.
    /// static PARITY: AtomicU32 = AtomicU32::new(0);
    ///
    /// let shard = 3;
    /// assert!(!PARITY.bit_toggle(shard, AcqRel), "shard 3 left an even epoch");
    /// assert!(PARITY.bit_toggle(shard, AcqRel), "and then the odd one it entered");
    /// ```
    #[must_use = "to discard the bit before, call `xor`, which every target has"]
    #[inline]
    pub fn bit_toggle<O: RmwOrdering>(&self, bit: u32, order: O) -> bool
    where
        T: AtomBitwise,
        T::Repr: BitTest,
    {
        let _ = order;
        let mask = bit_mask::<T::Repr>(bit);
        has_bit(T::Repr::fetch_xor_mask(self.primitive_cell(), mask, O::CORE), mask)
    }
}
