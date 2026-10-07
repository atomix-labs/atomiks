//! [`DoubleWord`]: two words in one 16-byte atomic, each a pointer or an integer.
//!
//! No Rust operation keeps a provenance through a 16-byte atomic: core's intrinsics take an integer
//! or one pointer, and an `asm!` block means what Rust code doing the same would, which for 16
//! bytes is an integer atomic. So the cell is `u128`'s, each pointer going in exposed and each
//! coming out taking back an exposed provenance, through core's own 128-bit instructions: `casp` or
//! `cmpxchg16b`, and LSE2's `ldp` and `stp` or AVX's `vmovdqa`.
//!
//! Miri runs a model of its own instead, a lock around plain copies of the words: each pointer
//! keeps its own provenance through the cell, as `DoubleWord`'s docs say, and every operation
//! synchronizes with the one before, so loom, which models the hardware's cell, checks the
//! orderings.

use core::intrinsics::const_eval_select;
use core::marker::Destruct;
use core::mem::transmute;
use core::ptr;

use super::Primitive;
use crate::range::mask;

/// How many bits the first word takes, at the low end of a double word's bits.
const WORD_BITS: u32 = u64::BITS;

/// A word of a [`DoubleWord`]: a `*mut T`, a `usize`, or a trait object's [`VtablePointer`].
///
/// Its bits are an integer's, or a pointer's address; a pointer made of bits has no provenance,
/// and a vtable pointer none but null. It is unnameable outside atomiks, so a double word's cell
/// alone takes a pointer back from its exposed bits, and no safe code forges a vtable.
///
/// [`VtablePointer`]: crate::VtablePointer
// `Unpin` and `const Destruct`, so Miri's cell, which holds the words, is a primitive's cell.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a word of a double word",
    label = "expected a thin pointer, a `usize` or a vtable pointer",
    note = "the derive reads a pointer to a slice, a `str` or a trait object, two words, from the field's type as written: a pointer through an alias, or to a type whose tail is a slice, it takes for one word"
)]
pub impl(crate) const trait Word: Copy + Unpin + const Destruct {
    /// Whether `is_bits` decides every `bits`: an integer's, whose bits are its whole value.
    const IS_BITS_EXACT: bool;
    /// The word of the low 64 bits of `bits`.
    fn from_bits(bits: u128) -> Self;
    /// Whether the word's bits are `bits`: an integer's exactly, a pointer's only as null, since a
    /// constant reads no other pointer's address.
    fn is_bits(self, bits: u128) -> bool;
    /// The word's bits: an integer's, or a pointer's address, which a constant reads of null
    /// alone.
    fn packed_bits(self) -> u128;
    /// The word whose bits are the low 64 of `bits`: the integer of them, or this pointer at that
    /// address, its provenance kept.
    #[must_use]
    fn with_packed_bits(self, bits: u128) -> Self;
    /// The word with the bits set in `mask` cleared.
    #[must_use]
    fn clear_packed_bits(self, mask: u128) -> Self;
    /// The word's bits, a pointer's address with its provenance exposed; in a constant, only a
    /// null pointer's, as no constant exposes a provenance.
    fn exposed_bits(self) -> u64;
    /// The word whose bits are `bits`, a pointer taking back a provenance `exposed_bits` exposed.
    fn from_exposed_bits(bits: u64) -> Self;
}

/// Implements `Word` for a primitive, its bits as its own `Primitive` impl reads and writes them.
macro_rules! primitive_word {
    (
        [$($param:ident)?] $word:ty,
        $word_value:ident => $exposed_bits:expr,
        $bits:ident => $from_exposed_bits:expr $(,)?
    ) => {
        const impl$(<$param>)? Word for $word {
            const IS_BITS_EXACT: bool = <Self as Primitive>::IS_BITS_EXACT;
            #[inline]
            fn from_bits(bits: u128) -> Self {
                <Self as Primitive>::from_bits(bits)
            }
            #[inline]
            fn is_bits(self, bits: u128) -> bool {
                <Self as Primitive>::is_bits(self, bits)
            }
            #[inline]
            fn packed_bits(self) -> u128 {
                <Self as Primitive>::packed_bits(self)
            }
            #[inline]
            fn with_packed_bits(self, bits: u128) -> Self {
                <Self as Primitive>::with_packed_bits(self, bits)
            }
            #[inline]
            fn clear_packed_bits(self, mask: u128) -> Self {
                <Self as Primitive>::clear_packed_bits(self, mask)
            }
            #[inline]
            fn exposed_bits(self) -> u64 {
                let $word_value = self;
                $exposed_bits
            }
            #[inline]
            fn from_exposed_bits(bits: u64) -> Self {
                let $bits = bits;
                $from_exposed_bits
            }
        }
    };
}

