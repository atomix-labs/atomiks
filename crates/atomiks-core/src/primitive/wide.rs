//! `u128` and `i128`, over a 16-byte cell of this crate's own.
//!
//! Core's `AtomicU128` reaches the cell on `aarch64`; `cmpxchg16b`, and AVX's 16-byte load and
//! store, on `x86_64`. Loom gets a model.

#[cfg(not(loom))]
use core::cell::UnsafeCell;
#[cfg(not(loom))]
use core::convert::identity;
#[cfg(not(loom))]
use core::fmt;
#[cfg(not(loom))]
use core::panic::RefUnwindSafe;
#[cfg(not(loom))]
use core::ptr;
#[cfg(not(loom))]
use core::sync::atomic::Ordering as CoreOrdering;

#[cfg(all(feature = "zerocopy-08", not(loom)))]
use zerocopy::{FromBytes, IntoBytes, KnownLayout};

#[cfg(not(loom))]
use super::{CellAccess, CompareExchange, RawAccess};
use super::{ExactBits, Primitive};
#[cfg(all(wide_load_store, not(loom)))]
use super::{Load, Store};

/// Implements `Primitive` and `ExactBits` for 128-bit integers.
macro_rules! wide_bits {
    ($($int:ty),+) => {$(
        const impl Primitive for $int {
            const BITS: u32 = 128;
            #[inline]
            fn from_bits(bits: u128) -> Self {
                bits.wrapping_cast()
            }
            #[inline]
            fn is_bits(self, bits: u128) -> bool {
                self.to_bits() == bits
            }
            #[inline]
            fn packed_bits(self) -> u128 {
                self.to_bits()
            }
            #[inline]
            fn with_packed_bits(self, bits: u128) -> Self {
                Self::from_bits(bits)
            }
        }
        const impl ExactBits for $int {
            #[inline]
            fn to_bits(self) -> u128 {
                self.wrapping_cast()
            }
        }
    )+};
}

wide_bits!(u128, i128);

/// A 16-byte atomic cell.
///
/// With zerocopy, its bits are plain memory, read and written as bytes.
#[cfg(not(loom))]
#[repr(C, align(16))]
#[cfg_attr(all(feature = "zerocopy-08", not(loom)), derive(KnownLayout, IntoBytes, FromBytes))]
pub struct Wide {
    // INVARIANT: while shared, every access is a 16-byte atomic: core's `AtomicU128` on `aarch64`;
    // `cmpxchg16b`, or AVX's load and store, on `x86_64`; or a caller's through `as_ptr` or
    // `from_ptr`, whose contracts (`Atomic`'s) allow no non-atomic or different-width access.
    /// The bits.
    bits: UnsafeCell<u128>,
}

// SAFETY: by the field INVARIANT, every shared access to `bits` is a 16-byte atomic.
#[cfg(not(loom))]
#[expect(unsafe_code, reason = "a cell every shared access to which is atomic")]
unsafe impl Sync for Wide {}

// As core's atomics: every shared access is atomic, so a panic leaves no torn value.
#[cfg(not(loom))]
impl RefUnwindSafe for Wide {}

#[cfg(not(loom))]
impl fmt::Debug for Wide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Wide").finish_non_exhaustive()
    }
}

#[cfg(not(loom))]
impl Wide {
    /// A cell holding `bits`.
    #[inline]
    const fn new(bits: u128) -> Self {
        Self { bits: UnsafeCell::new(bits) }
    }

    /// The bits, consuming the cell.
    #[inline]
    const fn into_inner(self) -> u128 {
        self.bits.into_inner()
    }

    /// The bits' place, through exclusive access.
    #[inline]
    const fn get_mut(&mut self) -> &mut u128 {
        self.bits.get_mut()
    }

    /// The bits' address.
    #[inline]
    const fn as_ptr(&self) -> *mut u128 {
        self.bits.get()
    }
}

