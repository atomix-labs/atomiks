//! The derives of atomiks: `Atom`, and `AtomAdd`, `AtomOrd` and `AtomBitwise`, the
//! read-modify-writes a newtype takes from its field.
//!
//! atomiks re-exports each under its `derive` feature: depend on atomiks, not on this crate. The
//! logic is `atomiks-derive-impl`'s; this crate renders its errors as the compiler's.

#![feature(allow_internal_unstable, proc_macro_def_site, proc_macro_diagnostic)]
#![expect(
    internal_features,
    reason = "`allow_internal_unstable` lets a crate without `const_trait_impl` build the `const` impl \
              `Atom`'s derive writes"
)]

use atomiks_derive_impl::{Capability, DeriveError, Expansion, expand_atom, expand_capability};
use proc_macro::{Diagnostic, Level, Span, TokenStream};

/// Derives `Atom` for a struct or an enum, so that an `Atomic` holds it in one word.
///
/// The impl is `const`, so a `static` of the type builds, and a type without parameters needs no
/// feature gate. `#[atom(…)]` on the type takes two keys: `repr`, below, and `crate = path`, which
/// names atomiks where `::atomiks` does not.
///
/// # Shapes
/// - **Newtype**, a struct of one field beside any `PhantomData` markers: the field's repr, range,
///   validity and conversions.
/// - **Zero-width**, a unit struct or one of markers alone: its one value, zero, in a `u8`.
/// - **Fieldless enum**: each variant as its discriminant, exactly as rustc evaluates it.
/// - **Packed struct**, of several fields: each field in bits of its own, from bit 0 in declaration
///   order, as few as its reprs need, in two's complement where that is fewer; the bits above
///   extend the last field of any bits, so a signed one's sign fills them.
/// - **Enum with fields**, tagged: each variant's fields packed as a struct's, below a tag of its
///   discriminant. Where one variant alone has fields and none states a discriminant, niche-filling
///   instead, unless that is wider: the unit variants take the reprs beside that variant's range,
///   as `Option`'s `None` takes one.
///
/// `from_repr` refuses each repr no value encodes to, so each value has one repr.
///
/// # Repr
/// A newtype's is its field's, and a fieldless enum's the integer its `#[repr]` names, or C's `int`
/// under `#[repr(C)]`, which must hold each discriminant; any other is the narrowest unsigned
/// integer that holds the layout. No repr is wider than 128 bits, or 64 on `x86_64` without
/// `cmpxchg16b`: a value that needs more is refused.
///
/// `#[atom(repr = u64)]` states the repr, any integer primitive, so the build refuses a type that
/// outgrows it: a newtype's field must have it, a fieldless enum's `#[repr]`, where it has one,
/// must name it, and every other shape must fit in it.
///
/// # Validity
/// - A newtype has its field's, and a zero-width struct `ZeroValid`.
/// - A fieldless enum is `Total` where the discriminants take every repr, `TotalZeroNiche` every
///   one but zero, else `ZeroValid` or `ZeroNiche` as zero is one or not.
/// - A packed struct or an enum with fields is `Total` where every pattern of its fields' bits
///   decodes and they fill the repr, else `ZeroValid`, `ZeroNiche` or `Partial`, by what its fields
///   promise of zero. With parameters, a packed struct is `ZeroValid` where each field's zero
///   decodes, an enum with fields where it states its discriminants and a unit variant's is 0,
///   stated or implied; else `Partial`.
///
/// # Generic Types
/// A crate enables `#![feature(const_trait_impl)]` where the impl converts a field that names one
/// of the type's parameters, a lifetime too: a newtype's field that holds the value, or any field
/// of a packed struct or an enum with fields, a `PhantomData` among them. The impl converts such a
/// field through `Atom`'s methods, `const` only where the field's impl is, and every other field
/// through functions bounded `const`; so a newtype whose markers alone name its parameters, a
/// zero-width struct, or a type whose `const` parameter no field names needs no gate.
///
/// A newtype takes its field's layout in each instance; a packed struct or an enum with fields that
/// has parameters states its repr, which each instance is checked against as it is built. A
/// fieldless enum takes no parameters.
///
/// # Threads
/// An `Atomic` may cross threads, so the type must be `Send` and `Sync`: checked beside the impl,
/// or bounded where it has parameters, so `Wrap<*mut u8>` has no impl, nor a type whose negative
/// impl says it is neither. A newtype whose field is written as a raw pointer, a `NonNull` or an
/// `Option` of one is neither, yet may cross, as a pointer's `Atom` impl promises: its markers
/// alone are checked. A packed struct or an enum with fields refuses a pointer field, whose bits
/// would lose its provenance.
///
/// # Examples
/// ## Wrapping a Value
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use core::num::NonZero;
///
/// use atomiks::ordering::{Acquire, Release};
/// use atomiks::{Atom, Atomic};
///
/// /// An owner's id, never zero.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct OwnerId(NonZero<u64>);
///
/// // The id's validity is its field's, so `None` takes zero, which no id is.
/// static OWNER: Atomic<Option<OwnerId>> = Atomic::new(None);
///
/// let owner = OwnerId(NonZero::new(7).expect("7 is not zero"));
/// OWNER.store(Some(owner), Release);
/// assert_eq!(OWNER.load(Acquire), Some(owner), "the owner stored");
/// assert_eq!(None::<OwnerId>.to_repr(), 0, "with `None` at zero");
/// ```
///
/// ## Storing a Marker
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use atomiks::ordering::{Acquire, Release};
/// use atomiks::{Atom, Atomic};
///
/// /// Why a queue closed: its producer hung up.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct HungUp;
///
/// static CLOSED: Atomic<Option<HungUp>> = Atomic::new(None);
///
/// CLOSED.store(Some(HungUp), Release);
/// assert_eq!(CLOSED.load(Acquire), Some(HungUp), "closed, the producer having hung up");
/// assert_eq!(None::<HungUp>.to_repr(), 1, "`None` past the marker's zero");
/// ```
///
/// ## Storing a Fieldless Enum
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use atomiks::ordering::{AcqRel, Acquire, Release};
/// use atomiks::{Atom, Atomic};
///
/// /// Where a job is, from 1, so zero is spare.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// enum Phase {
///     Queued = 1,
///     Running,
///     Done,
/// }
///
/// static PHASE: Atomic<Option<Phase>> = Atomic::new(None);
///
/// PHASE.store(Some(Phase::Queued), Release);
/// assert_eq!(PHASE.swap(Some(Phase::Running), AcqRel), Some(Phase::Queued), "the phase before");
/// assert_eq!(Phase::Running.to_repr(), 2_u8, "its discriminant, in a `u8`");
/// assert_eq!(None::<Phase>.to_repr(), 0, "and `None` zero, which no phase is");
/// ```
///
/// ## Packing Several Fields
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use atomiks::ordering::{AcqRel, Acquire};
/// use atomiks::{Atom, Atomic};
///
/// /// A ring buffer's head: the slot to read next, and how many times the reader has wrapped.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Head {
///     slot: u16,
///     lap: u16,
/// }
///
/// static HEAD: Atomic<Head> = Atomic::new(Head { slot: 1023, lap: 0 });
///
/// // Past the last of 1024 slots, the slot and the lap change together, in one compare-exchange.
/// let (last, wrapped) = (Head { slot: 1023, lap: 0 }, Head { slot: 0, lap: 1 });
/// assert_eq!(HEAD.compare_exchange(last, wrapped, AcqRel, Acquire), Ok(last), "wrapped");
/// assert_eq!(wrapped.to_repr(), 1 << 16, "the lap above the slot, in a `u32`");
/// ```
///
/// ## Storing an Enum with Fields
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use core::num::NonZero;
///
/// use atomiks::Atom;
///
/// /// An owner's id, never zero.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct OwnerId(NonZero<u64>);
///
/// /// A ring buffer's slot: two variants with fields, so a tag of two bits above a lap's 32.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// enum Slot {
///     Empty,
///     Writing { lap: u32 },
///     Ready { lap: u32 },
/// }
///
/// /// A lock: one variant with fields, so `Free` takes zero beside the ids, in the 64 bits a tag
/// /// would widen.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// enum Lock {
///     Free,
///     Held(OwnerId),
/// }
///
/// assert_eq!(Slot::Ready { lap: 7 }.to_repr(), 2 << 32 | 7, "its index above the lap");
/// let owner = OwnerId(NonZero::new(7).expect("7 is not zero"));
/// assert_eq!(Lock::Held(owner).to_repr(), 7, "an owner's id as it is");
/// assert_eq!(Lock::Free.to_repr(), 0, "and `Free` zero");
/// ```
///
/// ## Deriving for a Generic Type
/// ```
/// #![feature(const_trait_impl)]
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
///
/// use atomiks::ordering::{AcqRel, Acquire};
/// use atomiks::{Atom, Atomic};
///
/// /// A value beside how many times it was written, so a compare-exchange tells two writes of the
/// /// same value apart.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// #[atom(repr = u64)]
/// struct Versioned<T> {
///     value: T,
///     version: u16,
/// }
///
/// // Each instance must fit: `Versioned<u64>`, of 80 bits, is refused where it is built.
/// static LIMIT: Atomic<Versioned<u32>> = Atomic::new(Versioned { value: 100, version: 0 });
///
/// let raise = |old: Versioned<u32>| Versioned { value: 200, version: old.version + 1 };
/// assert_eq!(LIMIT.update(AcqRel, Acquire, raise).value, 100, "the limit before");
/// assert_eq!(LIMIT.load(Acquire), Versioned { value: 200, version: 1 }, "raised, once");
/// ```
#[proc_macro_derive(Atom, attributes(atom))]
#[allow_internal_unstable(const_trait_impl)]
pub fn derive_atom(input: TokenStream) -> TokenStream {
    emit(expand_atom(input.into(), Span::def_site().into()))
}

