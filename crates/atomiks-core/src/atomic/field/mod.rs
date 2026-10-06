//! One field of an atomic packed value, changed alone: [`AtomicField`], the place a
//! [`FieldPath`] names, and the projection that lends one per field.

#[cfg(opaque_bit_position)]
use core::arch::asm;
use core::marker::PhantomData;
use core::{fmt, ptr};

pub use self::path::{Field, FieldPath, Join, Then, TopField, Whole};
#[doc(hidden)]
pub use self::path::{HasPackedField, Reach};
use super::Atomic;
use crate::atom::{Atom, FieldAdd, FieldBitwise};
use crate::ordering::{LoadOrdering, Relaxed, RmwOrdering, StoreOrdering};
use crate::primitive::{
    BitTest, CellAccess, ExactBits, FetchAdd, FetchBitwise, Load, MaskBitwise, Primitive,
};
use crate::range::{mask, sign_extend};

mod path;

/// A packed value whose atomic projects onto its fields, each an [`AtomicField`].
///
/// `#[derive(Atom)]` implements it for a packed struct, `Quote`, and writes its projection,
/// `QuoteFields<'a, P>`, the struct of one `&'a AtomicField` per field, each of the field's own
/// visibility, that [`Atomic::fields`] and [`AtomicField::fields`] lend.
///
/// # Safety
/// The projection lends the place of each field of `Self`, at its path from the place projected,
/// each no more visible than its field, and no other place: never the place projected, whose path
/// at an atomic's root, [`Whole`], spans every bit of the repr, more than the field promises,
/// [`FieldBitwise`] and [`FieldAdd`], say decode.
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] projects an atomic ring buffer slot onto its fields.
///
/// [derive]: https://docs.rs/atomiks/latest/atomiks/derive.Atom.html#changing-one-field
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no fields an atomic projects onto",
    label = "`fields` needs a packed struct that derives `Atom`",
    note = "`#[derive(Atom)]` projects a struct of several fields: an enum, a newtype or a zero-width struct has none"
)]
#[expect(unsafe_code, reason = "the field operations trust the places the projection lends")]
pub const unsafe trait ProjectFields: Atom {
    /// The fields of the place at the path `P`, whose value is `Self`.
    type Fields<'a, P: FieldPath<Value = Self> + 'a>
    where
        Self: 'a;
    /// The fields of `place`.
    #[doc(hidden)]
    fn project<P: FieldPath<Value = Self>>(place: &AtomicField<P>) -> Self::Fields<'_, P>;
}

/// The repr of the container of the field `P`.
type ContainerRepr<P> = <<P as FieldPath>::Container as Atom>::Repr;

/// The mask of the repr of the container of the field `P`.
type ContainerMask<P> = <ContainerRepr<P> as MaskBitwise>::Mask;

/// The bits `bits` of a repr of `P`'s value, packed in their place in its container's repr: moved
/// to its offset, and, where signed, extended up to the bits it governs.
#[inline]
const fn pack<P: FieldPath>(bits: u128) -> u128 {
    let bits = if P::IS_SIGNED {
        sign_extend(bits, P::WIDTH).cast_unsigned()
    } else {
        bits & mask(P::WIDTH)
    };
    bits.unbounded_shl(P::OFFSET) & mask(P::END)
}

/// The bits the field `P` governs in its container's repr.
#[inline]
const fn governed<P: FieldPath>() -> u128 {
    mask(P::END) & !mask(P::OFFSET)
}

/// The field `P` of `bits`, a repr of its container's, unpacked as the unsigned bits of its value's
/// repr.
#[inline]
const fn unpack<P: FieldPath>(bits: u128) -> u128 {
    let field = bits.unbounded_shr(P::OFFSET) & mask(P::WIDTH);
    if P::IS_SIGNED {
        sign_extend(field, P::WIDTH).cast_unsigned()
            & mask(<<P::Value as Atom>::Repr as Primitive>::BITS)
    } else {
        field
    }
}