/// Implements the cell traits and `RawAccess` for a 128-bit integer over `Wide`.
///
/// `$to` converts the integer to its `u128`, `$from` converts back, and `$place` lends its place.
#[cfg(not(loom))]
macro_rules! cells {
    ($($int:ty: $to:path, $from:path, $place:path);+ $(;)?) => {$(
        const impl CellAccess for $int {
            type Cell = Wide;
            #[inline]
            fn into_cell(self) -> Wide {
                Wide::new($to(self))
            }
            #[inline]
            fn from_cell(cell: Wide) -> Self {
                $from(cell.into_inner())
            }
            #[inline]
            fn get(cell: &mut Wide) -> Self {
                $from(*cell.get_mut())
            }
            #[inline]
            fn set(cell: &mut Wide, value: Self) {
                *cell.get_mut() = $to(value);
            }
        }
        const impl RawAccess for $int {
            #[inline]
            fn get_mut(cell: &mut Wide) -> &mut Self {
                $place(cell.get_mut())
            }
            #[inline]
            fn as_ptr(cell: &Wide) -> *mut Self {
                cell.as_ptr().cast()
            }
        }
        impl CompareExchange for $int {
            #[inline]
            fn read_for_rmw(cell: &Wide, order: CoreOrdering) -> Self {
                $from(cell.read_for_rmw(order))
            }
            #[inline]
            fn compare_exchange(
                cell: &Wide, current: Self, new: Self, success: CoreOrdering, failure: CoreOrdering,
            ) -> Result<Self, Self> {
                cell.compare_exchange($to(current), $to(new), success, failure)
                    .map($from)
                    .map_err($from)
            }
            #[inline]
            fn compare_exchange_weak(
                cell: &Wide, current: Self, new: Self, success: CoreOrdering, failure: CoreOrdering,
            ) -> Result<Self, Self> {
                cell.compare_exchange_weak($to(current), $to(new), success, failure)
                    .map($from)
                    .map_err($from)
            }
        }
        #[cfg(wide_load_store)]
        impl Load for $int {
            #[inline]
            fn load(cell: &Wide, order: CoreOrdering) -> Self {
                $from(cell.load(order))
            }
        }
        #[cfg(wide_load_store)]
        impl Store for $int {
            #[inline]
            fn store(cell: &Wide, value: Self, order: CoreOrdering) {
                cell.store($to(value), order);
            }
        }
    )+};
}

#[cfg(not(loom))]
cells! {
    u128: identity, identity, identity;
    i128: i128::cast_unsigned, u128::cast_signed, signed_place;
}

/// The `i128` a `u128` place holds.
#[cfg(not(loom))]
#[expect(unsafe_code, reason = "the `u128` place lent as the `i128` it holds")]
const fn signed_place(bits: &mut u128) -> &mut i128 {
    // SAFETY: `u128` and `i128` share size, alignment and validity, so the exclusive place of the
    // one is the place of the other for the same borrow.
    unsafe { &mut *ptr::from_mut(bits).cast::<i128>() }
}

#[cfg(core_atomic_u128)]
mod aarch64 {
    //! Core's 16-byte atomic over the cell's bits.
    //!
    //! Compare-exchange is `casp`, or without LSE an outline call (`__aarch64_cas16_*`); with
    //! LSE2, the load and store are `ldp` and `stp`.

    use core::sync::atomic::{AtomicU128, Ordering as CoreOrdering};

    use super::Wide;

    impl Wide {
        /// Core's atomic over the bits.
        #[expect(unsafe_code, reason = "core's atomic over the cell's own bits")]
        #[inline]
        const fn atomic(&self) -> &AtomicU128 {
            // SAFETY: `bits` is valid for reads and writes and aligned to 16 (the `repr`) for the
            // borrow, and by the field INVARIANT every shared access to it is a 16-byte atomic.
            unsafe { AtomicU128::from_ptr(self.bits.get()) }
        }

        /// A compare-exchange on the bits.
        #[inline]
        pub(super) fn compare_exchange(
            &self, current: u128, new: u128, success: CoreOrdering, failure: CoreOrdering,
        ) -> Result<u128, u128> {
            self.atomic().compare_exchange(current, new, success, failure)
        }

        /// As `compare_exchange`, but may fail while the cell holds `current`.
        #[inline]
        pub(super) fn compare_exchange_weak(
            &self, current: u128, new: u128, success: CoreOrdering, failure: CoreOrdering,
        ) -> Result<u128, u128> {
            self.atomic().compare_exchange_weak(current, new, success, failure)
        }

        /// An update loop's first read: the LSE2 load, else a compare-exchange of zero for zero.
        #[inline]
        pub(super) fn read_for_rmw(&self, order: CoreOrdering) -> u128 {
            #[cfg(target_feature = "lse2")]
            return self.load(order);
            #[cfg(not(target_feature = "lse2"))]
            match self.compare_exchange(0, 0, order, order) {
                Ok(value) | Err(value) => value,
            }
        }

        /// The LSE2 16-byte load.
        #[cfg(target_feature = "lse2")]
        #[inline]
        pub(super) fn load(&self, order: CoreOrdering) -> u128 {
            self.atomic().load(order)
        }