primitive_word!(
    [T] *mut T,
    pointer => exposed_address(pointer).wrapping_cast(),
    bits => ptr::with_exposed_provenance_mut(bits.wrapping_cast()),
);
primitive_word!([] usize, integer => integer.wrapping_cast(), bits => bits.wrapping_cast());

/// A pointer's address with its provenance exposed, which a constant reads of a pointer made of an
/// integer alone.
///
/// No constant exposes a provenance, so a `static` holds a double word whose pointers have none:
/// null, or a null with tags set. It is one of atomiks' four branches on whether it runs at compile
/// time, beside a pointer word's [`address`](super::address), which keeps its provenance.
#[inline]
#[must_use]
pub(crate) const fn exposed_address<T>(pointer: *mut T) -> usize {
    const_eval_select(
        (pointer,),
        exposed_address_in_constant::<T>,
        exposed_address_at_run_time::<T>,
    )
}

/// A pointer's address in a constant, where it was made of an integer, which has no provenance to
/// expose; a pointer with provenance has no address a constant knows, and fails the build with
/// "unable to turn pointer into integer" where its address is used.
#[expect(unsafe_code, reason = "reads the address of a pointer made of an integer")]
#[expect(
    ptr_to_integer_transmute_in_consts,
    reason = "a pointer with provenance fails the build where its address is used, as the lint warns"
)]
const fn exposed_address_in_constant<T>(pointer: *mut T) -> usize {
    // SAFETY: a constant may transmute a pointer made of an integer to that integer, its address.
    // One with provenance gives a value whose use as an integer, the double word's bits, stops the
    // constant's evaluation, as `transmute` documents, so the build fails and nothing runs.
    unsafe { transmute::<*mut T, usize>(pointer) }
}

/// A pointer's address at run time, its provenance exposed.
#[inline]
fn exposed_address_at_run_time<T>(pointer: *mut T) -> usize {
    pointer.expose_provenance()
}

/// Two words in one 16-byte atomic, `first` at the lower address: two pointers, a pointer and an
/// integer, or a wide pointer's data pointer and metadata.
///
/// The repr of a pair of pointers, of a pointer to a slice, a `str` or a trait object, and of a
/// struct `#[derive(Atom)]` lays out in two words. Its operations are `u128`'s: compare-exchange
/// wherever the target has a 16-byte one, and a load and a store where it has those, as [`Load`]
/// and [`Store`] say; no exchange, add or bitwise operation, which no target at atomiks' floors
/// runs without a loop.
///
/// Rust has no 16-byte atomic that keeps a provenance ([UCG #517]), so the cell exposes each
/// pointer stored, as [`expose_provenance`] does, and each pointer loaded takes back an exposed
/// provenance, as [`with_exposed_provenance_mut`](ptr::with_exposed_provenance_mut) does, which
/// costs no instruction. It is the one place atomiks exposes a provenance: a pointer word or a
/// pointer enum, one word, never does. So:
///
/// - **The cell lends no place**, as [`RawAccess`] says: it holds the pointers' addresses, which
///   hold no provenance, so an atomic of two words has no `as_ptr`, `from_ptr` or `get_mut`.
/// - **No constant exposes a provenance**, so a `static` starts with pointers that have none, null
///   or a null with tags set, and takes its pointers at run time.
/// - **A trait object's compare-exchange compares its vtable pointer** beside its data pointer, and
///   one type may have several vtables, as [`ptr::eq`] warns, so it takes `current` from a load or
///   a failed exchange, as `update` does, never from a pointer coerced afresh.
/// - **Under Miri, the cell is a lock around plain copies of the words**, so each pointer keeps its
///   own provenance, and a run under `-Zmiri-strict-provenance` reports a pointer read back and
///   used once its allocation is freed, even where another allocation takes its address, which an
///   exposed provenance taken back would let pass. The lock orders every operation after the one
///   before, whatever its ordering, so it lets pass a race the orderings would allow: loom, which
///   models the hardware's cell, checks those.
///
/// # Examples
/// ```
/// # extern crate atomiks_core as atomiks;
/// use core::ptr::NonNull;
///
/// use atomiks::ordering::Acquire;
/// use atomiks::{Atom, Atomic, DoubleWord};
///
/// /// An order resting on the book.
/// struct Order {
///     quantity: u64,
/// }
///
/// let (bid, ask) = (Order { quantity: 300 }, Order { quantity: 200 });
/// let (bid, ask) = (NonNull::from_ref(&bid), NonNull::from_ref(&ask));
/// // The best bid and ask, in one atomic, so one compare-exchange changes both.
/// let best = Atomic::new((bid, ask));
/// let repr: DoubleWord<*mut Order, *mut Order> = best.load_rmw(Acquire).to_repr();
/// assert_eq!(repr.first, bid.as_ptr(), "the bid's pointer, in the first word");
/// ```
///
/// [`Load`]: crate::Load
/// [`Store`]: crate::Store
/// [`RawAccess`]: crate::RawAccess
/// [UCG #517]: https://github.com/rust-lang/unsafe-code-guidelines/issues/517
/// [`expose_provenance`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.expose_provenance
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DoubleWord<A, B> {
    /// The word at the lower address.
    pub first: A,
    /// The word at the higher address.
    pub second: B,
}

