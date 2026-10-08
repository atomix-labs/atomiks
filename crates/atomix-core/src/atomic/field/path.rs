//! The paths from an atomic's repr to the bits of one field: [`Field`], [`Then`] and [`Whole`],
//! each a [`FieldPath`], and each a type alone.

use core::any::type_name;
use core::fmt;
use core::marker::PhantomData;

use crate::atom::Atom;
use crate::primitive::Primitive;
use crate::range::{PackedField, PackedLayout};

/// The marker of `T` a path holds in place of a value: invariant in `T`, and `Send` and `Sync`
/// whatever `T` is.
type Invariant<T> = PhantomData<fn(T) -> T>;

/// The field at `INDEX`, in declaration order, of the packed value `C`, a `V`, as a path.
///
/// `#[derive(Atom)]` makes it a [`FieldPath`] for each field of a packed struct, indexed as core's
/// `field_of!` indexes a field, and names it in the type of the place its projection lends:
/// `&AtomicField<Field<Quote, 2, bool>>`. A path is a type alone: only a projection builds a place
/// of one, so the projection, which keeps each field's visibility, is the way to a field.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] names no path: its projection lends each field's place.
///
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-one-field
// Clone and Copy are derived: every container and value is `Copy`. Debug is written out, so it
// names the field rather than its marker.
#[derive(Clone, Copy)]
pub struct Field<C, const INDEX: u32, V>(Invariant<(C, V)>);

impl<C, const INDEX: u32, V> fmt::Debug for Field<C, INDEX, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Field<{}, {INDEX}, {}>", type_name::<C>(), type_name::<V>())
    }
}

/// The path to the field `B` of the value at the path `A`: a field of a field.
///
/// # Examples
/// In [`#[derive(Atom)]`'s example][derive], `ORDER.fields().flags.fields().halted`, a field of a
/// field, is a place of `Then<Field<Order, 0, Flags>, Field<Flags, 0, bool>>`.
///
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-a-field-through-generic-code
// As `Field`'s.
#[derive(Clone, Copy)]
pub struct Then<A, B>(Invariant<(A, B)>);

impl<A, B> fmt::Debug for Then<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Then<{}, {}>", type_name::<A>(), type_name::<B>())
    }
}

/// The whole value of an atomic of `C`, as a path: the root that [`Atomic::fields`] extends.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] projects an atomic's whole value, at this path, onto its
/// fields, each at its own [`Field`].
///
/// [`Atomic::fields`]: crate::Atomic::fields
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-one-field
// Whole's rule: no place of it reaches code but `ProjectFields::project`. Its bits are every bit
// of the repr, more than `FieldBitwise` and `FieldAdd` promise decode, so a place of it would
// `not` a `UInt<u8, 4>` to a repr past its 4 bits. It has no constructor, and `Atomic::fields`
// lends its one place to the projection alone, whose promise is to lend no place but its fields'.
#[derive(Clone, Copy)]
pub struct Whole<C>(Invariant<C>);

impl<C> fmt::Debug for Whole<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Whole<{}>", type_name::<C>())
    }
}

/// Where a field's bits lie in its container's repr, and how they read back.
///
/// In every repr of `Container` that decodes, the bits from `OFFSET` up to `OFFSET + WIDTH` hold a
/// repr of `Value` that decodes, read back zero-extended, or sign-extended where the field is two's
/// complement; and no other bit depends on the field, but, above a signed last field, the
/// container's bits that copy its sign. Writing any repr of `Value` that decodes there, its sign
/// copied so, and leaving every other bit as it is, leaves a repr of `Container` that decodes,
/// with that field alone changed: [`AtomicField`](super::AtomicField)'s operations rest on it. A
/// pointer repr's bits are its address, and each write keeps its provenance.
///
/// A field stored as a pointer, a pointer word's pointer, is not its bits: the bits from `OFFSET`
/// hold only the low `WIDTH` bits of its repr, its own tags, which a path to one of them goes on
/// from, and its value is read through the container's whole value.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] lends each field of a packed struct as a place, its path
/// a [`Field`].
///
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-one-field
#[diagnostic::on_unimplemented(
    message = "`{Self}` is no path to a field of a packed value",
    label = "expected a path to a field, such as `Field<Quote, 2, bool>`"
)]
pub impl(crate) trait FieldPath: Copy {
    /// The value the field lies in.
    type Container: Atom;
    /// The field's value.
    type Value: Atom;
    /// The field's lowest bit in the container's repr.
    const OFFSET: u32;
    /// How many bits the field takes.
    const WIDTH: u32;
    /// Whether the field is two's complement, sign-extended when read.
    #[doc(hidden)]
    const IS_SIGNED: bool;
    /// The bit after the last the field governs: `OFFSET + WIDTH`, or above it where the
    /// container's bits above its own width copy the field's top bit.
    #[doc(hidden)]
    const END: u32;
    /// The path to `B`, a field of this one's value: `Then<Self, B>`, or `B` itself from
    /// [`Whole`], so a field of an atomic's whole value has the path its own type names.
    #[doc(hidden)]
    type Join<B: FieldPath<Container = Self::Value>>: FieldPath<Container = Self::Container, Value = B::Value>;
    /// The field of `container`.
    #[doc(hidden)]
    fn field(container: Self::Container) -> Self::Value;
}