/// The field `P` of `bits`, a repr of its container's, replaced by `value`, a repr of its value's.
#[inline]
const fn replace<P: FieldPath>(bits: u128, value: u128) -> u128 {
    (bits & !governed::<P>()) | pack::<P>(value)
}

/// The field `P` of the container's repr `bits`.
///
/// # Safety
/// `bits` are a repr of `P`'s container that decodes.
#[expect(unsafe_code, reason = "decodes a field of a container repr that decodes")]
#[inline]
unsafe fn decode<P: FieldPath>(bits: u128) -> P::Value {
    // SAFETY: by `FieldPath`'s promise, the field of a repr that decodes is a repr that decodes,
    // and the caller's repr decodes.
    unsafe { P::Value::from_repr_unchecked(Primitive::from_bits(unpack::<P>(bits))) }
}

/// The field `P` of `repr`, a repr of its container's: its bits read back, or, for a field stored
/// as a pointer, whose bits do not hold its provenance, the field of the value `repr` decodes as.
///
/// # Safety
/// `repr` decodes as `P`'s container.
#[expect(unsafe_code, reason = "decodes a field of a container repr that decodes")]
#[inline]
unsafe fn decode_repr<P: FieldPath>(repr: ContainerRepr<P>) -> P::Value {
    if <<P::Value as Atom>::Repr as Primitive>::IS_BITS_EXACT {
        // SAFETY: the caller's repr decodes.
        unsafe { decode::<P>(repr.packed_bits()) }
    } else {
        // SAFETY: the caller's repr decodes.
        P::field(unsafe { P::Container::from_repr_unchecked(repr) })
    }
}

/// `value` packed in the place of the field `P`, every other bit clear: the operand of an or and
/// of a xor.
#[inline]
fn operand<P: FieldPath>(value: P::Value) -> ContainerMask<P>
where
    P::Value: FieldBitwise,
    ContainerRepr<P>: MaskBitwise,
{
    ContainerMask::<P>::from_bits(pack::<P>(value.to_repr().to_bits()))
}

/// `value` packed in the place of the field `P`, every bit it does not govern set: the operand of
/// an and, which so leaves every other field as it is, one whose zero does not decode too.
#[inline]
fn and_operand<P: FieldPath>(value: P::Value) -> ContainerMask<P>
where
    P::Value: FieldBitwise,
    ContainerRepr<P>: MaskBitwise,
{
    ContainerMask::<P>::from_bits(pack::<P>(value.to_repr().to_bits()) | !governed::<P>())
}

/// Every bit the field `P` governs: the operand of a not, as a xor.
#[inline]
fn not_operand<P: FieldPath>() -> ContainerMask<P>
where
    ContainerRepr<P>: MaskBitwise,
{
    ContainerMask::<P>::from_bits(governed::<P>())
}

/// The mask of the bit `P` lies in, a one-bit field: its offset.
///
/// `x86_64` lowers `fetch_or(bit) & bit != 0` to `lock bts`, but LLVM first folds the test of a
/// repr's lowest bit into a truncation, and of its top bit into a sign test, either of which
/// `x86_64` leaves a compare-exchange loop; so those two positions go through an empty `asm!`,
/// which hides the value, and the test stays a `lock bts` with the position in a register. Miri
/// runs no `asm!`.
#[inline]
fn bit<P: FieldPath>() -> ContainerMask<P>
where
    ContainerRepr<P>: MaskBitwise,
{
    #[cfg(opaque_bit_position)]
    if P::OFFSET == 0 || P::OFFSET.wrapping_add(1) == ContainerRepr::<P>::BITS {
        let mut position = P::OFFSET;
        // SAFETY: the template is a comment, so the block emits no instruction: it reads and
        // writes no memory, and leaves `position`, its one register, and the flags as they were.
        #[expect(unsafe_code, reason = "hides the position in a register; `black_box` spills it")]
        unsafe {
            asm!(
                "/* {0:e} */",
                inout(reg) position,
                options(pure, nomem, nostack, preserves_flags)
            );
        }
        return ContainerRepr::<P>::bit(position);
    }
    ContainerRepr::<P>::bit(P::OFFSET)
}

