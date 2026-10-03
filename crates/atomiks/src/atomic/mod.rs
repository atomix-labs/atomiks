//! The typed atomic, and the operations every value has.

use core::fmt;
use core::marker::PhantomData;
use core::panic::{RefUnwindSafe, UnwindSafe};

use crate::atom::Atom;
use crate::ordering::{LoadOrdering, Relaxed, RmwOrdering, StoreOrdering};
use crate::primitive::{CellAccess, CompareExchange, ExactBits, Load, Primitive, Store, Swap};
use crate::validity::Validity;

mod capability;
mod exclusive;
mod ptr;

pub use self::ptr::AtomicPtr;

/// A value of `T` shared between threads through one atomic word.
///
/// Each operation through `&self` is one atomic instruction on `T`'s repr, decoded on the way
/// out, except [`update`](Self::update), [`try_update`](Self::try_update) and
/// [`store_rmw`](Self::store_rmw), which are compare-exchange loops. That holds in an optimized
/// build, and on `aarch64` with LSE: [`and`](Self::and) says what `x86_64` compiles unoptimized,
/// and on `aarch64` without LSE a read-modify-write is an outline call or an LL/SC pair. An
/// operation the target lacks for the repr does not exist ([`Load`], [`Store`], [`Swap`],
/// [`FetchBitwise`](crate::FetchBitwise), [`MinMax`](crate::MinMax)), and a capability
/// ([`AtomAdd`](crate::AtomAdd), [`AtomOrd`](crate::AtomOrd), [`AtomBitwise`](crate::AtomBitwise))
/// gates each read-modify-write that means something only on some values.
///
/// [`new`](Self::new) and [`into_inner`](Self::into_inner) need `T: const Atom`, so generic
/// run-time code builds one with [`From`] and reads it back with [`get`](Self::get); a const
/// caller of `get` or [`set`](Self::set) needs `const_trait_impl`.
///
/// # Examples
/// ```
/// use std::thread;
///
/// use atomiks::AtomicI64;
/// use atomiks::ordering::Relaxed;
///
/// // The lowest ask any thread has seen, in ticks; it publishes nothing else, so Relaxed.
/// static LOW: AtomicI64 = AtomicI64::new(i64::MAX);
///
/// thread::scope(|scope| {
///     for ask in [10_250, 10_100, 10_400] {
///         scope.spawn(move || LOW.update(Relaxed, Relaxed, |low| low.min(ask)));
///     }
/// });
/// assert_eq!(LOW.load(Relaxed), 10_100, "the lowest of the three asks");
/// ```
#[repr(transparent)]
pub struct Atomic<T: Atom> {
    // INVARIANT: holds a repr that decodes: one `to_repr` returned; one a read-modify-write left,
    // which either needs `Total` (add, the bitwise operations, the pointer offsets), keeps one of
    // its operands (max, min), or leaves the repr as it was (`load_rmw`, and a 128-bit
    // `read_for_rmw` without `Load`, each a compare-exchange of zero for zero, on an integer repr,
    // whose exchange compares every bit); or any repr written through `get_mut`, whose `Total`
    // bound makes every one decode. Its writers are this module and its submodules, whoever writes
    // through `get_mut`'s place, and whoever writes through `as_ptr` or `from_ptr`, whose
    // contracts keep it.
    /// The cell holding `T`'s repr: the validity's wrapper around the primitive's cell.
    cell: <T::Validity as Validity>::Cell<T::Repr>,
    /// The type of the value the repr encodes.
    ///
    /// `cell`'s type, a projection of `T`, makes `Atomic<T>` invariant in `T`, and soundness rests
    /// on it: were `Atomic<T>` covariant, a `&Atomic<R<'static>>` would shrink to a
    /// `&Atomic<R<'a>>`, and a store through the shorter view would leave an `R<'a>` where the
    /// `'static` one reads it.
    marker: PhantomData<T>,
}