/// Derives `AtomAdd` for a newtype whose field has it: `add`, `sub` and their `fetch_` forms.
///
/// # Examples
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::{Atom, AtomAdd};
/// use atomiks::ordering::{Acquire, Relaxed};
/// use atomiks::{Atom, AtomAdd, Atomic};
///
/// /// A sequence number, which adds as its field does.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomAdd)]
/// struct Seq(u64);
///
/// static NEXT: Atomic<Seq> = Atomic::new(Seq(1));
///
/// assert_eq!(NEXT.fetch_add(1, Relaxed), Seq(1), "the first number taken");
/// assert_eq!(NEXT.load(Acquire), Seq(2), "and the next one up");
/// ```
#[proc_macro_derive(AtomAdd, attributes(atom))]
pub fn derive_atom_add(input: TokenStream) -> TokenStream {
    emit(expand_capability(input.into(), Capability::Add))
}

/// Derives `AtomOrd` for a newtype whose field has it: `max`, `min` and their `fetch_` forms.
///
/// `AtomOrd` promises that the repr's order is the value's `Ord`, so the newtype's `Ord` must be
/// its field's: derive `Ord` too.
///
/// # Examples
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::{Atom, AtomOrd};
/// use atomiks::ordering::{Acquire, Relaxed};
/// use atomiks::{Atom, AtomOrd, Atomic};
///
/// /// A price in ticks, which orders as its field does.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Atom, AtomOrd)]
/// struct Price(u32);
///
/// static HIGH: Atomic<Price> = Atomic::new(Price(0));
///
/// // `max` exists where it is one instruction: on `aarch64`, not `x86_64`.
/// #[cfg(target_arch = "aarch64")]
/// {
///     HIGH.max(Price(10_100), Relaxed);
///     HIGH.max(Price(10_050), Relaxed);
///     assert_eq!(HIGH.load(Acquire), Price(10_100), "the highest price seen");
/// }
/// ```
#[proc_macro_derive(AtomOrd, attributes(atom))]
pub fn derive_atom_ord(input: TokenStream) -> TokenStream {
    emit(expand_capability(input.into(), Capability::Ord))
}

