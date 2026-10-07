//! `bool`, the integers up to 64 bits and `*mut T`, over `core`'s atomics (loom's under loom).

use core::hint::assert_unchecked;
use core::intrinsics::const_eval_select;
use core::ptr;
#[cfg(not(loom))]
use core::sync::atomic;
use core::sync::atomic::Ordering as CoreOrdering;

#[cfg(loom)]
use loom::sync::atomic;

use super::{
    BitTest, Bitwise, CellAccess, CompareExchange, ExactBits, FetchAdd, Load, MaskBitwise,
    Primitive, PtrOffset, Store, Swap,
};
#[cfg(target_arch = "aarch64")]
use super::{FetchBitwise, MinMax};
use crate::message::{Message, refuse};

/// Exclusive access to a loom cell: `with_mut`, which loom checks against every other access, or,
/// for a cell without it (`relaxed`), a Relaxed load or store, which `&mut` keeps from racing.
#[cfg(loom)]
macro_rules! loom_exclusive {
    (get $cell:ident) => {
        $cell.with_mut(|value| *value)
    };
    (set $cell:ident $value:ident) => {
        $cell.with_mut(|place| *place = $value)
    };
    (get $cell:ident relaxed) => {
        // ORDERING: Relaxed; `&mut` access excludes every other thread.
        $cell.load(CoreOrdering::Relaxed)
    };
    (set $cell:ident $value:ident relaxed) => {
        // ORDERING: Relaxed; `&mut` access excludes every other thread.
        $cell.store($value, CoreOrdering::Relaxed)
    };
}

