//! `Atom` for pointers, each stored in an atomic pointer, so it keeps its provenance, or, a wide
//! one, in a `DoubleWord`; and [`PtrAtom`], every value stored as one pointer.

#![expect(unsafe_code, reason = "each `Atom` impl here promises what loads rely on")]

#[cfg(wide)]
use core::fmt;
#[cfg(wide)]
use core::hash::{Hash, Hasher};
#[cfg(wide)]
use core::marker::PhantomData;
#[cfg(wide)]
use core::mem::{align_of_val_raw, transmute};
#[cfg(wide)]
use core::ptr::{self, DynMetadata};
use core::ptr::{NonNull, Pointee};

use super::Atom;
use crate::primitive::{CompareExchange, Primitive};
#[cfg(wide)]
use crate::primitive::{DoubleWord, Word};
#[cfg(wide)]
use crate::range::mask;
use crate::range::{PointeeAlignment, ReprRange};
#[cfg(wide)]
use crate::validity::ZeroNiche;
use crate::validity::{Total, TotalZeroNiche, Validity};

/// The metadata of a pointer to a `T`: `()` for a sized `T`, a length for a slice or a `str`, a
/// `DynMetadata` for a trait object.
type Metadata<T> = <T as Pointee>::Metadata;

/// How a pointer to a `T` is stored, chosen by `T`'s metadata: a sized pointee's pointer alone, in
/// one word; a slice's or a `str`'s data pointer and length, and a trait object's data pointer and
/// `VtablePointer`, in a `DoubleWord`, each pointer's provenance exposed as a double word's is.
///
/// One `Atom` impl for each pointer kind reads it, since an impl for a slice's pointer and one for
/// a trait object's would overlap: coherence does not tell their metadata apart. Each impl is
/// `do_not_recommend`, so a pointer this target cannot store, a wide one without a 16-byte atomic,
/// reports `Atom`'s refusal, which names the CPU it needs, never this trait.
pub impl(crate) const trait PointerMetadata<T: ?Sized>: Copy {
    /// The repr of a pointer to a `T`.
    type Repr: const Primitive + CompareExchange;
    /// Which reprs decode as a raw pointer to a `T`.
    type RawValidity: const Validity;
    /// Which reprs decode as a `NonNull<T>`.
    type NonNullValidity: const Validity;
    /// The reprs a raw pointer to a `T` takes.
    const RAW_REPRS: ReprRange<Self::Repr>;
    /// The alignment of a `T`, where it is known at compile time, as the low bits it leaves clear
    /// in a pointer's address.
    const POINTEE_ALIGNMENT: PointeeAlignment;
    /// The repr of `pointer`.
    fn to_repr(pointer: *mut T) -> Self::Repr;
    /// The pointer `repr` holds, or `None` where its metadata could be no pointer's.
    fn from_repr(repr: Self::Repr) -> Option<*mut T>;
}

// A sized pointee's pointer is its own repr, every repr a pointer.
const impl<T> PointerMetadata<T> for () {
    type Repr = *mut T;
    type RawValidity = Total;
    type NonNullValidity = TotalZeroNiche;
    const RAW_REPRS: ReprRange<*mut T> = ReprRange::FULL;
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::of::<T>();
    #[inline]
    fn to_repr(pointer: *mut T) -> *mut T {
        pointer
    }
    #[inline]
    fn from_repr(repr: *mut T) -> Option<*mut T> {
        Some(repr)
    }
}

// A slice's or a `str`'s pointer is its data pointer and its length, and every data pointer beside
// every length is a raw pointer's: a raw pointer's length is no promise of what it points to.
#[cfg(wide)]
const impl<T: ?Sized + Pointee<Metadata = usize>> PointerMetadata<T> for usize {
    type Repr = DoubleWord<*mut (), usize>;
    type RawValidity = Total;
    type NonNullValidity = ZeroNiche;
    const RAW_REPRS: ReprRange<Self::Repr> = ReprRange::FULL;
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::new::<T>(tail_alignment::<T>());
    #[inline]
    fn to_repr(pointer: *mut T) -> Self::Repr {
        let (data, length) = pointer.to_raw_parts();
        DoubleWord { first: data, second: length }
    }
    #[inline]
    fn from_repr(repr: Self::Repr) -> Option<*mut T> {
        Some(ptr::from_raw_parts_mut(repr.first, repr.second))
    }
}