// A word's bits are its `Word` impl's.
const impl<A: [const] Word, B: [const] Word> Primitive for DoubleWord<A, B> {
    const BITS: u32 = u128::BITS;
    const IS_BITS_EXACT: bool = A::IS_BITS_EXACT && B::IS_BITS_EXACT;
    #[inline]
    fn from_bits(bits: u128) -> Self {
        Self { first: A::from_bits(bits), second: B::from_bits(bits.unbounded_shr(WORD_BITS)) }
    }
    #[inline]
    fn is_bits(self, bits: u128) -> bool {
        self.first.is_bits(bits & mask(WORD_BITS))
            && self.second.is_bits(bits.unbounded_shr(WORD_BITS))
    }
    #[inline]
    fn packed_bits(self) -> u128 {
        self.first.packed_bits() | self.second.packed_bits().unbounded_shl(WORD_BITS)
    }
    #[inline]
    fn with_packed_bits(self, bits: u128) -> Self {
        Self {
            first: self.first.with_packed_bits(bits & mask(WORD_BITS)),
            second: self.second.with_packed_bits(bits.unbounded_shr(WORD_BITS)),
        }
    }
    #[inline]
    fn clear_packed_bits(self, mask_bits: u128) -> Self {
        Self {
            first: self.first.clear_packed_bits(mask_bits & mask(WORD_BITS)),
            second: self.second.clear_packed_bits(mask_bits.unbounded_shr(WORD_BITS)),
        }
    }
}

#[cfg(not(double_word_lock))]
mod exposed {
    //! The hardware's cell, and loom's model of it: `u128`'s, each pointer's provenance exposed.

    use core::fmt;
    use core::marker::PhantomData;
    use core::sync::atomic::Ordering as CoreOrdering;

    use super::{DoubleWord, WORD_BITS, Word};
    use crate::primitive::{CellAccess, CompareExchange};
    #[cfg(wide_load_store)]
    use crate::primitive::{Load, Store};

    /// The words' types, as a cell of them holds them: invariantly, and `Send` and `Sync` whatever
    /// they are, as `Atom` declares a value may cross threads.
    type Invariant<A, B> = PhantomData<fn(A, B) -> (A, B)>;

    /// A 16-byte atomic cell holding a [`DoubleWord`]'s bits as `u128`'s cell holds its own.
    #[repr(transparent)]
    pub struct DoubleCell<A, B> {
        /// The words' bits, `first`'s low, each pointer's provenance exposed.
        bits: <u128 as CellAccess>::Cell,
        /// The words' types.
        marker: Invariant<A, B>,
    }

