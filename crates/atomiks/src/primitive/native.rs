//! `bool`, the integers up to 64 bits and `*mut T`, over `core`'s atomics (loom's under loom).

use core::ptr;
#[cfg(not(loom))]
use core::sync::atomic;
use core::sync::atomic::Ordering as CoreOrdering;

#[cfg(loom)]
use loom::sync::atomic;

use super::{
    Bitwise, CellAccess, CellOps, FetchAdd, Integer, Load, Primitive, PtrOffset, Store, Swap,
};
#[cfg(target_arch = "aarch64")]
use super::{FetchBitwise, MinMax};

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

/// Implements `CellAccess`, `CellOps`, `Load`, `Store` and `Swap` for a primitive.
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
        impl$(<$param>)? CellOps for $kind {
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

/// Implements `Bitwise` for a primitive whose every bit set is `$ones`.
///
/// Also `FetchBitwise` on `aarch64`, whose `ldclr`, `ldset` and `ldeor` return the value before.
macro_rules! bitwise {
    ($($kind:ty: $ones:expr);+ $(;)?) => {$(
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
        // `atomic/ops.rs` repeats this cfg in the `doc(cfg(...))` of `fetch_and`, `fetch_or`,
        // `fetch_xor` and `fetch_not`: change them with it.
        #[cfg(target_arch = "aarch64")]
        impl FetchBitwise for $kind {}
    )+};
}

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
        bitwise!($int: !0);
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
        }
        const impl Integer for $int {
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
        // `atomic/ops.rs` repeats this cfg in the `doc(cfg(...))` of `max`, `min`, `fetch_max` and
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

bitwise!(bool: true);

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
}

const impl Integer for bool {
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
}

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
        offset_in_loop(cell, order, |seen| seen.wrapping_add(count))
    }
    #[inline]
    fn fetch_ptr_sub(cell: &Self::Cell, count: usize, order: CoreOrdering) -> Self {
        offset_in_loop(cell, order, |seen| seen.wrapping_sub(count))
    }
    #[inline]
    fn fetch_byte_add(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self {
        offset_in_loop(cell, order, |seen| seen.wrapping_byte_add(bytes))
    }
    #[inline]
    fn fetch_byte_sub(cell: &Self::Cell, bytes: usize, order: CoreOrdering) -> Self {
        offset_in_loop(cell, order, |seen| seen.wrapping_byte_sub(bytes))
    }
}

/// Applies `offset` to the pointer in a compare-exchange loop, and returns the pointer before.
#[cfg(loom)]
fn offset_in_loop<T, F: Fn(*mut T) -> *mut T>(
    cell: &atomic::AtomicPtr<T>, order: CoreOrdering, offset: F,
) -> *mut T {
    // ORDERING: `order` on the exchange that lands, whose read is the pointer returned; Relaxed on
    // every other read, which is only compared.
    let mut seen = cell.load(CoreOrdering::Relaxed);
    loop {
        match cell.compare_exchange_weak(seen, offset(seen), order, CoreOrdering::Relaxed) {
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
};

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::Primitive;

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
}
