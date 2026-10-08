//! The typed atomic, and the operations every value has.

use core::fmt;
use core::marker::PhantomData;
use core::panic::{RefUnwindSafe, UnwindSafe};

// Written out, not as an alias: an alias would hide the feature's badge on the derived impls.
#[cfg(all(feature = "zerocopy-08", not(loom)))]
use zerocopy::{FromBytes, IntoBytes, KnownLayout, Unaligned};

use crate::atom::Atom;
use crate::ordering::{LoadOrdering, Relaxed, RmwOrdering, StoreOrdering};
#[cfg(not(loom))]
use crate::primitive::RawAccess;
use crate::primitive::{CellAccess, CompareExchange, Load, Primitive, ReadByExchange, Store, Swap};
use crate::range::{ReprRange, Tags, assert_aligned};
use crate::validity::Validity;

mod capability;
mod exclusive;
mod field;
mod ptr;

#[doc(hidden)]
pub use self::exclusive::RawValue;
pub use self::field::{AtomicField, Field, FieldPath, Join, ProjectFields, Then, TopField, Whole};
#[doc(hidden)]
pub use self::field::{HasPackedField, Reach, project_field};
pub use self::ptr::AtomicPtr;

/// A value of `T` shared between threads through one atomic primitive, its repr.
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
/// run-time code builds one with [`From`] and reads it back with [`get`](Self::get); a const caller
/// of `get` or [`set`](Self::set) needs `const_trait_impl`, of `From`, `const_convert` too, and of
/// `Default`, `const_default`.
///
/// With the `zerocopy-08` feature, and not under loom, it derives zerocopy's traits through its
/// cell, so it has each its cell has: [`KnownLayout`] always; [`IntoBytes`] but for a pointer, and
/// [`Unaligned`] at 8 bits; [`FromBytes`] where every repr decodes and the cell holds an integer.
/// An atomic `bool` or pointer reads only from zeros, or from bytes [`TryFromBytes`] checks, and a
/// pointer's check passes only zeros; any other, as a `char`'s, reads from neither. It is never
/// [`Immutable`], since its shared reference writes: bytes this process holds alone, as a page it
/// lays out before another process maps it, become an atomic through `mut_from_bytes`, then a
/// shared reborrow. Memory another process may already be writing is reached through a pointer, as
/// [`from_ptr`](Self::from_ptr) takes, since a `&mut [u8]` over it would be aliased.
///
/// # Examples
/// ```
/// # extern crate atomix_core as atomix;
/// use std::thread;
///
/// use atomix::AtomicI64;
/// use atomix::ordering::Relaxed;
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
///
/// [`KnownLayout`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.KnownLayout.html
/// [`IntoBytes`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.IntoBytes.html
/// [`Unaligned`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.Unaligned.html
/// [`FromBytes`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.FromBytes.html
/// [`TryFromBytes`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.TryFromBytes.html
/// [`Immutable`]: https://docs.rs/zerocopy/0.8/zerocopy/trait.Immutable.html
#[repr(transparent)]
#[cfg_attr(
    all(feature = "zerocopy-08", not(loom)),
    derive(KnownLayout, IntoBytes, Unaligned, FromBytes)
)]
pub struct Atomic<T: Atom> {
    // INVARIANT: holds a repr that decodes: one `to_repr` returned; one a read-modify-write left,
    // which either needs `Total` (add, the bitwise operations, the pointer offsets), keeps one of
    // its operands (max, min), or leaves the repr as it was (`load_rmw`, and a 128-bit
    // `read_for_rmw` without `Load`, each a compare-exchange of zero for zero, on an integer repr,
    // whose exchange compares every bit, or a double word's, whose cell holds addresses alone, as
    // `ReadByExchange` says); one a field operation left, which changes only the bits
    // its `FieldPath` governs: to a repr of the field's value placed as the path says (`update`,
    // `try_update`), to any pattern of a field whose every pattern decodes (the bitwise
    // operations, on `FieldBitwise`, `bool` among them), or by an add whose carry leaves the word
    // (the field's add, on `FieldAdd`, at a `TopField`), each through the place of a field, never
    // of `Whole`, as `ProjectFields` promises; or any repr written through `get_mut`,
    // `get_mut_slice`, or the place `from_mut` or `from_mut_slice` took, whose `Total` bound makes
    // every one decode; or any repr zerocopy's derives read from bytes or zeros, which they do
    // only where the cell is the primitive's own, so `Total`'s; or the zero repr bytemuck's
    // `Zeroable` writes, only where `ZeroValid` says it decodes. Its writers are this module
    // and its submodules, the zerocopy and bytemuck impls, and whoever writes through
    // `get_mut`'s place, `get_mut_slice`'s, the place `from_mut` or `from_mut_slice` took,
    // `as_ptr` or `from_ptr`, whose bounds and contracts keep it, each bounded by `RawAccess`,
    // which no double word has.
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
#[cfg(wide)]
pub type AtomicU128 = Atomic<u128>;
/// An atomic `i128`.
#[cfg(wide)]
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

// For every `T`, as core's atomics are: the atomic holds `T`'s repr, never a `T` to pin, and every
// cell is `Unpin`.
impl<T: Atom> Unpin for Atomic<T> {}

/// The reprs of an exchange's `current` and `new`, whose pointers, where `T` is a tagged pointer,
/// are tested for their tags in one test, so one refusal stands on the exchange's path.
///
/// # Panics
/// Where either's pointer has a bit set where its tags go.
#[inline]
#[track_caller]
fn exchanged_reprs<T: Atom>(current: T, new: T) -> (T::Repr, T::Repr) {
    let (current, current_misaligned) = current.to_tagged_repr(Tags::EMPTY);
    let (new, new_misaligned) = new.to_tagged_repr(Tags::EMPTY);
    assert_aligned::<T>(current_misaligned | new_misaligned);
    (current, new)
}

impl<T: Atom> Atomic<T> {
    /// An atomic holding `value`.
    ///
    /// Needs a `const` [`Atom`] impl; any other impl builds with [`From`].
    ///
    /// # Panics
    /// As [`store`](Self::store); in a constant, the build fails instead.
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
    ///
    /// # Panics
    /// As [`store`](Self::store).
    #[cfg(loom)]
    #[inline]
    #[must_use]
    pub fn new(value: T) -> Self {
        Self::from(value)
    }