    impl<A, B> fmt::Debug for DoubleCell<A, B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("DoubleCell").finish_non_exhaustive()
        }
    }

    /// The words' bits, `first`'s low, each pointer's provenance exposed.
    #[inline]
    const fn exposed_bits<A: [const] Word, B: [const] Word>(words: DoubleWord<A, B>) -> u128 {
        u128::from(words.first.exposed_bits())
            | u128::from(words.second.exposed_bits()).unbounded_shl(WORD_BITS)
    }

    /// The words of `exposed_bits`'s bits, each pointer taking back an exposed provenance.
    #[inline]
    const fn from_exposed_bits<A: [const] Word, B: [const] Word>(bits: u128) -> DoubleWord<A, B> {
        DoubleWord {
            first: A::from_exposed_bits(bits.wrapping_cast()),
            second: B::from_exposed_bits(bits.unbounded_shr(WORD_BITS).wrapping_cast()),
        }
    }

    /// The words a weak exchange of `exposed_bits`'s bits found, `Ok` where it wrote: decoded
    /// once, whichever the result, as the strong exchange decodes them. The result says which, as
    /// no comparison can: a weak exchange may fail where it finds `current`.
    #[inline]
    const fn from_exposed_result<A: [const] Word, B: [const] Word>(
        result: Result<u128, u128>,
    ) -> Result<DoubleWord<A, B>, DoubleWord<A, B>> {
        let (Ok(bits) | Err(bits)) = result;
        let words = from_exposed_bits(bits);
        if result.is_ok() { Ok(words) } else { Err(words) }
    }

    /// Implements `CellAccess` over `u128`'s cell, `const` where `u128`'s is: off loom.
    macro_rules! cell_access {
        ($($constness:ident)?) => {
            $($constness)? impl<A: $([$constness])? Word, B: $([$constness])? Word> CellAccess
                for DoubleWord<A, B>
            {
                type Cell = DoubleCell<A, B>;
                #[inline]
                fn into_cell(self) -> DoubleCell<A, B> {
                    DoubleCell { bits: exposed_bits(self).into_cell(), marker: PhantomData }
                }
                #[inline]
                fn from_cell(cell: DoubleCell<A, B>) -> Self {
                    from_exposed_bits(u128::from_cell(cell.bits))
                }
                #[inline]
                fn get(cell: &mut DoubleCell<A, B>) -> Self {
                    from_exposed_bits(u128::get(&mut cell.bits))
                }
                #[inline]
                fn set(cell: &mut DoubleCell<A, B>, value: Self) {
                    u128::set(&mut cell.bits, exposed_bits(value));
                }
            }
        };
    }

    #[cfg(not(loom))]
    cell_access!(const);
    #[cfg(loom)]
    cell_access!();

    impl<A: Word, B: Word> CompareExchange for DoubleWord<A, B> {
        #[inline]
        fn read_for_rmw(cell: &DoubleCell<A, B>, order: CoreOrdering) -> Self {
            from_exposed_bits(u128::read_for_rmw(&cell.bits, order))
        }
        #[inline]
        fn compare_exchange(
            cell: &DoubleCell<A, B>, current: Self, new: Self, success: CoreOrdering,
            failure: CoreOrdering,
        ) -> Result<Self, Self> {
            let (current, new) = (exposed_bits(current), exposed_bits(new));
            // The bits found are decoded once, whichever the result: decoded in each arm, LLVM
            // writes `current` for them in the `Ok` arm, and joins the two decodes with a branch or
            // a `cmov` that `u128`'s exchange has no need of. A strong exchange writes exactly
            // where it finds `current`, so the comparison of the bits says which arm.
            let (Ok(bits) | Err(bits)) =
                u128::compare_exchange(&cell.bits, current, new, success, failure);
            let words = from_exposed_bits(bits);
            if bits == current { Ok(words) } else { Err(words) }
        }
        #[inline]
        fn compare_exchange_weak(
            cell: &DoubleCell<A, B>, current: Self, new: Self, success: CoreOrdering,
            failure: CoreOrdering,
        ) -> Result<Self, Self> {
            let (current, new) = (exposed_bits(current), exposed_bits(new));
            from_exposed_result(u128::compare_exchange_weak(
                &cell.bits, current, new, success, failure,
            ))
        }
    }

    #[cfg(wide_load_store)]
    impl<A: Word, B: Word> Load for DoubleWord<A, B> {
        #[inline]
        fn load(cell: &DoubleCell<A, B>, order: CoreOrdering) -> Self {
            from_exposed_bits(u128::load(&cell.bits, order))
        }
    }

    #[cfg(wide_load_store)]
    impl<A: Word, B: Word> Store for DoubleWord<A, B> {
        #[inline]
        fn store(cell: &DoubleCell<A, B>, value: Self, order: CoreOrdering) {
            u128::store(&cell.bits, exposed_bits(value), order);
        }
    }
}