        /// The LSE2 16-byte store.
        #[cfg(target_feature = "lse2")]
        #[inline]
        pub(super) fn store(&self, value: u128, order: CoreOrdering) {
            self.atomic().store(value, order);
        }
    }
}

#[cfg(all(target_arch = "x86_64", not(loom)))]
mod x86_64 {
    //! `cmpxchg16b` for compare-exchange and every loop; with AVX, the 16-byte load and store.
    //!
    //! Intel and AMD guarantee that load and store single-copy atomic when 16-byte aligned.

    use core::arch::x86_64::cmpxchg16b;
    #[cfg(target_feature = "avx")]
    use core::intrinsics::{AtomicOrdering, atomic_load, atomic_store};
    use core::sync::atomic::Ordering as CoreOrdering;

    use super::Wide;

    impl Wide {
        /// The bits a compare-exchange found, whether or not it wrote `new`.
        #[expect(unsafe_code, reason = "`cmpxchg16b` on the cell's own 16-byte-aligned bits")]
        #[inline]
        fn exchange(
            &self, current: u128, new: u128, success: CoreOrdering, failure: CoreOrdering,
        ) -> u128 {
            // SAFETY: `bits` is valid and aligned to 16 for the cell's life, every shared access
            // is atomic (the field INVARIANT), and the target has `cmpxchg16b`, which the gate on
            // `mod wide` in `primitive/mod.rs` requires.
            unsafe { cmpxchg16b(self.bits.get(), current, new, success, failure) }
        }

        /// A compare-exchange on the bits.
        #[inline]
        pub(super) fn compare_exchange(
            &self, current: u128, new: u128, success: CoreOrdering, failure: CoreOrdering,
        ) -> Result<u128, u128> {
            let previous = self.exchange(current, new, success, failure);
            if previous == current { Ok(previous) } else { Err(previous) }
        }

        /// As `compare_exchange`, which never fails spuriously.
        #[inline]
        pub(super) fn compare_exchange_weak(
            &self, current: u128, new: u128, success: CoreOrdering, failure: CoreOrdering,
        ) -> Result<u128, u128> {
            self.compare_exchange(current, new, success, failure)
        }

        /// An update loop's first read: the AVX load, else a compare-exchange of zero for zero,
        /// whose found bits it returns as they are, so no `Result` asks for a `cmov` to join them.
        #[inline]
        pub(super) fn read_for_rmw(&self, order: CoreOrdering) -> u128 {
            #[cfg(target_feature = "avx")]
            return self.load(order);
            #[cfg(not(target_feature = "avx"))]
            self.exchange(0, 0, order, order)
        }

        /// The AVX 16-byte load.
        ///
        /// A comparison chain, not a `match`: `Ordering` is non-exhaustive, and its
        /// callers, `Atomic::load` and `read_for_rmw`, pass only a `LoadOrdering`'s, so the
        /// last branch is `SeqCst`'s.
        #[cfg(target_feature = "avx")]
        #[expect(unsafe_code, reason = "the 16-byte atomic load of the cell's own bits")]
        #[inline]
        pub(super) fn load(&self, order: CoreOrdering) -> u128 {
            let bits = self.bits.get().cast_const();
            if order == CoreOrdering::Relaxed {
                // SAFETY: `bits` is valid and aligned to 16, and every shared access is atomic (the
                // field INVARIANT).
                unsafe { atomic_load::<u128, { AtomicOrdering::Relaxed }, false>(bits) }
            } else if order == CoreOrdering::Acquire {
                // SAFETY: as the Relaxed load.
                unsafe { atomic_load::<u128, { AtomicOrdering::Acquire }, false>(bits) }
            } else {
                // SAFETY: as the Relaxed load.
                unsafe { atomic_load::<u128, { AtomicOrdering::SeqCst }, false>(bits) }
            }
        }

        /// The AVX 16-byte store.
        ///
        /// A comparison chain, as the load is: its caller, `Atomic::store`, passes only a
        /// `StoreOrdering`'s, so the last branch is `SeqCst`'s.
        #[cfg(target_feature = "avx")]
        #[expect(unsafe_code, reason = "the 16-byte atomic store to the cell's own bits")]
        #[inline]
        pub(super) fn store(&self, value: u128, order: CoreOrdering) {
            let bits = self.bits.get();
            if order == CoreOrdering::Relaxed {
                // SAFETY: `bits` is valid and aligned to 16, and every shared access is atomic (the
                // field INVARIANT).
                unsafe {
                    atomic_store::<u128, { AtomicOrdering::Relaxed }, false>(bits, value);
                }
            } else if order == CoreOrdering::Release {
                // SAFETY: as the Relaxed store.
                unsafe {
                    atomic_store::<u128, { AtomicOrdering::Release }, false>(bits, value);
                }
            } else {
                // SAFETY: as the Relaxed store.
                unsafe {
                    atomic_store::<u128, { AtomicOrdering::SeqCst }, false>(bits, value);
                }
            }
        }
    }
}