/// Implements `CellAccess`, `CompareExchange`, `Load`, `Store` and `Swap` for a primitive.
///
/// Its loom cell has the same name as its core one; `relaxed` marks a loom cell without `with_mut`.
macro_rules! cells {
    (@ [$($param:ident)?] $kind:ty => $cell:ty $(, $relaxed:ident)?) => {
        #[cfg(not(loom))]
        const impl$(<$param>)? CellAccess for $kind {
            type Cell = $cell;
            #[inline]
            fn into_cell(self) -> Self::Cell {
                <$cell>::new(self)
            }
            #[inline]
            fn from_cell(cell: Self::Cell) -> Self {
                cell.into_inner()
            }
            #[inline]
            fn get(cell: &mut Self::Cell) -> Self {
                *cell.get_mut()
            }
            #[inline]
            fn set(cell: &mut Self::Cell, value: Self) {
                *cell.get_mut() = value;
            }
            #[inline]
            fn get_mut(cell: &mut Self::Cell) -> &mut Self {
                cell.get_mut()
            }
            #[inline]
            fn as_ptr(cell: &Self::Cell) -> *mut Self {
                cell.as_ptr()
            }
        }
        #[cfg(loom)]
        impl$(<$param>)? CellAccess for $kind {
            type Cell = $cell;
            #[inline]
            fn into_cell(self) -> Self::Cell {
                <$cell>::new(self)
            }
            #[inline]
            fn from_cell(cell: Self::Cell) -> Self {
                cell.into_inner()
            }
            #[inline]
            fn get(cell: &mut Self::Cell) -> Self {
                loom_exclusive!(get cell $($relaxed)?)
            }
            #[inline]
            fn set(cell: &mut Self::Cell, value: Self) {
                loom_exclusive!(set cell value $($relaxed)?);
            }
        }
        impl$(<$param>)? CompareExchange for $kind {
            #[inline]
            fn read_for_rmw(cell: &Self::Cell, order: CoreOrdering) -> Self {
                cell.load(order)
            }
            #[inline]
            fn compare_exchange(
                cell: &Self::Cell, current: Self, new: Self, success: CoreOrdering,
                failure: CoreOrdering,
            ) -> Result<Self, Self> {
                cell.compare_exchange(current, new, success, failure)
            }
            #[inline]
            fn compare_exchange_weak(
                cell: &Self::Cell, current: Self, new: Self, success: CoreOrdering,
                failure: CoreOrdering,
            ) -> Result<Self, Self> {
                cell.compare_exchange_weak(current, new, success, failure)
            }
        }
        impl$(<$param>)? Load for $kind {
            #[inline]
            fn load(cell: &Self::Cell, order: CoreOrdering) -> Self {
                cell.load(order)
            }
        }
        impl$(<$param>)? Store for $kind {
            #[inline]
            fn store(cell: &Self::Cell, value: Self, order: CoreOrdering) {
                cell.store(value, order);
            }
        }
        impl$(<$param>)? Swap for $kind {
            #[inline]
            fn swap(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self {
                cell.swap(value, order)
            }
        }
    };
    (<$param:ident> $kind:ty => $cell:ty) => {
        cells!(@ [$param] $kind => $cell);
    };
    ($kind:ty => $cell:ty, relaxed) => {
        cells!(@ [] $kind => $cell, relaxed);
    };
    ($kind:ty => $cell:ty) => {
        cells!(@ [] $kind => $cell);
    };
}

/// Implements `Bitwise` and `MaskBitwise` for a primitive whose every bit set is `$ones`, and whose
/// bit at a position is `$bit` of it.
///
/// Also `FetchBitwise` on `aarch64`, whose `ldclr`, `ldset` and `ldeor` return the value before.
macro_rules! bitwise {
    ($($kind:ty: $ones:expr, $bit:expr);+ $(;)?) => {$(
        impl MaskBitwise for $kind {
            type Mask = Self;
            #[inline]
            fn bit(position: u32) -> Self {
                $bit(position)
            }
            #[inline]
            fn fetch_or_mask(cell: &Self::Cell, mask: Self, order: CoreOrdering) -> Self {
                cell.fetch_or(mask, order)
            }
            #[inline]
            fn fetch_and_mask(cell: &Self::Cell, mask: Self, order: CoreOrdering) -> Self {
                cell.fetch_and(mask, order)
            }
            #[inline]
            fn fetch_xor_mask(cell: &Self::Cell, mask: Self, order: CoreOrdering) -> Self {
                cell.fetch_xor(mask, order)
            }
        }
        impl Bitwise for $kind {
            #[inline]
            fn fetch_and(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self {
                cell.fetch_and(value, order)
            }
            #[inline]
            fn fetch_or(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self {
                cell.fetch_or(value, order)
            }
            #[inline]
            fn fetch_xor(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self {
                cell.fetch_xor(value, order)
            }
            #[inline]
            fn fetch_not(cell: &Self::Cell, order: CoreOrdering) -> Self {
                cell.fetch_xor($ones, order)
            }
        }
        // `atomic/capability.rs` and `atomic/field/mod.rs` repeat this cfg in the `doc(cfg(...))`
        // of `fetch_and`, `fetch_or`, `fetch_xor` and `fetch_not`: change them with it, and the
        // pointer's below.
        #[cfg(target_arch = "aarch64")]
        impl FetchBitwise for $kind {}
    )+};
}

/// Implements `BitTest` for each primitive whose bit test-and-set is one instruction.
macro_rules! bit_test {
    ($($kind:ty),+ $(,)?) => {$(
        impl BitTest for $kind {}
    )+};
}

// `x86_64`'s `lock bts`, `btr` and `btc` take 16 bits or more.
bit_test!(u16, u32, u64, usize, i16, i32, i64, isize);
#[cfg(target_arch = "aarch64")]
bit_test!(u8, i8, bool);

/// The unsigned bits of an integer: itself, or its two's complement.
macro_rules! unsigned_bits {
    ($value:expr) => {
        $value
    };
    ($value:expr,signed) => {
        $value.cast_unsigned()
    };
}

/// Implements every trait for an integer, `signed` where it is one.
macro_rules! integers {
    ($($int:ident => $cell:ident $(, $signed:ident)?);+ $(;)?) => {$(
        cells!($int => atomic::$cell);
        // Wrapping, as `x86_64`'s `bts` with its position in a register needs.
        bitwise!($int: !0, |position: u32| <$int>::wrapping_shl(1, position));
        const impl Primitive for $int {
            const BITS: u32 = <$int>::BITS;
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
                unsigned_bits!(self $(, $signed)?).wrapping_cast()
            }
        }
        impl FetchAdd for $int {
            #[inline]
            fn fetch_add(cell: &Self::Cell, delta: Self, order: CoreOrdering) -> Self {
                cell.fetch_add(delta, order)
            }
            #[inline]
            fn fetch_sub(cell: &Self::Cell, delta: Self, order: CoreOrdering) -> Self {
                cell.fetch_sub(delta, order)
            }
        }
        // `ldsmax`, `ldumin` and the rest (an LL/SC pair without LSE); `x86_64` has neither.
        // `atomic/capability.rs` repeats this cfg in the `doc(cfg(...))` of `fetch_max` and
        // `fetch_min`: change them with it.
        #[cfg(target_arch = "aarch64")]
        impl MinMax for $int {
            #[inline]
            fn fetch_max(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self {
                cell.fetch_max(value, order)
            }
            #[inline]
            fn fetch_min(cell: &Self::Cell, value: Self, order: CoreOrdering) -> Self {
                cell.fetch_min(value, order)
            }
        }
    )+};
}

integers! {
    u8 => AtomicU8;
    u16 => AtomicU16;
    u32 => AtomicU32;
    u64 => AtomicU64;
    usize => AtomicUsize;
    i8 => AtomicI8, signed;
    i16 => AtomicI16, signed;
    i32 => AtomicI32, signed;
    i64 => AtomicI64, signed;
    isize => AtomicIsize, signed;
}

// Loom's `AtomicBool` has no `with_mut`.
cells!(bool => atomic::AtomicBool, relaxed);
cells!(<T> *mut T => atomic::AtomicPtr<T>);

bitwise!(bool: true, |_: u32| true);

const impl Primitive for bool {
    const BITS: u32 = 1;
    #[inline]
    fn from_bits(bits: u128) -> Self {
        bits & 1 == 1
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

const impl ExactBits for bool {
    #[inline]
    fn to_bits(self) -> u128 {
        u128::from(self)
    }
}

const impl<T> Primitive for *mut T {
    const BITS: u32 = usize::BITS;
    const IS_BITS_EXACT: bool = false;
    #[inline]
    fn from_bits(bits: u128) -> Self {
        ptr::without_provenance_mut(bits.wrapping_cast())
    }
    #[inline]
    fn is_bits(self, bits: u128) -> bool {
        bits == 0 && self.is_null()
    }
    #[inline]
    fn packed_bits(self) -> u128 {
        address(self).wrapping_cast()
    }
    // The offset from its address to `bits`, as `with_addr`, which is not `const`, writes it.
    #[inline]
    fn with_packed_bits(self, bits: u128) -> Self {
        self.wrapping_byte_add(bits.wrapping_cast::<usize>().wrapping_sub(address(self)))
    }
    #[inline]
    fn clear_packed_bits(self, mask: u128) -> Self {
        clear_tags(self, mask.wrapping_cast())
    }
}

/// A pointer's address, which a constant reads of null alone.
///
/// No constant reads the address of a pointer with provenance, so one that reads any but null's
/// fails to build. It is one of atomiks' three branches on whether it runs at compile time: `addr`
/// is not `const`, and core's `is_null` branches so too. [`clear_tags`] and [`subtract_tags`] are
/// the others.
#[inline]
#[must_use]
pub(crate) const fn address<T>(pointer: *mut T) -> usize {
    const_eval_select((pointer,), address_in_constant::<T>, address_at_run_time::<T>)
}

/// A pointer's address in a constant: zero for null, else a refusal of the build.
#[track_caller]
const fn address_in_constant<T>(pointer: *mut T) -> usize {
    if pointer.is_null() {
        return 0;
    }
    refuse(&Message::new().text(concat!(
        "a constant reads no pointer's address but null's: build or read a pointer word or a ",
        "pointer enum whose pointer is not null at run time"
    )))
}

/// A pointer's address at run time.
#[inline]
fn address_at_run_time<T>(pointer: *mut T) -> usize {
    pointer.addr()
}

/// `pointer` with the bits of `tag_mask` cleared from its address, its provenance kept: the pointer
/// a pointer word holds, its tags taken off.
///
/// Both ways give the same pointer. At run time, `mask` clears the bits, which tells LLVM they are
/// clear, so an exchange that encodes the pointer again tests none of them, and a word over a word
/// tests none of either's: the bits an outer word masked off stay known clear under an inner
/// word's mask, where an offset by a tag no match fixes would lose them. A constant offsets the
/// pointer back by them, as `with_packed_bits` does, since `mask` is not `const`, and so reads no
/// pointer's address but null's, as [`address`] does.
#[inline]
#[must_use]
pub(crate) const fn clear_tags<T>(pointer: *mut T, tag_mask: usize) -> *mut T {
    const_eval_select((pointer, tag_mask), clear_tags_in_constant::<T>, clear_tags_at_run_time::<T>)
}

/// `pointer` with the bits of `tag_mask` cleared in a constant: offset back by them.
#[inline]
const fn clear_tags_in_constant<T>(pointer: *mut T, tag_mask: usize) -> *mut T {
    pointer.wrapping_byte_sub(address(pointer) & tag_mask)
}

/// `pointer` with the bits of `tag_mask` cleared at run time: masked off.
#[inline]
fn clear_tags_at_run_time<T>(pointer: *mut T, tag_mask: usize) -> *mut T {
    pointer.mask(!tag_mask)
}

/// `pointer` offset back by the bits of `tag_mask` its address has set, its provenance kept: the
/// pointer a pointer enum holds, its tag taken off.
///
/// It gives the pointer [`clear_tags`] does. At run time the offset is the tag, which each arm of
/// a match of the enum knows, so a load through the pointer takes the tag into its own offset,
/// where a mask would add an instruction before it; and LLVM is told the bits are clear, so an
/// update that encodes the pointer again tests none of them. A constant offsets the pointer back
/// as [`clear_tags`] does.
#[inline]
#[must_use]
pub(crate) const fn subtract_tags<T>(pointer: *mut T, tag_mask: usize) -> *mut T {
    const_eval_select(
        (pointer, tag_mask),
        clear_tags_in_constant::<T>,
        subtract_tags_at_run_time::<T>,
    )
}

/// `pointer` offset back by the bits of `tag_mask` at run time, and LLVM told those bits are clear.
#[inline]
fn subtract_tags_at_run_time<T>(pointer: *mut T, tag_mask: usize) -> *mut T {
    let tags = pointer.addr() & tag_mask;
    let cleared = pointer.wrapping_byte_sub(tags);
    // SAFETY: the condition holds for every pointer and mask. Each bit of `tags` is set in the
    // address too, so subtracting `tags` borrows from no bit and does not wrap: it clears those
    // bits, which leaves `address & !tag_mask`. `wrapping_byte_sub` moves the address by `tags`
    // exactly, so `cleared`'s address is that, which has no bit of `tag_mask` set.
    #[expect(unsafe_code, reason = "tells LLVM the bits the offset clears, which it cannot see")]
    unsafe {
        assert_unchecked(cleared.addr() & tag_mask == 0);
    }
    cleared
}

// Each operation keeps the pointer's provenance, as core's do.
impl<T> MaskBitwise for *mut T {
    type Mask = usize;
    #[inline]
    fn bit(position: u32) -> usize {
        1_usize.wrapping_shl(position)
    }
    #[cfg(not(loom))]
    #[inline]
    fn fetch_or_mask(cell: &Self::Cell, mask: usize, order: CoreOrdering) -> Self {
        cell.fetch_or(mask, order)
    }
    #[cfg(not(loom))]
    #[inline]
    fn fetch_and_mask(cell: &Self::Cell, mask: usize, order: CoreOrdering) -> Self {
        cell.fetch_and(mask, order)
    }
    #[cfg(not(loom))]
    #[inline]
    fn fetch_xor_mask(cell: &Self::Cell, mask: usize, order: CoreOrdering) -> Self {
        cell.fetch_xor(mask, order)
    }
    // Loom's pointer cell has no bitwise operation.
    #[cfg(loom)]
    #[inline]
    fn fetch_or_mask(cell: &Self::Cell, mask: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.map_addr(|address| address | mask))
    }
    #[cfg(loom)]
    #[inline]
    fn fetch_and_mask(cell: &Self::Cell, mask: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.map_addr(|address| address & mask))
    }
    #[cfg(loom)]
    #[inline]
    fn fetch_xor_mask(cell: &Self::Cell, mask: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.map_addr(|address| address ^ mask))
    }
}

// `lock bts`, `btr` and `btc` on the pointer's 64 bits, and `ldset`, `ldclr` and `ldeor`.
impl<T> BitTest for *mut T {}

// As the integers', whose cfg `atomic/field/mod.rs` repeats.
#[cfg(target_arch = "aarch64")]
impl<T> FetchBitwise for *mut T {}

#[cfg(not(loom))]
impl<T> PtrOffset for *mut T {
    #[inline]
    fn fetch_ptr_add(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self {
        cell.fetch_ptr_add(count, order)
    }
    #[inline]
    fn fetch_ptr_sub(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self {
        cell.fetch_ptr_sub(count, order)
    }
    #[inline]
    fn fetch_byte_add(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self {
        cell.fetch_byte_add(bytes, order)
    }
    #[inline]
    fn fetch_byte_sub(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self {
        cell.fetch_byte_sub(bytes, order)
    }
}

// Loom's pointer cell has no arithmetic.
#[cfg(loom)]
impl<T> PtrOffset for *mut T {
    #[inline]
    fn fetch_ptr_add(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.wrapping_add(count))
    }
    #[inline]
    fn fetch_ptr_sub(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.wrapping_sub(count))
    }
    #[inline]
    fn fetch_byte_add(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.wrapping_byte_add(bytes))
    }
    #[inline]
    fn fetch_byte_sub(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self {
        update_in_loop(cell, order, |seen| seen.wrapping_byte_sub(bytes))
    }
}

/// Applies `change` to the pointer in a compare-exchange loop, and returns the pointer before.
#[cfg(loom)]
fn update_in_loop<T, F: Fn(*mut T) -> *mut T>(
    cell: &atomic::AtomicPtr<T>, order: CoreOrdering, change: F,
) -> *mut T {
    // ORDERING: `order` on the exchange that lands, whose read is the pointer returned; Relaxed on
    // every other read, which is only compared.
    let mut seen = cell.load(CoreOrdering::Relaxed);
    loop {
        match cell.compare_exchange_weak(seen, change(seen), order, CoreOrdering::Relaxed) {
            Ok(before) => return before,
            Err(found) => seen = found,
        }
    }
}

// The impls stay `const`: each line evaluates at compile time.
#[cfg(not(loom))]
const _: () = {
    assert!((-1_i8).to_bits() == 0xFF, "an integer's bits are its two's complement");
    assert!(i8::from_bits(0xFF) == -1, "and back");
    assert!(u8::from_bits(0x1_FF) == 0xFF, "only the low bits count");
    assert!(u64::from_cell(7_u64.into_cell()) == 7, "a cell gives back its value");
    assert!(ptr::null_mut::<u8>().is_bits(0), "null is zero");
    assert!(address(ptr::null_mut::<u8>()) == 0, "a constant reads null's address");
    assert!(clear_tags(ptr::null_mut::<u8>(), 0b111).is_null(), "and clears null's tags");
    assert!(subtract_tags(ptr::null_mut::<u8>(), 0b111).is_null(), "or subtracts them");
};

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::{Primitive, address, clear_tags, clear_tags_in_constant, subtract_tags};

    #[test]
    fn bool_reads_only_its_low_bit() {
        assert!(bool::from_bits(0b01), "the low bit set is true");
        assert!(!bool::from_bits(0b10), "a higher bit alone is false");
    }

    #[test]
    fn a_pointer_matches_bits_only_when_null() {
        assert!(ptr::null_mut::<u8>().is_bits(0), "null is zero");
        assert!(
            !ptr::without_provenance_mut::<u8>(8).is_bits(8),
            "an address is unreadable in const, so it never matches"
        );
    }

    #[test]
    fn a_pointers_bits_are_its_address_and_a_write_keeps_its_provenance() {
        let mut value = 7_u64;
        let pointer = ptr::from_mut(&mut value);
        assert_eq!(address(pointer), pointer.addr(), "its address, at run time");
        let bits = u128::try_from(pointer.addr()).expect("an address fits 128 bits");
        assert_eq!(pointer.packed_bits(), bits, "as its bits");
        let tagged = pointer.with_packed_bits(bits | 1);
        assert_eq!(tagged.addr(), pointer.addr() | 1, "a low bit set");
        let untagged = tagged.with_packed_bits(bits);
        // SAFETY: `untagged` is `pointer` at its own address, so it keeps its provenance over
        // `value`, which is live.
        #[expect(unsafe_code, reason = "reads through the pointer, so Miri checks its provenance")]
        let read = unsafe { untagged.read() };
        assert_eq!(read, 7, "and back, it reads the value");
    }

    #[test]
    fn a_constant_clears_tags_to_the_pointer_run_time_does() {
        let mut value = 7_u64;
        let pointer = ptr::from_mut(&mut value);
        for tags in 0..8 {
            let tagged = pointer.wrapping_byte_add(tags);
            let at_run_time = clear_tags(tagged, 0b111);
            let in_constant = clear_tags_in_constant(tagged, 0b111);
            assert_eq!(at_run_time, pointer, "{tags:#05b}: the tags cleared at run time");
            assert_eq!(in_constant, at_run_time, "{tags:#05b}: and as a constant does");
            let as_primitive = tagged.clear_packed_bits(0b111);
            assert_eq!(as_primitive, at_run_time, "{tags:#05b}: and as a primitive does");
            let tag_alone = clear_tags(tagged, 0b100);
            let named = pointer.addr() | (tags & 0b011);
            assert_eq!(tag_alone.addr(), named, "{tags:#05b}: only the bits named");
            let in_constant_alone = clear_tags_in_constant(tagged, 0b100);
            assert_eq!(in_constant_alone, tag_alone, "{tags:#05b}: both ways");
            for cleared in [at_run_time, in_constant] {
                // SAFETY: `cleared` is `pointer`, offset by the tags and cleared of them, so it
                // keeps its provenance over `value`, which is live.
                #[expect(unsafe_code, reason = "reads through it, so Miri checks its provenance")]
                let read = unsafe { cleared.read() };
                assert_eq!(read, 7, "{tags:#05b}: each reads the value");
            }
        }
    }

    #[test]
    fn subtracting_tags_gives_the_pointer_clearing_them_does() {
        let mut value = 7_u64;
        let pointer = ptr::from_mut(&mut value);
        for tags in 0..8 {
            let tagged = pointer.wrapping_byte_add(tags);
            let subtracted = subtract_tags(tagged, 0b111);
            assert_eq!(subtracted, clear_tags(tagged, 0b111), "{tags:#05b}: every tag");
            let tag_alone = subtract_tags(tagged, 0b100);
            assert_eq!(tag_alone, clear_tags(tagged, 0b100), "{tags:#05b}: only the bits named");
            // SAFETY: `subtracted` is `pointer`, offset by the tags and back, so it keeps its
            // provenance over `value`, which is live.
            #[expect(unsafe_code, reason = "reads through it, so Miri checks its provenance")]
            let read = unsafe { subtracted.read() };
            assert_eq!(read, 7, "{tags:#05b}: it reads the value");
        }
    }
}