/// Whether the bit of `mask` is set in `value`.
#[inline]
fn has_bit<R: MaskBitwise>(value: R, mask: R::Mask) -> bool {
    value.packed_bits() & mask.to_bits() != 0
}

impl<C: Atom> Atomic<C> {
    /// The fields of the value, each a place of its own: `quote.fields().live`.
    ///
    /// `F` is the projection `#[derive(Atom)]` writes beside the value: `QuoteFields`.
    ///
    /// # Examples
    /// [`#[derive(Atom)]`'s example][derive] claims a ring buffer slot through its fields.
    ///
    /// [derive]: https://docs.rs/atomiks/latest/atomiks/derive.Atom.html#changing-one-field
    // `F`, rather than the projection in the return type, which rustc would resolve before the
    // bound, so a value without fields reports `ProjectFields`' message.
    #[expect(unsafe_code, reason = "lends the place of the whole value to the projection")]
    #[inline]
    #[must_use]
    pub const fn fields<'a, F>(&'a self) -> F
    where
        C: [const] ProjectFields<Fields<'a, Whole<C>> = F>,
    {
        // SAFETY: the place of the whole value goes to the projection alone, which, by
        // `ProjectFields`' promise, lends only its fields' places, each no more visible than its
        // field.
        C::project(unsafe { AtomicField::<Whole<C>>::from_atomic(self) })
    }
}

/// The place of the field `INDEX`, a `V`, of the value at `place`: each place of the projection
/// `#[derive(Atom)]` writes.
///
/// # Safety
/// The place reaches only code the field is visible to: a private field may carry an invariant its
/// owner's `unsafe` code relies on, which no other module may then break.
#[doc(hidden)]
#[expect(unsafe_code, reason = "lends a field's place, which only code it is visible to may hold")]
#[inline]
#[must_use]
pub const unsafe fn project_field<P: FieldPath, V: Atom, const INDEX: u32>(
    place: &AtomicField<P>,
) -> &AtomicField<Join<P, Field<P::Value, INDEX, V>>>
where
    P::Value: HasPackedField<INDEX, V>,
{
    // SAFETY: the place reaches only code the field is visible to, by the caller's contract, and
    // its path ends at a `Field`, not at `Whole`.
    unsafe { AtomicField::from_atomic(&place.atomic) }
}

/// A field of an atomic packed value, as a place of its own: what [`Atomic::fields`] lends.
///
/// Each operation is one atomic instruction on the atomic's whole word, as [`Atomic`]'s are, whose
/// operand changes no other field, but [`update`](Self::update) and
/// [`try_update`](Self::try_update), compare-exchange loops, and [`load_rmw`](Self::load_rmw), one
/// compare-exchange, which writes. Which a field has depends on its value, and on what its
/// container's repr runs without a loop:
///
/// - **Every field**: [`load`](Self::load) where the repr has [`Load`], else
///   [`load_rmw`](Self::load_rmw), and `update` and `try_update`: a 128-bit container's field has
///   these alone.
/// - **A `bool`**: [`set`](Self::set), [`clear`](Self::clear), [`toggle`](Self::toggle) and
///   [`store`](Self::store), and, where the repr has [`BitTest`],
///   [`test_and_set`](Self::test_and_set), [`test_and_clear`](Self::test_and_clear) and
///   [`test_and_toggle`](Self::test_and_toggle), which return the bit before.
/// - **A value with [`FieldBitwise`]**: [`and`](Self::and), [`or`](Self::or), [`xor`](Self::xor)
///   and [`not`](Self::not), where the repr has [`MaskBitwise`], and their `fetch_` forms, where
///   [`FetchBitwise`] too.
/// - **A value with [`FieldAdd`], in the field that ends at the repr's top bit ([`TopField`])**:
///   [`fetch_add`](Self::fetch_add) and [`fetch_sub`](Self::fetch_sub), where it has [`FetchAdd`].
///
/// Each `fetch_` form returns the container before: the one snapshot of every field that the
/// instruction reads. A pointer word's tag field has each operation its value gives it but the add,
/// since the pointer lies above it, and each keeps the pointer's provenance. Its pointer field has
/// only [`load`](Self::load), which reads it through the word, and, where the pointer is a word
/// too, [`fields`](Self::fields).
///
/// # Examples
/// [`#[derive(Atom)]`'s example][derive] claims a ring buffer slot with one field's
/// `test_and_set`, and counts its readers with another's `fetch_add`.
///
/// [derive]: https://docs.rs/atomiks/latest/atomiks/derive.Atom.html#changing-one-field
#[repr(transparent)]
pub struct AtomicField<P: FieldPath> {
    /// The path from the atomic's repr to the field.
    path: PhantomData<P>,
    /// The atomic the field lies in.
    atomic: Atomic<P::Container>,
}