#[cfg(loom)]
mod modelled {
    //! Loom has no 16-byte atomics: a loom `AtomicUsize` indexes a table of 128-bit values.
    //!
    //! Each value is interned once, so equal values share an index, a compare-exchange on the index
    //! is one on the value, and every operation keeps its ordering. A compare-exchange interns
    //! `current` too, so it compares the index any thread storing `current` writes.
    //!
    //! The table is std's, shared by every execution and invisible to loom, so it adds no
    //! synchronization to a model.

    use alloc::vec::Vec;
    use core::convert::identity;
    use core::sync::atomic::Ordering as CoreOrdering;
    use std::sync::{Mutex, PoisonError};

    use loom::sync::atomic::AtomicUsize;

    use crate::primitive::{CellAccess, CompareExchange, RawAccess};
    #[cfg(wide_load_store)]
    use crate::primitive::{Load, Store};

    /// Every 128-bit value a model has interned, each once.
    static VALUES: Mutex<Vec<u128>> = Mutex::new(Vec::new());

    /// The index of `value` in the table, adding it if new.
    fn intern(value: u128) -> usize {
        let mut values = VALUES.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = values.iter().position(|interned| *interned == value) {
            return index;
        }
        values.push(value);
        values.len().saturating_sub(1)
    }

    /// The value interned as `index`.
    fn lookup(index: usize) -> u128 {
        let values = VALUES.lock().unwrap_or_else(PoisonError::into_inner);
        values.get(index).copied().unwrap_or_default()
    }

    /// A modelled 16-byte cell.
    #[derive(Debug)]
    pub struct Wide {
        /// The index of the cell's value.
        index: AtomicUsize,
    }

    /// Implements the cell traits and `RawAccess` for one 128-bit integer over the modelled cell.
    ///
    /// `$to` converts the integer to the table's `u128`, and `$from` converts back.
    macro_rules! cells {
        ($($int:ty: $to:path, $from:path);+ $(;)?) => {$(
            impl CellAccess for $int {
                type Cell = Wide;
                #[inline]
                fn into_cell(self) -> Wide {
                    Wide { index: AtomicUsize::new(intern($to(self))) }
                }
                #[inline]
                fn from_cell(cell: Wide) -> Self {
                    $from(lookup(cell.index.into_inner()))
                }
                #[inline]
                fn get(cell: &mut Wide) -> Self {
                    $from(lookup(cell.index.with_mut(|index| *index)))
                }
                #[inline]
                fn set(cell: &mut Wide, value: Self) {
                    let index = intern($to(value));
                    cell.index.with_mut(|place| *place = index);
                }
            }
            impl RawAccess for $int {}
            impl CompareExchange for $int {
                #[inline]
                fn read_for_rmw(cell: &Wide, order: CoreOrdering) -> Self {
                    $from(lookup(cell.index.load(order)))
                }
                #[inline]
                fn compare_exchange(
                    cell: &Wide, current: Self, new: Self, success: CoreOrdering, failure: CoreOrdering,
                ) -> Result<Self, Self> {
                    cell.index
                        .compare_exchange(intern($to(current)), intern($to(new)), success, failure)
                        .map(|index| $from(lookup(index)))
                        .map_err(|index| $from(lookup(index)))
                }
                #[inline]
                fn compare_exchange_weak(
                    cell: &Wide, current: Self, new: Self, success: CoreOrdering, failure: CoreOrdering,
                ) -> Result<Self, Self> {
                    cell.index
                        .compare_exchange_weak(intern($to(current)), intern($to(new)), success, failure)
                        .map(|index| $from(lookup(index)))
                        .map_err(|index| $from(lookup(index)))
                }
            }
            #[cfg(wide_load_store)]
            impl Load for $int {
                #[inline]
                fn load(cell: &Wide, order: CoreOrdering) -> Self {
                    $from(lookup(cell.index.load(order)))
                }
            }
            #[cfg(wide_load_store)]
            impl Store for $int {
                #[inline]
                fn store(cell: &Wide, value: Self, order: CoreOrdering) {
                    cell.index.store(intern($to(value)), order);
                }
            }
        )+};
    }

    cells! {
        u128: identity, identity;
        i128: i128::cast_unsigned, u128::cast_signed;
    }
}