/// The alignment of a `T` whose tail is a slice or a `str`, in bytes.
#[cfg(wide)]
#[expect(unsafe_code, reason = "reads a slice-tailed type's alignment through a pointer to none")]
const fn tail_alignment<T: ?Sized + Pointee<Metadata = usize>>() -> usize {
    let empty = ptr::from_raw_parts::<T>(ptr::null::<()>(), 0);
    // SAFETY: `empty`'s tail is a slice or a `str` of no elements, whose length is initialized, and
    // its size is the rest of `T`'s, which fits an `isize`, as every type's does.
    unsafe { align_of_val_raw(empty) }
}

/// The vtable pointer of a pointer to the trait object `T`, the second word of the
/// [`DoubleWord`] that holds the pointer: one a [`DynMetadata`] gives, or null.
///
/// It has no constructor: a trait object's pointer gives one, and bits make none. A word built of
/// bits, or changed to them, is null, which decodes as no pointer, so no safe code forges a vtable
/// that a pointer decoded from a repr would call through. A compare-exchange compares it, and one
/// type may have several vtables, so it takes its `current` from a load, as [`DoubleWord`] says.
#[cfg(wide)]
#[doc(cfg(any(
    target_arch = "aarch64",
    target_arch = "arm64ec",
    all(target_arch = "x86_64", target_feature = "cmpxchg16b")
)))]
#[repr(transparent)]
pub struct VtablePointer<T: ?Sized> {
    // INVARIANT: null, or a `DynMetadata<T>`'s vtable pointer, with the provenance it had or one
    // exposed beside its address. Its writers are `of`, which reads a `DynMetadata<T>`'s; `Word`'s
    // `from_bits`, `with_packed_bits` and `clear_packed_bits`, which keep it or make it null; and
    // `Word::from_exposed_bits`, which only a double word's cell calls, on the bits it holds,
    // those `exposed_bits` gave of such a pointer: the cell lends no place for other bits. `Word`
    // is unnameable outside atomix, so no other code calls them.
    //
    // A vtable pointer, unlike a data pointer, points to no memory Rust's abstract machine knows
    // of, and std documents no exposure of one: that one taken back from its exposed address keeps
    // the vtable is assumed, and no checker runs it, since Miri runs the lock model, which exposes
    // no word, and checks each call through the pointer it kept.
    /// The vtable pointer.
    pointer: *mut (),
    /// The trait object whose vtable it is.
    marker: PhantomData<fn() -> *const T>,
}

#[cfg(wide)]
impl<T: ?Sized> VtablePointer<T> {
    /// No vtable, which decodes as no pointer.
    const NULL: Self = Self { pointer: ptr::null_mut(), marker: PhantomData };
}

#[cfg(wide)]
impl<T: ?Sized + Pointee<Metadata = DynMetadata<T>>> VtablePointer<T> {
    /// The vtable pointer of `metadata`.
    #[inline]
    const fn of(metadata: DynMetadata<T>) -> Self {
        // SAFETY: `DynMetadata` is its vtable's `NonNull` beside a `PhantomData`, a layout the
        // compiler hard-codes and core's own `vtable_ptr` reads by this `transmute`, so its bytes
        // are that pointer's, provenance and all, as many as a `*mut ()`'s, which `transmute`
        // checks.
        let pointer = unsafe { transmute::<DynMetadata<T>, *mut ()>(metadata) };
        Self { pointer, marker: PhantomData }
    }

    /// The metadata whose vtable pointer this is, or `None` for null.
    #[inline]
    const fn metadata(self) -> Option<DynMetadata<T>> {
        if self.pointer.is_null() {
            return None;
        }
        // SAFETY: by the field INVARIANT, a pointer that is not null is a `DynMetadata<T>`'s
        // vtable pointer, with its provenance or one taken back from its exposed address, so its
        // bytes are a `DynMetadata<T>`'s, as `of` says.
        Some(unsafe { transmute::<*mut (), DynMetadata<T>>(self.pointer) })
    }
}

// A vtable pointer's bits are its address; bits that are not its own make it null.
#[cfg(wide)]
const impl<T: ?Sized> Word for VtablePointer<T> {
    const IS_BITS_EXACT: bool = false;
    #[inline]
    fn from_bits(_: u128) -> Self {
        Self::NULL
    }
    #[inline]
    fn is_bits(self, bits: u128) -> bool {
        Word::is_bits(self.pointer, bits)
    }
    #[inline]
    fn packed_bits(self) -> u128 {
        Word::packed_bits(self.pointer)
    }
    #[inline]
    fn with_packed_bits(self, bits: u128) -> Self {
        if Word::packed_bits(self.pointer) == bits & mask(usize::BITS) { self } else { Self::NULL }
    }
    #[inline]
    fn clear_packed_bits(self, mask: u128) -> Self {
        if Word::packed_bits(self.pointer) & mask == 0 { self } else { Self::NULL }
    }
    #[inline]
    fn exposed_bits(self) -> u64 {
        self.pointer.exposed_bits()
    }
    #[inline]
    fn from_exposed_bits(bits: u64) -> Self {
        Self { pointer: Word::from_exposed_bits(bits), marker: PhantomData }
    }
}