/// The path to the field `B` of the value at the path `P`: `Then<P, B>`, or `B` where `P` is
/// [`Whole`].
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] lends the field `written` of an atomic `Slot` as a place
/// of `Join<Whole<Slot>, Field<Slot, 0, bool>>`, which is `Field<Slot, 0, bool>`.
///
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-one-field
pub type Join<P, B> = <P as FieldPath>::Join<B>;

/// A field that ends at its container repr's top bit, so a carry out of it leaves the word as it
/// wraps the field: the one field an add changes alone.
///
/// A field of a generic container, or of a field, has no such path: add to it with `update`.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] counts a slot's readers in its top field with
/// `fetch_add`.
///
/// [derive]: https://docs.rs/atomix-rs/latest/atomix/derive.Atom.html#changing-one-field
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not end at its container's top bit",
    label = "a carry out of this field would reach the bits above it",
    note = "only the field that ends at the repr's top bit adds in place, its carry leaving the word: make it the last field, of a width that fills the repr",
    note = "a field of a generic struct, or of a field, adds in place nowhere: no constant knows where it ends",
    note = "to add to another field, call `update`"
)]
pub impl(crate) trait TopField: FieldPath {}

/// Where `#[derive(Atom)]` packs the field `INDEX` of a packed struct, a `V`, which makes
/// `Field<Self, INDEX, V>` a [`FieldPath`].
///
/// The derive implements it in the struct's crate, which cannot implement `FieldPath` for
/// `Field<Self, INDEX, V>`, a type of atomix. The field's type is a parameter, not an associated
/// type, so the impl is as visible as that type: a public struct may have a field of a private
/// type.
///
/// # Safety
/// `V` is the type of the field `INDEX`, and `PLACEMENT` is `PackedField::new(V::REPRS, offset)`,
/// so the field takes the bits [`FieldBitwise`](crate::FieldBitwise) and
/// [`FieldAdd`](crate::FieldAdd) speak for; `to_repr` packs that field there, each other field in
/// bits of its own, and extends the bits above the value's width as `LAYOUT` does; a repr decodes
/// wherever each field's bits decode as its type and the bits above extend the layout; `Reach` is
/// `Reach<true>` only where the field ends at the repr's top bit; and `field` returns the field.
///
/// Where `Self`'s repr is a pointer, a pointer word's, each field lies below `Self::TAG_WIDTH`: its
/// tag fields above its pointer field's own tags, and its `LAYOUT` is one whose top field is the
/// pointer, above the tags, so no tag reaches the top. Its pointer field, `V` stored as a pointer,
/// takes the low bits of `Self`'s repr that are the low bits of `V`'s, below `V::TAG_WIDTH`, where
/// `V`'s own fields lie, at offset 0. Where it is a double word, its tag fields lie so in its first
/// pointer's low bits, or in an integer word from bit 64, whose `LAYOUT` is theirs; and a second
/// pointer field takes the low bits of the second word that hold its own tags, at bit 64.
#[doc(hidden)]
#[expect(unsafe_code, reason = "`FieldPath` for `Field` trusts the placement")]
pub unsafe trait HasPackedField<const INDEX: u32, V: Atom>: Atom {
    /// Where the field lies.
    const PLACEMENT: PackedField;
    /// How the value lays out every field.
    const LAYOUT: PackedLayout;
    /// `Reach<true>` where the field ends at the repr's top bit, else `Reach<false>`.
    type Reach;
    /// The field's value.
    fn field(self) -> V;
}

