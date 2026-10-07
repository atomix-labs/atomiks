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

/// Derives `Atom` for a struct or an enum, so that one `Atomic` holds it.
///
/// The impl is `const`, so a `static` of the type builds, and a type without parameters needs no
/// feature gate. `#[atom(…)]` on the type takes two keys: `repr`, below, and `crate = path`, which
/// names atomiks where `::atomiks` does not. On a field, `#[atom(ptr)]` marks a pointer its type
/// does not show: an alias, a parameter, a newtype of one, a pointer word or a pointer enum.
///
/// # Shapes
/// - **Newtype**, a struct of one field beside any `PhantomData` markers: the field's repr, range,
///   validity and conversions.
/// - **Zero-width**, a unit struct or one of markers alone: its one value, zero, in a `u8`.
/// - **Fieldless enum**: each variant as its discriminant, exactly as rustc evaluates it.
/// - **Packed struct**, of several fields: each field in bits of its own, from bit 0 in declaration
///   order, as few as its reprs need, in two's complement where that is fewer; the bits above
///   extend the last field of any bits, so a signed one's sign fills them. A ranged integer field
///   takes the bits its range needs, with no attribute: 4 for a `RangedI8<-5, 5>`.
/// - **Enum with fields**, tagged: each variant's fields packed as a struct's, below a tag of its
///   discriminant. Where one variant alone has fields and none states a discriminant, niche-filling
///   instead, unless that is wider: the unit variants take the reprs beside that variant's range,
///   as `Option`'s `None` takes one.
/// - **Pointer word**, a struct of several fields, one or two of them pointers, each written as a
///   `NonNull`, an `Option` of one or a raw pointer, or marked: the pointer's repr, each other
///   field a tag packed above the pointer's own tags into the low bits its pointee's alignment
///   leaves clear, which the build checks, a generic word's in each instance. Its `to_repr` panics
///   on a pointer with a bit set where the tags go. Two words, a `DoubleWord`, hold two pointers,
///   the tags in the first one's low bits; a pointer to a slice or a `str`, the tags in its data
///   pointer's, or to a trait object, beside no tag; or one pointer beside tags no alignment holds,
///   more than 29 bits, since Rust's largest alignment is 2^29, in an integer word of their own. A
///   pointer past those two words is refused. A pointer's width is read from its type as written:
///   one to a slice, a `str` or a trait object behind an alias, or to a struct whose tail is a
///   slice, is taken for one word, and refused beside tags, so write such a pointer out. Two words
///   need a 16-byte atomic: where `x86_64` has no `cmpxchg16b`, the type is refused once, naming
///   the CPU it needs.
/// - **Pointer enum**, one variant or more holding a pointer: a `*mut ()`, each variant's tag its
///   discriminant, above its pointers' own tags; a pointer variant's tag fields above the tag, a
///   data variant's fields above the bits every pointer's alignment leaves clear; one unit beside
///   one pointer that is never null takes null, and no tag, unless a variant states its
///   discriminant. Its `to_repr` panics as a pointer word's does.
///
/// A pointer word's or a pointer enum's layout, a pointer word's width of one word or two among
/// it, reads its fields but never their pointees' alignment, which the build checks once the
/// pointees are laid out: so a node may hold an atomic of the word that points to it, as a list's
/// node holds the link to the next, and a pointee's alignment that shrinks below a word's tags
/// fails the build, never widening the word.
///
/// `from_repr` refuses each repr no value encodes to, so each value has one repr.
///
/// # Fields
/// A packed struct's atomic projects onto its fields. Beside a struct `Quote`, the derive writes
/// the projection `Atomic::fields` lends, and `AtomicField::fields` where a field's value is a
/// `Quote`: `QuoteFields<'a, P>`, of `Quote`'s visibility, and `#[non_exhaustive]` and
/// `#[doc(hidden)]` where `Quote` is. It holds one `&'a AtomicField` per field, each of the field's
/// own visibility and docs, so each field is a place of its own, changed alone, and a private
/// field's place stays in its module. Each place's type names its field's path, as `Field<Quote,
/// 2, bool>`, which is a type alone. A tuple struct's projection is a tuple struct. A type named
/// `QuoteFields` beside `Quote` clashes with the projection. A pointer word projects so too: each
/// tag is a place, and its pointer a place whose `load` reads it through the word, and whose own
/// `fields` reach an inner word's tags.
///
/// Generic code takes a place as `&AtomicField<P>`, and states what each operation asks of the
/// container's repr: [`BitTest`], [`MaskBitwise`] or [`FetchAdd`].
///
/// [`BitTest`]: https://docs.rs/atomiks/latest/atomiks/trait.BitTest.html
/// [`FetchAdd`]: https://docs.rs/atomiks/latest/atomiks/trait.FetchAdd.html
/// [`MaskBitwise`]: https://docs.rs/atomiks/latest/atomiks/trait.MaskBitwise.html
///
/// # Repr
/// A newtype's is its field's, a fieldless enum's the integer its `#[repr]` names, or C's `int`
/// under `#[repr(C)]`, which must hold each discriminant, a pointer enum's its pointer's, which
/// `repr = u64` or `usize` alone may state, and a pointer word's its pointer's, or a `DoubleWord`
/// of two words; any other is the narrowest unsigned integer that holds the layout. Of a pointer
/// word, `repr = u64` or `usize` states one word, and refuses tags its pointee's alignment cannot
/// hold; `repr = u128` or `i128` states two, its tags in an integer word beside one pointer, even
/// where they would fit its low bits. No repr is wider than 128 bits, or 64 on `x86_64` without
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
/// - A pointer word is what its fields promise, its pointers among them, and a pointer enum what
///   the variant its zero repr holds promises of zero; with parameters, a pointer word is
///   `ZeroValid` where each field's zero decodes, and a pointer enum where its first variant is a
///   unit and none states a discriminant; else `Partial`.
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
/// has parameters states its repr, which each instance is checked against as it is built, but a
/// pointer word or a pointer enum, whose repr is its pointer's, need not state one: a pointer word
/// of one pointer is one word in each instance, unless it states `repr = u128`. A fieldless enum
/// takes no parameters.
///
/// # Threads
/// An `Atomic` may cross threads, so the type must be `Send` and `Sync`: checked beside the impl,
/// or bounded where it has parameters, so `Wrap<*mut u8>` has no impl, nor a type whose negative
/// impl says it is neither. A newtype whose field is written as a raw pointer, a `NonNull` or an
/// `Option` of one is neither, yet may cross, as a pointer's `Atom` impl promises: its markers
/// alone are checked, and a pointer word's or a variant's pointer field is exempt alike. A packed
/// struct refuses a field stored as a pointer that its type does not show, an alias or a parameter
/// left unmarked, whose bits would lose its provenance.
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
/// ## Changing One Field
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use atomiks::ordering::{AcqRel, Acquire};
/// use atomiks::{Atom, Atomic, RangedU32};
///
/// /// A ring buffer's slot: whether it is written, its lap, then how many read it, in the top half.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Slot {
///     written: bool,
///     lap: RangedU32<0, 0x7FFF_FFFF>,
///     readers: u32,
/// }
///
/// static SLOT: Atomic<Slot> = Atomic::new(Slot { written: false, lap: RangedU32::MIN, readers: 0 });
///
/// // The one writer that finds the slot unwritten writes it: one `lock bts`, or `ldsetal`.
/// assert!(!SLOT.fields().written.test_and_set(AcqRel), "this writer claimed the slot");
/// assert!(SLOT.fields().written.test_and_set(AcqRel), "and no other can");
/// // Each reader counts itself in at the word's top, so a carry would leave the word.
/// let before = SLOT.fields().readers.fetch_add(1, AcqRel);
/// let claimed = Slot { written: true, lap: RangedU32::MIN, readers: 0 };
/// assert_eq!(before, claimed, "the first reader of a written slot");
/// assert_eq!(SLOT.fields().readers.load(Acquire), 1, "and one reader now");
/// ```
///
/// ## Changing a Field Through Generic Code
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use atomiks::ordering::{AcqRel, Acquire};
/// use atomiks::{Atom, Atomic, AtomicField, BitTest, FetchAdd, FieldPath, RangedU32, TopField};
///
/// /// What an order may do: whether it is halted, and whether it only rests on the book.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Flags {
///     halted: bool,
///     post_only: bool,
/// }
///
/// /// An order: its flags, a price in ticks, then how many of it filled, in the top half.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Order {
///     flags: Flags,
///     price: RangedU32<0, 0x3FFF_FFFF>,
///     filled: u32,
/// }
///
/// /// The halt of whatever order holds `flags`: whether this call halted it.
/// fn halt<P>(flags: &AtomicField<P>) -> bool
/// where
///     P: FieldPath<Value = Flags, Container: Atom<Repr: BitTest>>,
/// {
///     !flags.fields().halted.test_and_set(AcqRel)
/// }
///
/// /// Counts `quantity` more filled, in a field at the top of its word.
/// fn fill<P>(filled: &AtomicField<P>, quantity: u32)
/// where
///     P: TopField<Value = u32, Container: Atom<Repr: FetchAdd>>,
/// {
///     filled.fetch_add(quantity, AcqRel);
/// }
///
/// const PRICE: RangedU32<0, 0x3FFF_FFFF> = RangedU32::new(10_050).expect("below 2^30 ticks");
/// static ORDER: Atomic<Order> = Atomic::new(Order {
///     flags: Flags { halted: false, post_only: true },
///     price: PRICE,
///     filled: 0,
/// });
///
/// fill(ORDER.fields().filled, 300);
/// // `ORDER.fields().flags.fields().halted`, a field of a field, in one `lock bts`, or `ldsetal`.
/// assert!(halt(ORDER.fields().flags), "this call halted the order");
/// assert!(!halt(ORDER.fields().flags), "and no other call does");
/// let flags = Flags { halted: true, post_only: true };
/// let halted = Order { flags, price: PRICE, filled: 300 };
/// assert_eq!(ORDER.load(Acquire), halted, "filled, then halted");
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
/// ## Tagging a Pointer
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use core::ptr::{self, NonNull};
///
/// use atomiks::ordering::{AcqRel, Acquire, Release};
/// use atomiks::{Atom, Atomic};
///
/// /// A node of a list, which holds the link to the next node, or none: aligned to 8, so a
/// /// pointer to one leaves three low bits clear.
/// #[repr(align(8))]
/// struct Node {
///     value: u64,
///     next: Atomic<Option<Link>>,
/// }
///
/// /// A Harris list's link: the next node, and whether the node that holds the link is deleted.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Link {
///     next: NonNull<Node>,
///     deleted: bool,
/// }
///
/// /// A link and a lock its writer takes: a word holding another, its tag above the link's.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Guarded {
///     #[atom(ptr)]
///     link: Link,
///     locked: bool,
/// }
///
/// let node = Node { value: 7, next: Atomic::new(None) };
/// let next = NonNull::from(&node);
/// let deleted = Link { next, deleted: true };
/// assert_eq!(deleted.to_repr().addr(), next.as_ptr().addr() | 1, "the mark in bit 0");
/// assert_eq!(None::<Link>.to_repr(), ptr::null_mut(), "`None` null, which no link is");
///
/// let guarded = Atomic::new(Guarded { link: Link { next, deleted: false }, locked: false });
/// guarded.fields().locked.set(Release);
/// // A field of a field, in bit 0 below the lock: one `lock bts`, or `ldsetal`.
/// assert!(!guarded.fields().link.fields().deleted.test_and_set(AcqRel), "this thread deleted it");
/// assert_eq!(guarded.fields().link.load(Acquire), deleted, "the link, read through the word");
/// assert_eq!(guarded.load(Acquire).to_repr().addr(), next.as_ptr().addr() | 0b11, "both tags");
/// ```
///
/// ## Tagging Pointers in Two Words
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use core::ptr::NonNull;
///
/// use atomiks::Atom;
///
/// /// A node of a graph, aligned to 8, so a pointer to one leaves three low bits clear.
/// #[repr(align(8))]
/// struct Node {
///     value: u64,
/// }
///
/// /// An edge of a graph, and whether it is deleted: two pointers, so two words, the mark in the
/// /// first one's bit 0.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// struct Edge {
///     source: NonNull<Node>,
///     target: NonNull<Node>,
///     deleted: bool,
/// }
///
/// /// A link and how many readers hold it: 16 bits, more than a node's alignment leaves clear, so
/// /// the link states two words, and the count takes a word of its own.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// #[atom(repr = u128)]
/// struct Link {
///     next: Option<NonNull<Node>>,
///     readers: u16,
/// }
///
/// let nodes = [Node { value: 1 }, Node { value: 2 }];
/// let (source, target) = (NonNull::from(&nodes[0]), NonNull::from(&nodes[1]));
/// let repr = Edge { source, target, deleted: true }.to_repr();
/// assert_eq!(repr.first.addr(), source.as_ptr().addr() | 1, "the mark in the first pointer");
/// assert_eq!(repr.second, target.as_ptr(), "and the second as it is");
/// let repr = Link { next: Some(source), readers: 3 }.to_repr();
/// assert_eq!((repr.first, repr.second), (source.as_ptr(), 3), "the count beside the pointer");
/// ```
///
/// ## Storing a Pointer Enum
/// ```
/// # extern crate atomiks_core as atomiks;
/// # use atomiks_derive::Atom;
/// use core::ptr::{self, NonNull};
///
/// use atomiks::{Atom, Atomic};
///
/// /// A leaf of a tree, aligned to 8.
/// #[repr(align(8))]
/// struct Leaf {
///     value: u64,
/// }
///
/// /// A branch of a tree, aligned to 8.
/// #[repr(align(8))]
/// struct Branch {
///     left: u64,
///     right: u64,
/// }
///
/// /// A child of a tree: a leaf or a branch, its tag in bit 0, which both alignments leave clear.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// enum Child {
///     Leaf(NonNull<Leaf>),
///     Branch(NonNull<Branch>),
/// }
///
/// /// The next leaf of a list, or its end, which takes null: `Option<NonNull<Leaf>>`'s layout.
/// #[derive(Clone, Copy, Debug, PartialEq, Eq, Atom)]
/// enum Next {
///     End,
///     Leaf(NonNull<Leaf>),
/// }
///
/// let subtree = Branch { left: 1, right: 2 };
/// let branch = NonNull::from(&subtree);
/// assert_eq!(Child::Branch(branch).to_repr().addr(), branch.as_ptr().addr() | 1, "tagged 1");
/// assert_eq!(Next::End.to_repr(), ptr::null_mut(), "the end null, with no tag");
/// assert_eq!(size_of::<Atomic<Option<Child>>>(), 8, "and `None` null, which no child is");
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

/// Derives `AtomAdd` for a newtype whose field has it: `fetch_add` and `fetch_sub`.
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

/// Derives `AtomOrd` for a newtype whose field has it: `fetch_max` and `fetch_min`.
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
/// // `fetch_max` exists where it is one instruction: on `aarch64`, not `x86_64`.
/// #[cfg(target_arch = "aarch64")]
/// {
///     HIGH.fetch_max(Price(10_100), Relaxed);
///     HIGH.fetch_max(Price(10_050), Relaxed);
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