// Written out: a derive would bound `T`, a trait object, by each trait.
#[cfg(wide)]
impl<T: ?Sized> Clone for VtablePointer<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(wide)]
impl<T: ?Sized> Copy for VtablePointer<T> {}

#[cfg(wide)]
impl<T: ?Sized> PartialEq for VtablePointer<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.pointer == other.pointer
    }
}

#[cfg(wide)]
impl<T: ?Sized> Eq for VtablePointer<T> {}

#[cfg(wide)]
impl<T: ?Sized> Hash for VtablePointer<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pointer.hash(state);
    }
}

#[cfg(wide)]
impl<T: ?Sized> fmt::Debug for VtablePointer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VtablePointer").field(&self.pointer).finish()
    }
}

// A trait object's pointer is its data pointer and its vtable pointer, carried as a
// `VtablePointer`, so it keeps its provenance, which one rebuilt from its address alone would not,
// and so no repr forges one. Its alignment is the vtable's, read at run time, so none is known
// here.
#[cfg(wide)]
const impl<T: ?Sized + Pointee<Metadata = DynMetadata<T>>> PointerMetadata<T> for DynMetadata<T> {
    type Repr = DoubleWord<*mut (), VtablePointer<T>>;
    type RawValidity = ZeroNiche;
    type NonNullValidity = ZeroNiche;
    // The vtable pointer is never null.
    const RAW_REPRS: ReprRange<Self::Repr> = ReprRange::NONZERO;
    const POINTEE_ALIGNMENT: PointeeAlignment = PointeeAlignment::NONE;
    #[inline]
    fn to_repr(pointer: *mut T) -> Self::Repr {
        let (data, metadata) = pointer.to_raw_parts();
        DoubleWord { first: data, second: VtablePointer::of(metadata) }
    }
    #[inline]
    fn from_repr(repr: Self::Repr) -> Option<*mut T> {
        match repr.second.metadata() {
            Some(metadata) => Some(ptr::from_raw_parts_mut(repr.first, metadata)),
            None => None,
        }
    }
}

/// Stored as the pointer itself, `*mut T`, where `T` is sized; a pointer to a slice or a `str` as
/// its data pointer and length, a `DoubleWord<*mut (), usize>`; and a pointer to a trait object
/// as its data pointer and vtable pointer, a `DoubleWord<*mut (), VtablePointer<T>>`, whose
/// compare-exchange takes `current` from a load, as `DoubleWord` says. Two words need a 16-byte
/// atomic, which `x86_64` has from `x86-64-v2`.
///
/// Every repr decodes, but a trait object's with a null vtable pointer, which none has.
// SAFETY: the repr is the pointer's, as `PointerMetadata` stores it, within its range, every repr,
// or every one but zero for a trait object's, whose vtable pointer is never null; `from_repr`
// decodes exactly the reprs whose metadata is a pointer's, every one but a trait object's with no
// vtable, as the validity says, each as the pointer it holds; and its address may cross threads,
// as `AtomicPtr`'s does.
#[diagnostic::do_not_recommend]
const unsafe impl<T: ?Sized> Atom for *mut T
where
    Metadata<T>: [const] PointerMetadata<T>,
{
    type Repr = <Metadata<T> as PointerMetadata<T>>::Repr;
    type Validity = <Metadata<T> as PointerMetadata<T>>::RawValidity;
    const REPRS: ReprRange<Self::Repr> = <Metadata<T> as PointerMetadata<T>>::RAW_REPRS;
    const POINTEE_ALIGNMENT: PointeeAlignment =
        <Metadata<T> as PointerMetadata<T>>::POINTEE_ALIGNMENT;
    #[inline]
    fn to_repr(self) -> Self::Repr {
        <Metadata<T> as PointerMetadata<T>>::to_repr(self)
    }
    #[inline]
    fn from_repr(repr: Self::Repr) -> Option<Self> {
        <Metadata<T> as PointerMetadata<T>>::from_repr(repr)
    }
}