#[cfg(double_word_lock)]
mod lock {
    //! Miri's model: plain copies of the words under one process-wide lock, so each pointer keeps
    //! its own provenance, as a 16-byte atomic that kept it, which Rust lacks, would keep it.
    //!
    //! The lock is taken with Acquire and given back with Release, so every operation on a double
    //! word synchronizes with the one before it, whatever its ordering: Miri checks each pointer's
    //! provenance and what it reaches, and loom's model the orderings.

    use core::cell::UnsafeCell;
    use core::panic::RefUnwindSafe;
    use core::sync::atomic::{AtomicBool, Ordering as CoreOrdering};
    use core::{fmt, hint};

    use super::{DoubleWord, Word};
    use crate::primitive::{CellAccess, CompareExchange, Primitive};
    #[cfg(wide_load_store)]
    use crate::primitive::{Load, Store};

    /// A 16-byte cell holding a [`DoubleWord`] as plain words, aligned as the hardware's.
    #[repr(C, align(16))]
    pub struct DoubleCell<A, B> {
        // INVARIANT: while shared, every access is a critical section of `with_lock`'s, whose
        // callers are this module's `CompareExchange`, `Load` and `Store`: a double word lends no
        // place, having no `RawAccess`, so no `as_ptr`, `from_ptr` or `get_mut` reaches it.
        /// The words.
        words: UnsafeCell<DoubleWord<A, B>>,
    }

    // SAFETY: by the field INVARIANT, every shared access to `words` holds the lock.
    #[expect(unsafe_code, reason = "a cell every shared access to which holds a lock")]
    unsafe impl<A, B> Sync for DoubleCell<A, B> {}

    // SAFETY: moving the cell moves only its words, which `Atom` declares may cross threads.
    #[expect(unsafe_code, reason = "a cell of words `Atom` declares may cross threads")]
    #[expect(
        clippy::non_send_fields_in_send_ty,
        reason = "its pointers may cross threads, as `Atom` declares"
    )]
    unsafe impl<A, B> Send for DoubleCell<A, B> {}

    // As core's atomics: every shared access holds the lock, so a panic leaves no torn value.
    impl<A, B> RefUnwindSafe for DoubleCell<A, B> {}

    impl<A, B> fmt::Debug for DoubleCell<A, B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("DoubleCell").finish_non_exhaustive()
        }
    }

    /// The lock, held while an operation reads or writes a double word's cell.
    static LOCK: AtomicBool = AtomicBool::new(false);

    /// Runs `operation` on `cell`'s words while holding the lock.
    #[expect(unsafe_code, reason = "reads and writes a cell's words under the lock")]
    fn with_lock<A, B, R>(
        cell: &DoubleCell<A, B>, operation: impl FnOnce(&mut DoubleWord<A, B>) -> R,
    ) -> R {
        // ORDERING: Acquire, pairing with the Release that gives the lock back, so each critical
        // section sees the words the one before it left.
        while LOCK
            .compare_exchange_weak(false, true, CoreOrdering::Acquire, CoreOrdering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }
        // SAFETY: the lock excludes every other shared access, by the field INVARIANT, for as long
        // as the reference lives, which ends before the lock is given back.
        let result = operation(unsafe { &mut *cell.words.get() });
        // ORDERING: Release, pairing with the Acquire that takes the lock next.
        LOCK.store(false, CoreOrdering::Release);
        result
    }

    const impl<A: [const] Word, B: [const] Word> CellAccess for DoubleWord<A, B> {
        type Cell = DoubleCell<A, B>;
        #[inline]
        fn into_cell(self) -> DoubleCell<A, B> {
            // As the hardware's cell, a constant builds one of null pointers alone.
            let _ = (self.first.exposed_bits(), self.second.exposed_bits());
            DoubleCell { words: UnsafeCell::new(self) }
        }
        #[inline]
        fn from_cell(cell: DoubleCell<A, B>) -> Self {
            cell.words.into_inner()
        }
        #[inline]
        fn get(cell: &mut DoubleCell<A, B>) -> Self {
            *cell.words.get_mut()
        }
        #[inline]
        fn set(cell: &mut DoubleCell<A, B>, value: Self) {
            *cell.words.get_mut() = value;
        }
    }

    // A pointer's address alone is compared, as the hardware's exchange compares it.
    impl<A: Word, B: Word> CompareExchange for DoubleWord<A, B> {
        fn read_for_rmw(cell: &DoubleCell<A, B>, _: CoreOrdering) -> Self {
            with_lock(cell, |words| *words)
        }
        fn compare_exchange(
            cell: &DoubleCell<A, B>, current: Self, new: Self, _: CoreOrdering, _: CoreOrdering,
        ) -> Result<Self, Self> {
            with_lock(cell, |words| {
                let seen = *words;
                if seen.packed_bits() != current.packed_bits() {
                    return Err(seen);
                }
                *words = new;
                Ok(seen)
            })
        }
        fn compare_exchange_weak(
            cell: &DoubleCell<A, B>, current: Self, new: Self, success: CoreOrdering,
            failure: CoreOrdering,
        ) -> Result<Self, Self> {
            Self::compare_exchange(cell, current, new, success, failure)
        }
    }

    #[cfg(wide_load_store)]
    impl<A: Word, B: Word> Load for DoubleWord<A, B> {
        fn load(cell: &DoubleCell<A, B>, _: CoreOrdering) -> Self {
            with_lock(cell, |words| *words)
        }
    }

    #[cfg(wide_load_store)]
    impl<A: Word, B: Word> Store for DoubleWord<A, B> {
        fn store(cell: &DoubleCell<A, B>, value: Self, _: CoreOrdering) {
            with_lock(cell, |words| *words = value);
        }
    }
}