/// Whether a field ends at its container repr's top bit, as a type, which a constant the derive
/// computes selects.
#[doc(hidden)]
#[derive(Debug)]
pub struct Reach<const TOP: bool>;

/// A field's [`Reach`] at its container repr's top bit: `Reach<true>` alone.
#[doc(hidden)]
pub impl(crate) trait ReachesTop {}

impl ReachesTop for Reach<true> {}

// Each path keeps `FieldPath`'s promise.
//
// Every bit of the repr is the value's own repr, read back as it is.
impl<C: Atom> FieldPath for Whole<C> {
    type Container = C;
    type Value = C;
    const OFFSET: u32 = 0;
    const WIDTH: u32 = C::Repr::BITS;
    const IS_SIGNED: bool = false;
    const END: u32 = C::Repr::BITS;
    type Join<B: FieldPath<Container = C>> = B;
    #[inline]
    fn field(container: C) -> C {
        container
    }
}

// `HasPackedField` places the field where `to_repr` packs it and `from_repr` reads it back, in the
// bits its type's `REPRS` takes, its own: any repr of its type that decodes, written there, decodes
// as the container with that field changed, since `from_repr` decodes each field alone and checks
// only the bits above the width, which extend the last field of any bits, which `END` includes
// where it is signed.
impl<C: HasPackedField<INDEX, V>, const INDEX: u32, V: Atom> FieldPath for Field<C, INDEX, V> {
    type Container = C;
    type Value = V;
    const OFFSET: u32 = C::PLACEMENT.offset();
    const WIDTH: u32 = C::PLACEMENT.layout().width();
    const IS_SIGNED: bool = C::PLACEMENT.layout().is_signed();
    const END: u32 = if C::LAYOUT.is_extended_by(C::PLACEMENT, C::Repr::BITS) {
        C::Repr::BITS
    } else {
        C::PLACEMENT.next_offset()
    };
    type Join<B: FieldPath<Container = Self::Value>> = Then<Self, B>;
    #[inline]
    fn field(container: C) -> V {
        <C as HasPackedField<INDEX, V>>::field(container)
    }
}

// `B`'s container is `A`'s value, whose repr's low `A::WIDTH` bits `A` stores, each of its fields
// among them, so `B`'s bits lie at `A::OFFSET + B::OFFSET`; writing them keeps `A`'s value
// decoding, by `B`'s promise, so the container too, by `A`'s. `B` governs no more of `A`'s value
// than `A` stores, except that where the container's bits above `A` copy `A`'s top bit, which is
// `B`'s where `B` is signed and ends at `A`'s width, it governs those too.
impl<A: FieldPath, B: FieldPath<Container = A::Value>> FieldPath for Then<A, B> {
    type Container = A::Container;
    type Value = B::Value;
    const OFFSET: u32 = A::OFFSET.saturating_add(B::OFFSET);
    const WIDTH: u32 = B::WIDTH;
    const IS_SIGNED: bool = B::IS_SIGNED;
    const END: u32 = if A::END > A::OFFSET.saturating_add(A::WIDTH)
        && B::IS_SIGNED
        && B::OFFSET.saturating_add(B::WIDTH) == A::WIDTH
    {
        A::END
    } else if B::END < A::WIDTH {
        A::OFFSET.saturating_add(B::END)
    } else {
        A::OFFSET.saturating_add(A::WIDTH)
    };
    type Join<N: FieldPath<Container = Self::Value>> = Then<Self, N>;
    #[inline]
    fn field(container: A::Container) -> B::Value {
        B::field(A::field(container))
    }
}

// `do_not_recommend`, so a field below the top reports `TopField`'s message, not `ReachesTop`'s.
// The derive selects `Reach<true>` only where the field ends at the repr's top bit.
#[diagnostic::do_not_recommend]
impl<C: HasPackedField<INDEX, V>, const INDEX: u32, V: Atom> TopField for Field<C, INDEX, V> where
    C::Reach: ReachesTop
{
}