/// Derives `AtomBitwise` for a newtype whose field has it: `and`, `or`, `xor`, `not` and their
/// `fetch_` forms.
///
/// # Examples
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::{Atom, AtomBitwise};
/// use atomiks::ordering::{Acquire, Relaxed};
/// use atomiks::{Atom, AtomBitwise, Atomic};
///
/// /// The venues an order may route to, a bit each, which combine as their field's bits do.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom, AtomBitwise)]
/// struct Venues(u8);
///
/// static OPEN: Atomic<Venues> = Atomic::new(Venues(0b0011));
///
/// OPEN.and(Venues(0b1110), Relaxed);
/// OPEN.or(Venues(0b0100), Relaxed);
/// assert_eq!(OPEN.load(Acquire), Venues(0b0110), "venue 0 closed, and venue 2 opened");
/// ```
#[proc_macro_derive(AtomBitwise, attributes(atom))]
pub fn derive_atom_bitwise(input: TokenStream) -> TokenStream {
    emit(expand_capability(input.into(), Capability::Bitwise))
}

/// The code a derive writes, each of its errors rendered first.
fn emit(expansion: Expansion) -> TokenStream {
    expansion.errors.into_iter().for_each(render);
    expansion.code.into()
}

/// Emits `error` as the compiler's, with its notes and help.
fn render(error: DeriveError) {
    let mut diagnostic = Diagnostic::spanned(error.span.unwrap(), Level::Error, error.message);
    for (span, note) in error.notes {
        diagnostic = match span {
            Some(span) => diagnostic.span_note(span.unwrap(), note),
            None => diagnostic.note(note),
        };
    }
    if let Some(help) = error.help {
        diagnostic = diagnostic.help(help);
    }
    diagnostic.emit();
}