/// An atomic `bool`.
pub type AtomicBool = Atomic<bool>;
/// An atomic `u8`.
pub type AtomicU8 = Atomic<u8>;
/// An atomic `u16`.
pub type AtomicU16 = Atomic<u16>;
/// An atomic `u32`.
pub type AtomicU32 = Atomic<u32>;
/// An atomic `u64`.
pub type AtomicU64 = Atomic<u64>;
/// An atomic `usize`.
pub type AtomicUsize = Atomic<usize>;
/// An atomic `i8`.
pub type AtomicI8 = Atomic<i8>;
/// An atomic `i16`.
pub type AtomicI16 = Atomic<i16>;
/// An atomic `i32`.
pub type AtomicI32 = Atomic<i32>;
/// An atomic `i64`.
pub type AtomicI64 = Atomic<i64>;
/// An atomic `isize`.
pub type AtomicIsize = Atomic<isize>;
/// An atomic `u128`.
#[cfg(any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "cmpxchg16b")))]
pub type AtomicU128 = Atomic<u128>;
/// An atomic `i128`.
#[cfg(any(target_arch = "aarch64", all(target_arch = "x86_64", target_feature = "cmpxchg16b")))]
pub type AtomicI128 = Atomic<i128>;

// SAFETY: every shared access is one atomic operation on the cell, and `Atom` declares the value
// may cross threads as its repr.
#[expect(unsafe_code, reason = "the marker would deny it for a raw pointer, which is not `Sync`")]
unsafe impl<T: Atom> Sync for Atomic<T> {}

// SAFETY: moving the atomic moves only its repr, which `Atom` declares may cross threads.
#[expect(unsafe_code, reason = "as `Sync`")]
unsafe impl<T: Atom> Send for Atomic<T> {}

// As `T`'s: a panic leaves the cell holding a whole repr. Written out: core's `AtomicPtr<P>` is
// `UnwindSafe` only where `P: RefUnwindSafe`, so the cell's bounds cannot promise it.
impl<T: Atom + UnwindSafe> UnwindSafe for Atomic<T> {}

// For every `T`, as core's `AtomicPtr<T>` is: shared access is only atomic operations, so a panic
// leaves a whole repr.
impl<T: Atom> RefUnwindSafe for Atomic<T> {}

impl<T: Atom> Atomic<T> {
    /// An atomic holding `value`.
    #[cfg(not(loom))]
    #[inline]
    #[must_use]
    pub const fn new(value: T) -> Self
    where
        T: const Atom,
    {
        Self {
            cell: T::Validity::wrap::<T::Repr>(value.to_repr().into_cell()),
            marker: PhantomData,
        }
    }

    /// An atomic holding `value`; not `const` under loom, whose cells are built at run time.
    #[cfg(loom)]
    #[inline]
    #[must_use]
    pub fn new(value: T) -> Self {
        Self::from(value)
    }