impl<P: FieldPath> AtomicField<P> {
    /// The place of the field `P` of `atomic`.
    ///
    /// # Safety
    /// The place reaches only code the field is visible to; and, where `P` is [`Whole`], whose
    /// bits are more than [`FieldBitwise`] and [`FieldAdd`] promise decode, only the projection.
    #[expect(unsafe_code, reason = "a reference to a transparent wrapper of the referent")]
    #[inline]
    const unsafe fn from_atomic(atomic: &Atomic<P::Container>) -> &Self {
        // SAFETY: `AtomicField<P>` is `repr(transparent)` over `Atomic<P::Container>`, beside a
        // zero-sized marker, so the cast keeps the layout, the provenance and the referent's
        // `UnsafeCell`, and the reference lives as long as the atomic's.
        unsafe { &*ptr::from_ref(atomic).cast::<Self>() }
    }

    /// The primitive's cell.
    #[inline]
    const fn cell(&self) -> &<ContainerRepr<P> as CellAccess>::Cell {
        self.atomic.primitive_cell()
    }

    /// The fields of this field's value, each a place of its own:
    /// `outer.fields().inner.fields().ready`.
    ///
    /// `F` is the projection `#[derive(Atom)]` writes beside the value, as for
    /// [`Atomic::fields`].
    ///
    /// # Examples
    /// [`#[derive(Atom)]`'s example][derive] halts an order through a field of its field.
    ///
    /// [derive]: https://docs.rs/atomiks/latest/atomiks/derive.Atom.html#changing-a-field-through-generic-code
    // `F`, as in `Atomic::fields`.
    #[inline]
    #[must_use]
    pub const fn fields<'a, F>(&'a self) -> F
    where
        P::Value: [const] ProjectFields<Fields<'a, P> = F>,
    {
        P::Value::project(self)
    }

    /// Reads the field: a load of the whole word, and the field's bits read back, or, for a
    /// pointer word's pointer, the word decoded.
    #[expect(unsafe_code, reason = "decodes a field of a repr read from the cell")]
    #[inline]
    pub fn load<O: LoadOrdering>(&self, order: O) -> P::Value
    where
        ContainerRepr<P>: Load,
    {
        let _ = order;
        let repr = ContainerRepr::<P>::load(self.cell(), O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { decode_repr::<P>(repr) }
    }

    /// Reads the field with a compare-exchange of the whole word, for a container whose repr has
    /// no [`Load`].
    ///
    /// As [`Atomic::load_rmw`]: the exchange takes the cache line exclusive and writes it, so it
    /// faults on read-only memory.
    #[expect(unsafe_code, reason = "decodes a field of a repr read from the cell")]
    #[inline]
    pub fn load_rmw<O: LoadOrdering>(&self, order: O) -> P::Value
    where
        ContainerRepr<P>: ExactBits,
    {
        let repr = self.atomic.load_rmw_repr(order);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { decode::<P>(repr.to_bits()) }
    }

    /// Replaces the field with `f` of it, and returns the container before.
    ///
    /// A compare-exchange loop on the whole word, as [`Atomic::update`] is, which leaves every
    /// other field as it finds it.
    #[expect(unsafe_code, reason = "decodes reprs read from the cell")]
    #[inline]
    pub fn update<S: RmwOrdering, F: LoadOrdering, U: FnMut(P::Value) -> P::Value>(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> P::Container
    where
        <P::Value as Atom>::Repr: ExactBits,
    {
        let replace_field = |seen: ContainerRepr<P>| {
            let bits = seen.packed_bits();
            // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
            let value = f(unsafe { decode::<P>(bits) }).to_repr().to_bits();
            Some(seen.with_packed_bits(replace::<P>(bits, value)))
        };
        // SAFETY: each repr `replace_field` returns is one read from the cell, which decodes, by
        // `Atomic`'s field INVARIANT, with the field replaced by a repr of a value of it, so it
        // decodes, by `FieldPath`'s promise.
        let replaced =
            unsafe { self.atomic.try_update_repr(set_order, fetch_order, replace_field) };
        match replaced {
            // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
            Ok(before) | Err(before) => unsafe { P::Container::from_repr_unchecked(before) },
        }
    }

    /// As [`update`](Self::update), stopping without a write when `f` returns `None`.
    ///
    /// # Errors
    /// The container seen, when `f` declined its field.
    #[expect(unsafe_code, reason = "decodes reprs read from the cell")]
    #[inline]
    pub fn try_update<S: RmwOrdering, F: LoadOrdering, U: FnMut(P::Value) -> Option<P::Value>>(
        &self, set_order: S, fetch_order: F, mut f: U,
    ) -> Result<P::Container, P::Container>
    where
        <P::Value as Atom>::Repr: ExactBits,
    {
        let replace_field = |seen: ContainerRepr<P>| {
            let bits = seen.packed_bits();
            // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
            let value = f(unsafe { decode::<P>(bits) })?.to_repr().to_bits();
            Some(seen.with_packed_bits(replace::<P>(bits, value)))
        };
        // SAFETY: as in `update`.
        let replaced =
            unsafe { self.atomic.try_update_repr(set_order, fetch_order, replace_field) };
        match replaced {
            // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
            Ok(before) => Ok(unsafe { P::Container::from_repr_unchecked(before) }),
            // SAFETY: as above.
            Err(seen) => Err(unsafe { P::Container::from_repr_unchecked(seen) }),
        }
    }

    /// Turns the bit on: `lock or` on `x86_64`, `ldset` on `aarch64`.
    #[inline]
    pub fn set<O: RmwOrdering>(&self, order: O)
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: MaskBitwise,
    {
        self.or(true, order);
    }

    /// Turns the bit off: `lock and` on `x86_64`, `ldclr` on `aarch64`.
    #[inline]
    pub fn clear<O: RmwOrdering>(&self, order: O)
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: MaskBitwise,
    {
        self.and(false, order);
    }

    /// Inverts the bit: `lock xor` on `x86_64`, `ldeor` on `aarch64`.
    #[inline]
    pub fn toggle<O: RmwOrdering>(&self, order: O)
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: MaskBitwise,
    {
        self.xor(true, order);
    }

    /// Writes `value` to the bit: [`set`](Self::set) or [`clear`](Self::clear).
    ///
    /// A plain store of the field alone would race the other fields' read-modify-writes, so this
    /// is a read-modify-write of the whole word too, taking a store's orderings.
    #[inline]
    pub fn store<O: StoreOrdering>(&self, value: bool, order: O)
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: MaskBitwise,
    {
        if value { self.set(order) } else { self.clear(order) }
    }

    /// Turns the bit on, and returns it before: `lock bts` on `x86_64`, `ldset` on `aarch64`.
    ///
    /// `x86_64` has no 8-bit `lock bts`, so a container of 8 bits has it on `aarch64` alone.
    #[must_use = "to discard the bit before, call `set`, which every target has"]
    #[inline]
    pub fn test_and_set<O: RmwOrdering>(&self, order: O) -> bool
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: BitTest,
    {
        let _ = order;
        self.test_bit(|cell, bit| ContainerRepr::<P>::fetch_or_mask(cell, bit, O::CORE))
    }

    /// Turns the bit off, and returns it before: `lock btr` on `x86_64`, `ldclr` on `aarch64`.
    ///
    /// As [`test_and_set`](Self::test_and_set), on the containers it takes.
    #[must_use = "to discard the bit before, call `clear`, which every target has"]
    #[inline]
    pub fn test_and_clear<O: RmwOrdering>(&self, order: O) -> bool
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: BitTest,
    {
        let _ = order;
        self.test_bit(|cell, bit| {
            let others = ContainerMask::<P>::from_bits(!bit.to_bits());
            ContainerRepr::<P>::fetch_and_mask(cell, others, O::CORE)
        })
    }

    /// Inverts the bit, and returns it before: `lock btc` on `x86_64`, `ldeor` on `aarch64`.
    ///
    /// As [`test_and_set`](Self::test_and_set), on the containers it takes.
    #[must_use = "to discard the bit before, call `toggle`, which every target has"]
    #[inline]
    pub fn test_and_toggle<O: RmwOrdering>(&self, order: O) -> bool
    where
        P: FieldPath<Value = bool>,
        ContainerRepr<P>: BitTest,
    {
        let _ = order;
        self.test_bit(|cell, bit| ContainerRepr::<P>::fetch_xor_mask(cell, bit, O::CORE))
    }

    /// Whether the bit was set in the container before `change`, which changes it, given the cell
    /// and the bit's mask.
    #[inline]
    fn test_bit(
        &self,
        change: impl FnOnce(
            &<ContainerRepr<P> as CellAccess>::Cell,
            ContainerMask<P>,
        ) -> ContainerRepr<P>,
    ) -> bool
    where
        ContainerRepr<P>: BitTest,
    {
        let bit = bit::<P>();
        has_bit(change(self.cell(), bit), bit)
    }

    /// Applies `& value` to the field: [`fetch_and`](Self::fetch_and) with the container before
    /// discarded, which every target runs as one instruction, as [`Atomic::and`] says.
    ///
    /// Its operand keeps every other bit, so a field whose zero does not decode stays as it is.
    #[inline]
    pub fn and<O: RmwOrdering>(&self, value: P::Value, order: O)
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise,
    {
        let _ = order;
        ContainerRepr::<P>::fetch_and_mask(self.cell(), and_operand::<P>(value), O::CORE);
    }

    /// Applies `| value` to the field: [`fetch_or`](Self::fetch_or) with the container before
    /// discarded, at [`and`](Self::and)'s cost.
    #[inline]
    pub fn or<O: RmwOrdering>(&self, value: P::Value, order: O)
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise,
    {
        let _ = order;
        ContainerRepr::<P>::fetch_or_mask(self.cell(), operand::<P>(value), O::CORE);
    }

    /// Applies `^ value` to the field: [`fetch_xor`](Self::fetch_xor) with the container before
    /// discarded, at [`and`](Self::and)'s cost.
    #[inline]
    pub fn xor<O: RmwOrdering>(&self, value: P::Value, order: O)
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise,
    {
        let _ = order;
        ContainerRepr::<P>::fetch_xor_mask(self.cell(), operand::<P>(value), O::CORE);
    }

    /// Inverts every bit of the field: [`fetch_not`](Self::fetch_not) with the container before
    /// discarded, at [`and`](Self::and)'s cost.
    #[inline]
    pub fn not<O: RmwOrdering>(&self, order: O)
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise,
    {
        let _ = order;
        ContainerRepr::<P>::fetch_xor_mask(self.cell(), not_operand::<P>(), O::CORE);
    }

    /// Applies `& value` to the field, and returns the container before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the container before, call `and`, which every target has"]
    #[inline]
    pub fn fetch_and<O: RmwOrdering>(&self, value: P::Value, order: O) -> P::Container
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise + FetchBitwise,
    {
        let _ = order;
        let before =
            ContainerRepr::<P>::fetch_and_mask(self.cell(), and_operand::<P>(value), O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { P::Container::from_repr_unchecked(before) }
    }

    /// Applies `| value` to the field, and returns the container before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the container before, call `or`, which every target has"]
    #[inline]
    pub fn fetch_or<O: RmwOrdering>(&self, value: P::Value, order: O) -> P::Container
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise + FetchBitwise,
    {
        let _ = order;
        let before = ContainerRepr::<P>::fetch_or_mask(self.cell(), operand::<P>(value), O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { P::Container::from_repr_unchecked(before) }
    }

    /// Applies `^ value` to the field, and returns the container before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the container before, call `xor`, which every target has"]
    #[inline]
    pub fn fetch_xor<O: RmwOrdering>(&self, value: P::Value, order: O) -> P::Container
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise + FetchBitwise,
    {
        let _ = order;
        let before = ContainerRepr::<P>::fetch_xor_mask(self.cell(), operand::<P>(value), O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { P::Container::from_repr_unchecked(before) }
    }

    /// Inverts every bit of the field, and returns the container before.
    #[doc(cfg(target_arch = "aarch64"))]
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[must_use = "to discard the container before, call `not`, which every target has"]
    #[inline]
    pub fn fetch_not<O: RmwOrdering>(&self, order: O) -> P::Container
    where
        P::Value: FieldBitwise,
        ContainerRepr<P>: MaskBitwise + FetchBitwise,
    {
        let _ = order;
        let before = ContainerRepr::<P>::fetch_xor_mask(self.cell(), not_operand::<P>(), O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { P::Container::from_repr_unchecked(before) }
    }

    /// Adds `delta` to the field, wrapping, and returns the container before: `lock xadd` on
    /// `x86_64`, `ldadd` on `aarch64`.
    ///
    /// The field ends at the repr's top bit ([`TopField`]), so the carry out of it leaves the word.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn fetch_add<O: RmwOrdering>(
        &self, delta: <P::Value as Atom>::Repr, order: O,
    ) -> P::Container
    where
        P: TopField,
        P::Value: FieldAdd,
        ContainerRepr<P>: FetchAdd,
    {
        let _ = order;
        let delta = ContainerRepr::<P>::from_bits(pack::<P>(delta.to_bits()));
        let before = ContainerRepr::<P>::fetch_add(self.cell(), delta, O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { P::Container::from_repr_unchecked(before) }
    }

    /// Subtracts `delta` from the field, wrapping, and returns the container before: `lock xadd`
    /// on `x86_64`, `ldadd` on `aarch64`, after a negation.
    ///
    /// As [`fetch_add`](Self::fetch_add), the borrow leaving the word.
    #[expect(unsafe_code, reason = "decodes a repr read from the cell")]
    #[inline]
    pub fn fetch_sub<O: RmwOrdering>(
        &self, delta: <P::Value as Atom>::Repr, order: O,
    ) -> P::Container
    where
        P: TopField,
        P::Value: FieldAdd,
        ContainerRepr<P>: FetchAdd,
    {
        let _ = order;
        let delta = ContainerRepr::<P>::from_bits(pack::<P>(delta.to_bits()));
        let before = ContainerRepr::<P>::fetch_sub(self.cell(), delta, O::CORE);
        // SAFETY: by `Atomic`'s field INVARIANT, the repr read from the cell decodes.
        unsafe { P::Container::from_repr_unchecked(before) }
    }
}

impl<P: FieldPath> fmt::Debug for AtomicField<P>
where
    P::Value: fmt::Debug,
    ContainerRepr<P>: Load,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ORDERING: Relaxed, as `Atomic`'s `Debug`: printing publishes nothing and pairs with no
        // store.
        self.load(Relaxed).fmt(f)
    }
}