// The impls stay `const`: each line evaluates at compile time.
#[cfg(not(loom))]
const _: () = {
    let null = DoubleWord { first: ptr::null_mut::<u8>(), second: 7_usize };
    let seven_above = 7_u128.unbounded_shl(WORD_BITS);
    assert!(null.packed_bits() == seven_above, "the second word's bits above the first's");
    assert!(null.is_bits(seven_above), "and read back");
    assert!(!null.is_bits(7), "never at the low end");
    assert!(null.first.exposed_bits() == 0, "a constant exposes a null pointer's address");
};

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::{DoubleWord, Word};
    use crate::primitive::Primitive;

    #[test]
    fn the_first_word_takes_the_low_bits_and_the_second_the_high() {
        let words = DoubleWord::<usize, usize>::from_bits(3 | 5 << 64);
        assert_eq!(words, DoubleWord { first: 3, second: 5 }, "split at bit 64");
        assert_eq!(words.packed_bits(), 3 | 5 << 64, "and joined again");
        const {
            assert!(<DoubleWord<usize, usize> as Primitive>::IS_BITS_EXACT, "two integers' bits");
            assert!(!<DoubleWord<*mut u8, usize> as Primitive>::IS_BITS_EXACT, "not a pointer's");
        }
    }

    #[test]
    fn a_double_words_packed_bits_change_each_pointer_and_keep_its_provenance() {
        let mut values = [7_u64, 11];
        let [first, second] = values.each_mut().map(ptr::from_mut);
        let words = DoubleWord { first, second };
        let bits = words.packed_bits();
        let tagged = words.with_packed_bits(bits | 1 | 1 << 64);
        assert_eq!(tagged.first.addr(), first.addr() | 1, "the first pointer's bit 0 set");
        assert_eq!(tagged.second.addr(), second.addr() | 1, "and the second's");
        let cleared = tagged.clear_packed_bits(1 | 1 << 64);
        assert_eq!(cleared, words, "both cleared again");
        for (pointer, value) in [(cleared.first, 7), (cleared.second, 11)] {
            // SAFETY: the pointer is one of `values`', offset and cleared, so it keeps its
            // provenance over its value, which is live.
            #[expect(unsafe_code, reason = "reads through it, so Miri checks its provenance")]
            let read = unsafe { pointer.read() };
            assert_eq!(read, value, "each reads its value");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "strict provenance refuses a provenance taken back from exposure")]
    fn a_word_round_trips_a_pointers_address_and_an_integers_bits() {
        let mut value = 7_u64;
        let pointer = ptr::from_mut(&mut value);
        assert_eq!(<*mut u64>::from_exposed_bits(pointer.exposed_bits()), pointer, "the address");
        assert_eq!(usize::from_exposed_bits(9_usize.exposed_bits()), 9, "an integer's bits");
    }
}