    /// The value, consuming the atomic.
    #[cfg(not(loom))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[must_use]
    pub const fn into_inner(self) -> T
    where
        T: const Atom,
    {
        let repr = T::Repr::from_cell(T::Validity::into_inner::<T::Repr>(self.cell));
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(repr) }
    }

    /// The value, consuming the atomic.
    #[cfg(loom)]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> T {
        let repr = T::Repr::from_cell(T::Validity::into_inner::<T::Repr>(self.cell));
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(repr) }
    }

    /// The primitive's cell, inside the validity's wrapper.
    #[inline]
    const fn primitive_cell(&self) -> &<T::Repr as CellAccess>::Cell {
        T::Validity::get_ref::<T::Repr>(&self.cell)
    }

    /// Reads the value.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn load<O: LoadOrdering>(&self, order: O) -> T
    where
        T::Repr: Load,
    {
        let _ = order;
        let repr = T::Repr::load(self.primitive_cell(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(repr) }
    }

    /// Writes `value`.
    #[inline]
    pub fn store<O: StoreOrdering>(&self, value: T, order: O)
    where
        T::Repr: Store,
    {
        let _ = order;
        T::Repr::store(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Writes `value`, and returns the value before.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn swap<O: RmwOrdering>(&self, value: T, order: O) -> T
    where
        T::Repr: Swap,
    {
        let _ = order;
        let before = T::Repr::swap(self.primitive_cell(), value.to_repr(), O::CORE);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(before) }
    }

    /// Writes `new` if the value is `current`, and returns the value before.
    ///
    /// It compares reprs: `-0.0` and `0.0` differ, and a NaN matches only its own bits.
    ///
    /// # Errors
    /// The value the exchange read, when it is not `current`.
    ///
    /// # Examples
    /// ```
    /// use atomiks::AtomicU64;
    /// use atomiks::ordering::{AcqRel, Acquire};
    ///
    /// // The session that owns the order book, or 0 while it is free.
    /// let owner = AtomicU64::new(0);
    /// assert_eq!(owner.compare_exchange(0, 7, AcqRel, Acquire), Ok(0), "session 7 claims it");
    /// assert_eq!(owner.compare_exchange(0, 9, AcqRel, Acquire), Err(7), "9 finds 7's claim");
    /// ```
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn compare_exchange<S: RmwOrdering, F: LoadOrdering>(
        &self, current: T, new: T, success: S, failure: F,
    ) -> Result<T, T> {
        let _ = (success, failure);
        let cell = self.primitive_cell();
        match T::Repr::compare_exchange(cell, current.to_repr(), new.to_repr(), S::CORE, F::CORE) {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            Ok(before) => Ok(unsafe { T::from_repr_unchecked(before) }),
            // SAFETY: as above.
            Err(found) => Err(unsafe { T::from_repr_unchecked(found) }),
        }
    }

    /// As [`compare_exchange`](Self::compare_exchange), but allowed to fail spuriously.
    ///
    /// A retry loop absorbs the spurious failures.
    ///
    /// # Errors
    /// The value the exchange read, when it is not `current` or the exchange failed spuriously.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn compare_exchange_weak<S: RmwOrdering, F: LoadOrdering>(
        &self, current: T, new: T, success: S, failure: F,
    ) -> Result<T, T> {
        let _ = (success, failure);
        let cell = self.primitive_cell();
        match T::Repr::compare_exchange_weak(
            cell,
            current.to_repr(),
            new.to_repr(),
            S::CORE,
            F::CORE,
        ) {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            Ok(before) => Ok(unsafe { T::from_repr_unchecked(before) }),
            // SAFETY: as above.
            Err(found) => Err(unsafe { T::from_repr_unchecked(found) }),
        }
    }

    /// Replaces the value with `f` of it, and returns the value before.
    ///
    /// It retries until no other write intervenes, so `f` may run more than once; its first read
    /// is a load, or a compare-exchange where the repr has no [`Load`]. `set_order` orders the
    /// exchange that lands; `fetch_order` orders every read, the first and each failed exchange's,
    /// as [`compare_exchange`](Self::compare_exchange)'s `success` and `failure`.
    ///
    /// # Examples
    /// ```
    /// use atomiks::AtomicU32;
    /// use atomiks::ordering::{AcqRel, Acquire};
    ///
    /// // A retry delay in microseconds, doubled on each failure up to a millisecond.
    /// let delay = AtomicU32::new(400);
    /// let double = |micros: u32| (micros * 2).min(1_000);
    /// assert_eq!(delay.update(AcqRel, Acquire, double), 400, "the delay before");
    /// assert_eq!(delay.update(AcqRel, Acquire, double), 800, "doubled");
    /// assert_eq!(delay.load(Acquire), 1_000, "and capped at a millisecond");
    /// ```
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn update<S: RmwOrdering, F: LoadOrdering, U: FnMut(T) -> T>(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> T {
        let _ = (set_order, fetch_order);
        let cell = self.primitive_cell();
        let mut seen = T::Repr::read_for_rmw(cell, F::CORE);
        loop {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            let next = f(unsafe { T::from_repr_unchecked(seen) }).to_repr();
            match T::Repr::compare_exchange_weak(cell, seen, next, S::CORE, F::CORE) {
                // SAFETY: as above.
                Ok(before) => return unsafe { T::from_repr_unchecked(before) },
                Err(found) => seen = found,
            }
        }
    }

    /// As [`update`](Self::update), stopping without a write when `f` returns `None`.
    ///
    /// # Errors
    /// The value seen, when `f` declined it.
    ///
    /// # Examples
    /// ```
    /// use atomiks::AtomicU32;
    /// use atomiks::ordering::{AcqRel, Acquire};
    ///
    /// // Seats left on a flight: a booking takes one, and none once they run out.
    /// let seats = AtomicU32::new(1);
    /// let book = |left: u32| left.checked_sub(1);
    /// assert_eq!(seats.try_update(AcqRel, Acquire, book), Ok(1), "the last seat booked");
    /// assert_eq!(seats.try_update(AcqRel, Acquire, book), Err(0), "none left: nothing written");
    /// ```
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn try_update<S: RmwOrdering, F: LoadOrdering, U: FnMut(T) -> Option<T>>(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> Result<T, T> {
        let _ = (set_order, fetch_order);
        let cell = self.primitive_cell();
        let mut seen = T::Repr::read_for_rmw(cell, F::CORE);
        loop {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            let current = unsafe { T::from_repr_unchecked(seen) };
            let Some(next) = f(current) else { return Err(current) };
            match T::Repr::compare_exchange_weak(cell, seen, next.to_repr(), S::CORE, F::CORE) {
                // SAFETY: as above.
                Ok(before) => return Ok(unsafe { T::from_repr_unchecked(before) }),
                Err(found) => seen = found,
            }
        }
    }

    /// Reads the value with a compare-exchange, for an [`ExactBits`] repr without [`Load`].
    ///
    /// The exchange takes the cache line exclusive and writes it, so it faults on read-only memory.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn load_rmw<O: LoadOrdering>(&self, order: O) -> T
    where
        T::Repr: ExactBits,
    {
        let _ = order;
        let zero = T::Repr::from_bits(0);
        // It writes zero only over zero, and an integer's exchange compares every bit, so the repr
        // stays as it was, whether or not zero decodes. A pointer's compares only the address, so
        // over address zero it would write a null without the pointer's provenance: hence
        // `ExactBits`, which costs nothing, as every pointer has a `Load`.
        match T::Repr::compare_exchange(self.primitive_cell(), zero, zero, O::CORE, O::CORE) {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            Ok(current) | Err(current) => unsafe { T::from_repr_unchecked(current) },
        }
    }

    /// Writes `value` with a compare-exchange loop, for a primitive without [`Store`].
    #[inline]
    pub fn store_rmw<O: StoreOrdering>(&self, value: T, order: O) {
        // ORDERING: `order` on the exchange that lands, pairing as the caller's store would;
        // Relaxed on the reads before it, which only feed the comparison and pair with nothing.
        let _ = self.update(order, Relaxed, |_| value);
    }

    /// The repr's address, for interop.
    ///
    /// Every access through it while the atomic is shared is atomic and of the repr's width, and a
    /// write leaves a repr [`from_repr`](Atom::from_repr) decodes.
    #[cfg(not(loom))]
    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *mut T::Repr {
        T::Repr::as_ptr(self.primitive_cell())
    }

    /// The atomic `ptr` points to.
    ///
    /// # Safety
    /// For all of `'a`: `ptr` is aligned to `align_of::<Atomic<T>>()` and valid for reads and
    /// writes; no non-atomic or different-width access reaches it without synchronization; and
    /// the pointee holds a repr [`from_repr`](Atom::from_repr) decodes (any repr, where
    /// `T::Validity` is [`Total`](crate::validity::Total)), which every write keeps.
    ///
    /// # Examples
    /// ```
    /// use atomiks::AtomicU64;
    /// use atomiks::ordering::{Acquire, Release};
    ///
    /// // A sequence number laid out as a plain `u64`, as in a mapped page.
    /// let mut seq = 0_u64;
    /// {
    ///     // SAFETY: `seq` is a live `u64`, aligned as `AtomicU64` on every supported target, and
    ///     // every repr of it decodes; it outlives `view`, and nothing else reaches it while `view`
    ///     // lives.
    ///     let view = unsafe { AtomicU64::from_ptr(&raw mut seq) };
    ///     view.store(1, Release);
    ///     assert_eq!(view.load(Acquire), 1, "the view reads its own store");
    /// }
    /// assert_eq!(seq, 1, "and the plain `u64` holds it once the view has ended");
    /// ```
    #[cfg(not(loom))]
    #[expect(unsafe_code, reason = "a reference to memory only the caller knows")]
    #[inline]
    #[must_use]
    pub const unsafe fn from_ptr<'a>(ptr: *mut T::Repr) -> &'a Self {
        // SAFETY: `Atomic<T>` is `repr(transparent)` over `T::Validity`'s cell, which has the
        // primitive's cell's layout (the cell itself, or `Opaque`'s `repr(transparent)` over it),
        // which has the primitive's size; the caller upholds `from_ptr`'s contract, which keeps the
        // field INVARIANT.
        unsafe { &*ptr.cast::<Self>() }
    }
}

impl<T: Atom> From<T> for Atomic<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self {
            cell: T::Validity::wrap::<T::Repr>(value.to_repr().into_cell()),
            marker: PhantomData,
        }
    }
}

impl<T: Atom + Default> Default for Atomic<T> {
    #[inline]
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T: Atom + fmt::Debug> fmt::Debug for Atomic<T>
where
    T::Repr: Load,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ORDERING: Relaxed, as core's `Debug`: printing publishes nothing and pairs with no store.
        self.load(Relaxed).fmt(f)
    }
}