    /// The value, consuming the atomic.
    ///
    /// Needs a `const` [`Atom`] impl, as [`new`](Self::new) does; with any other impl, read the
    /// value back with [`get`](Self::get).
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
    ///
    /// # Panics
    /// Where `T` is a tagged pointer, a derived pointer word or pointer enum, and `value`'s pointer
    /// has a bit set where its tags go.
    #[inline]
    pub fn store<O: StoreOrdering>(&self, value: T, order: O)
    where
        T::Repr: Store,
    {
        let _ = order;
        T::Repr::store(self.primitive_cell(), value.to_repr(), O::CORE);
    }

    /// Writes `value`, and returns the value before.
    ///
    /// # Panics
    /// As [`store`](Self::store).
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
    /// # Panics
    /// Where `T` is a tagged pointer, a derived pointer word or pointer enum, and `current`'s or
    /// `new`'s pointer has a bit set where its tags go.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU64;
    /// use atomix::ordering::{AcqRel, Acquire};
    ///
    /// // The session that owns the order book, or 0 while it is free.
    /// let owner = AtomicU64::new(0);
    /// assert_eq!(owner.compare_exchange(0, 7, AcqRel, Acquire), Ok(0), "session 7 claims it");
    /// assert_eq!(owner.compare_exchange(0, 9, AcqRel, Acquire), Err(7), "9 finds 7's claim");
    /// ```
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[track_caller]
    pub fn compare_exchange<S: RmwOrdering, F: LoadOrdering>(
        &self, current: T, new: T, success: S, failure: F,
    ) -> Result<T, T> {
        let _ = (success, failure);
        let (current, new) = exchanged_reprs(current, new);
        match T::Repr::compare_exchange(self.primitive_cell(), current, new, S::CORE, F::CORE) {
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
    ///
    /// # Panics
    /// As [`compare_exchange`](Self::compare_exchange).
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    #[track_caller]
    pub fn compare_exchange_weak<S: RmwOrdering, F: LoadOrdering>(
        &self, current: T, new: T, success: S, failure: F,
    ) -> Result<T, T> {
        let _ = (success, failure);
        let (current, new) = exchanged_reprs(current, new);
        match T::Repr::compare_exchange_weak(self.primitive_cell(), current, new, S::CORE, F::CORE)
        {
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
    /// # Panics
    /// Where `T` is a tagged pointer, a derived pointer word or pointer enum, and a value `f`
    /// returns has a pointer with a bit set where its tags go.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU32;
    /// use atomix::ordering::{AcqRel, Acquire};
    ///
    /// // A retry delay in microseconds, doubled on each failure up to a millisecond.
    /// let delay = AtomicU32::new(400);
    /// let double = |micros: u32| (micros * 2).min(1_000);
    /// assert_eq!(delay.update(AcqRel, Acquire, double), 400, "the delay before");
    /// assert_eq!(delay.update(AcqRel, Acquire, double), 800, "doubled");
    /// assert_eq!(delay.load(Acquire), 1_000, "and capped at a millisecond");
    /// ```
    #[expect(unsafe_code, reason = "decodes reprs read from the cell")]
    #[inline]
    pub fn update<S: RmwOrdering, F: LoadOrdering, U: FnMut(T) -> T>(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> T {
        let replace = |seen| {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            Some(f(unsafe { T::from_repr_unchecked(seen) }).to_repr())
        };
        // SAFETY: each repr `replace` returns is a value's own, which decodes.
        let replaced = unsafe { self.try_update_repr(set_order, fetch_order, replace) };
        match replaced {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            Ok(before) | Err(before) => unsafe { T::from_repr_unchecked(before) },
        }
    }

    /// As [`update`](Self::update), stopping without a write when `f` returns `None`.
    ///
    /// # Errors
    /// The value seen, when `f` declined it.
    ///
    /// # Panics
    /// As [`update`](Self::update).
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU32;
    /// use atomix::ordering::{AcqRel, Acquire};
    ///
    /// // Seats left on a flight: a booking takes one, and none once they run out.
    /// let seats = AtomicU32::new(1);
    /// let book = |left: u32| left.checked_sub(1);
    /// assert_eq!(seats.try_update(AcqRel, Acquire, book), Ok(1), "the last seat booked");
    /// assert_eq!(seats.try_update(AcqRel, Acquire, book), Err(0), "none left: nothing written");
    /// ```
    #[expect(unsafe_code, reason = "decodes reprs read from the cell")]
    #[inline]
    pub fn try_update<S: RmwOrdering, F: LoadOrdering, U: FnMut(T) -> Option<T>>(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> Result<T, T> {
        let replace = |seen| {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            f(unsafe { T::from_repr_unchecked(seen) }).map(Atom::to_repr)
        };
        // SAFETY: each repr `replace` returns is a value's own, which decodes.
        let replaced = unsafe { self.try_update_repr(set_order, fetch_order, replace) };
        match replaced {
            // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
            Ok(before) => Ok(unsafe { T::from_repr_unchecked(before) }),
            // SAFETY: as above.
            Err(seen) => Err(unsafe { T::from_repr_unchecked(seen) }),
        }
    }

    /// Replaces the repr with `f` of it, until no other write intervenes, and returns the repr
    /// before; `Err` with the repr seen, without a write, when `f` returns `None`.
    ///
    /// The loop of [`update`](Self::update) and [`try_update`](Self::try_update), and of each
    /// field's.
    ///
    /// # Safety
    /// Each repr `f` returns decodes, as the field INVARIANT asks of each repr the cell holds.
    #[expect(unsafe_code, reason = "writes each repr its closure returns to the cell")]
    #[inline]
    unsafe fn try_update_repr<
        S: RmwOrdering,
        F: LoadOrdering,
        U: FnMut(T::Repr) -> Option<T::Repr>,
    >(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> Result<T::Repr, T::Repr> {
        let _ = (set_order, fetch_order);
        let cell = self.primitive_cell();
        let mut seen = T::Repr::read_for_rmw(cell, F::CORE);
        loop {
            let next = f(seen).ok_or(seen)?;
            match T::Repr::compare_exchange_weak(cell, seen, next, S::CORE, F::CORE) {
                Ok(before) => return Ok(before),
                Err(found) => seen = found,
            }
        }
    }

    /// Reads the value with a compare-exchange, for a repr without [`Load`]: a 128-bit integer's,
    /// or a `DoubleWord`'s, where the target has no 16-byte load.
    ///
    /// The exchange takes the cache line exclusive and writes it, so it faults on read-only memory.
    /// A pointer, which has a [`Load`] everywhere, has no `load_rmw`: its exchange compares its
    /// address alone, so one of null for null could strip a null's provenance.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn load_rmw<O: LoadOrdering>(&self, order: O) -> T
    where
        T::Repr: ReadByExchange,
    {
        let repr = self.load_rmw_repr(order);
        // SAFETY: by the field INVARIANT, the repr read from the cell decodes.
        unsafe { T::from_repr_unchecked(repr) }
    }

    /// Reads the repr with a compare-exchange, which leaves it as it was: the read of
    /// [`load_rmw`](Self::load_rmw), and of each field's.
    #[inline]
    fn load_rmw_repr<O: LoadOrdering>(&self, order: O) -> T::Repr
    where
        T::Repr: ReadByExchange,
    {
        let _ = order;
        let zero = T::Repr::from_bits(0);
        // It writes zero only over zero, and an integer's exchange compares every bit, as a double
        // word's does of a cell that holds addresses alone, so the repr stays as it was, whether or
        // not zero decodes. A pointer's compares only the address, so over address zero it would
        // write a null without the pointer's provenance: hence `ReadByExchange`, which costs
        // nothing, as every pointer has a `Load`.
        match T::Repr::compare_exchange(self.primitive_cell(), zero, zero, O::CORE, O::CORE) {
            Ok(current) | Err(current) => current,
        }
    }

    /// Writes `value` with a compare-exchange loop, for a primitive without [`Store`].
    ///
    /// # Panics
    /// As [`store`](Self::store).
    #[inline]
    pub fn store_rmw<O: StoreOrdering>(&self, value: T, order: O) {
        // ORDERING: `order` on the exchange that lands, pairing as the caller's store would;
        // Relaxed on the reads before it, which only feed the comparison and pair with nothing.
        let _ = self.update(order, Relaxed, |_| value);
    }

    /// The repr's address, for interop.
    ///
    /// Every access through it while the atomic is shared is atomic and of the repr's width, and a
    /// write leaves a repr [`from_repr`](Atom::from_repr) decodes. A repr of two words, whose cell
    /// holds its pointers' addresses alone, lends none, as [`RawAccess`] says.
    #[cfg(not(loom))]
    #[inline]
    #[must_use]
    pub const fn as_ptr(&self) -> *mut T::Repr
    where
        T::Repr: [const] RawAccess,
    {
        T::Repr::as_ptr(self.primitive_cell())
    }

    /// The atomic `ptr` points to.
    ///
    /// A repr of two words, whose cell holds its pointers' addresses alone, has none, as
    /// [`RawAccess`] says.
    ///
    /// # Safety
    /// For all of `'a`: `ptr` is aligned to `align_of::<Atomic<T>>()` and valid for reads and
    /// writes; no non-atomic or different-width access reaches it without synchronization; and
    /// the pointee holds a repr [`from_repr`](Atom::from_repr) decodes (any repr, where
    /// `T::Validity` is [`Total`](crate::validity::Total)), which every write keeps.
    ///
    /// # Examples
    /// ```
    /// # extern crate atomix_core as atomix;
    /// use atomix::AtomicU64;
    /// use atomix::ordering::{Acquire, Release};
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
    pub const unsafe fn from_ptr<'a>(ptr: *mut T::Repr) -> &'a Self
    where
        T::Repr: RawAccess,
    {
        // SAFETY: `Atomic<T>` is `repr(transparent)` over `T::Validity`'s cell, which has the
        // primitive's cell's layout (the cell itself, or `Opaque`'s `repr(transparent)` over it),
        // which has the primitive's size and bit validity, by `RawAccess`'s contract; the caller
        // upholds `from_ptr`'s contract, which keeps the field INVARIANT.
        unsafe { &*ptr.cast::<Self>() }
    }
}

// `const`, as core's are, where the cells are.
#[cfg(not(loom))]
const impl<T: [const] Atom> From<T> for Atomic<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self {
            cell: T::Validity::wrap::<T::Repr>(value.to_repr().into_cell()),
            marker: PhantomData,
        }
    }
}

#[cfg(loom)]
impl<T: Atom> From<T> for Atomic<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self {
            cell: T::Validity::wrap::<T::Repr>(value.to_repr().into_cell()),
            marker: PhantomData,
        }
    }
}

// `const`, as `From` is, where the cells are.
#[cfg(not(loom))]
const impl<T: [const] Atom + [const] Default> Default for Atomic<T> {
    #[inline]
    fn default() -> Self {
        Self::from(T::default())
    }
}

#[cfg(loom)]
impl<T: Atom + Default> Default for Atomic<T> {
    #[inline]
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T: Atom + fmt::Pointer> fmt::Pointer for Atomic<T>
where
    T::Repr: Load,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ORDERING: Relaxed, as core's `AtomicPtr`'s: printing publishes nothing and pairs with no
        // store.
        fmt::Pointer::fmt(&self.load(Relaxed), f)
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

/// An atomic as a value: never true, so the one impl it bounds never applies, and rustc reports
/// its message for an atomic inside an atomic.
///
/// `Copy` is its supertrait, so the bound fails here rather than at `Atom`'s own `Copy`, and no
/// type implements it: an atomic is never `Copy`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is an atomic, a place a value lives in, not a value an atomic holds",
    label = "an atomic, not a value",
    note = "store its value instead, `Atomic<u64>` for `Atomic<AtomicU64>`; for a place inside a value, derive `Atom` for a struct of values, whose atomic lends each field as a place through `fields()`",
    note = "to keep several atomics together, put them side by side in a struct; to point at one, store a `NonNull` to it"
)]
pub impl(crate) trait NotAnAtomic: Copy {}

// It exists so that rustc reports `NotAnAtomic`'s message for an atomic inside an atomic, rather
// than `Atom`'s, which advises deriving `Atom`; hidden, since rustdoc would list it among `Atom`'s
// implementors.
//
// SAFETY: it never applies: its bound, `NotAnAtomic`, has no impl, and an `Atomic<T>` is not
// `Copy`, which `NotAnAtomic` needs.
#[expect(unsafe_code, reason = "an impl that never applies, for its diagnostic")]
#[doc(hidden)]
unsafe impl<T: Atom> Atom for Atomic<T>
where
    Self: NotAnAtomic,
{
    type Repr = T::Repr;
    const REPRS: ReprRange<T::Repr> = T::REPRS;
    #[inline]
    fn to_repr(self) -> T::Repr {
        T::Repr::from_cell(T::Validity::into_inner::<T::Repr>(self.cell))
    }
    #[inline]
    fn from_repr(_: T::Repr) -> Option<Self> {
        None
    }
}

// Under loom, `From` and `Default` are not `const`.
#[cfg(not(loom))]
#[cfg(test)]
mod tests {
    use crate::atomic::AtomicU64;

    #[test]
    fn from_is_const() {
        const {
            let mut value = AtomicU64::from(5);
            assert!(value.get() == 5, "`From` builds the atomic in const");
        }
    }

    #[test]
    fn default_is_const() {
        const {
            let mut value = AtomicU64::default();
            assert!(
                value.get() == 0,
                "`Default` builds the atomic of the value's default in const"
            );
        }
    }
}