/// Stored as a `*mut T` is: the pointer itself where `T` is sized, else in a `DoubleWord`.
// SAFETY: as `*mut T`'s, the pointer made mutable.
#[diagnostic::do_not_recommend]
const unsafe impl<T: ?Sized> Atom for *const T
where
    Metadata<T>: [const] PointerMetadata<T>,
{
    type Repr = <Metadata<T> as PointerMetadata<T>>::Repr;
    type Validity = <Metadata<T> as PointerMetadata<T>>::RawValidity;
    const REPRS: ReprRange<Self::Repr> = <Metadata<T> as PointerMetadata<T>>::RAW_REPRS;
    const POINTEE_ALIGNMENT: PointeeAlignment =
        <Metadata<T> as PointerMetadata<T>>::POINTEE_ALIGNMENT;
    #[inline]
    fn to_repr(self) -> Self::Repr {
        <Metadata<T> as PointerMetadata<T>>::to_repr(self.cast_mut())
    }
    #[inline]
    fn from_repr(repr: Self::Repr) -> Option<Self> {
        match <Metadata<T> as PointerMetadata<T>>::from_repr(repr) {
            Some(pointer) => Some(pointer.cast_const()),
            None => None,
        }
    }
}

/// Stored as a `*mut T` is: the pointer itself where `T` is sized, else in a `DoubleWord`, whose
/// first word, the data pointer, is never null, so an `Option` of it is free.
// SAFETY: the repr is the pointer's, as `PointerMetadata` stores it, its data pointer never null,
// so within every repr but zero; `from_repr` decodes exactly the reprs whose metadata is a
// pointer's and whose data pointer is not null, as the validity says, each as the pointer it
// holds; and its address may cross threads, as `AtomicPtr`'s does.
#[diagnostic::do_not_recommend]
const unsafe impl<T: ?Sized> Atom for NonNull<T>
where
    Metadata<T>: [const] PointerMetadata<T>,
{
    type Repr = <Metadata<T> as PointerMetadata<T>>::Repr;
    type Validity = <Metadata<T> as PointerMetadata<T>>::NonNullValidity;
    const REPRS: ReprRange<Self::Repr> = ReprRange::NONZERO;
    const POINTEE_ALIGNMENT: PointeeAlignment =
        <Metadata<T> as PointerMetadata<T>>::POINTEE_ALIGNMENT;
    #[inline]
    fn to_repr(self) -> Self::Repr {
        <Metadata<T> as PointerMetadata<T>>::to_repr(self.as_ptr())
    }
    #[inline]
    fn from_repr(repr: Self::Repr) -> Option<Self> {
        match <Metadata<T> as PointerMetadata<T>>::from_repr(repr) {
            Some(pointer) => Self::new(pointer),
            None => None,
        }
    }
    #[inline]
    unsafe fn from_repr_unchecked(repr: Self::Repr) -> Self {
        let pointer = <Metadata<T> as PointerMetadata<T>>::from_repr(repr);
        // SAFETY: the caller's repr decodes, so its metadata is a pointer's.
        let pointer = unsafe { pointer.unwrap_unchecked() };
        // SAFETY: the caller's repr decodes, so its data pointer is not null.
        unsafe { Self::new_unchecked(pointer) }
    }
}

/// A value stored as a pointer to its [`Pointee`](Self::Pointee): a `NonNull`, an `Option` of one,
/// a raw pointer, or any value whose [`Atom::Repr`] is a `*mut`, such as a newtype of one, a
/// pointer word or a pointer enum.
///
/// A pointer word, `#[derive(Atom)]`'s struct of one such field beside tag fields, packs the tags
/// into the low bits the pointer's alignment leaves clear, and its derive bounds a generic pointer
/// field by this trait. A pointer to a slice, a `str` or a trait object is none: two words hold it.
///
/// One impl gives it to every such [`Atom`], so it promises nothing of its own: it names the
/// pointee, so generic code knows the repr is `*mut Self::Pointee`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not stored as a pointer, so no tag can share its bits",
    label = "expected a value whose repr is a `*mut`",
    note = "a pointer word's pointer field, or a pointer enum variant's, is a `NonNull`, an `Option` of one, a raw pointer, or a value stored as one, such as a newtype of one",
    note = "a pointer to a slice, a `str` or a trait object is two words, which the derive sees where the field's type writes the pointer out, never through an alias"
)]
pub const trait PtrAtom: [const] Atom<Repr = *mut Self::Pointee> {
    /// The type the pointer points to.
    type Pointee;
}

// `do_not_recommend`, so a value whose repr is no pointer reports `PtrAtom`'s message.
#[diagnostic::do_not_recommend]
const impl<T: [const] Atom<Repr = *mut P>, P> PtrAtom for T {
    type Pointee = P;
}
