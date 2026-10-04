//! [`FieldLayout`], how a packed value stores a field, [`PackedField`] and [`PackedLayout`], where
//! its fields lie and how it extends them, and [`NicheLayout`], where an enum puts its unit
//! variants beside its one payload.

use super::{ReprRange, Span, bit_length, mask, sign_extend};
use crate::primitive::{ExactBits, Primitive};

/// How a packed value stores a field: the low `width` bits of its repr, read back zero-extended, or
/// sign-extended where `signed`; unsigned, or two's complement where that is narrower, unsigned on
/// a tie.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldLayout {
    /// How many bits the field takes: at most its primitive's, and past 128 only for a packed
    /// value too wide for any repr, which the derive refuses by this width.
    pub(super) width: u32,
    /// Whether the field is two's complement, sign-extended when read.
    pub(super) signed: bool,
}

impl FieldLayout {
    /// How many bits the field takes.
    #[inline]
    #[must_use]
    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    /// How a packed value stores a field of the integers from `start` up to `end`, of any width to
    /// 128 bits: unsigned where none is negative and that is narrower, else two's complement.
    #[inline]
    #[must_use]
    pub(crate) const fn from_signed(start: i128, end: i128) -> Self {
        Span { start: start.cast_unsigned(), end: end.cast_unsigned(), width: u128::BITS }
            .field_layout()
    }

    /// How a packed value stores itself, whose top field, the last of nonzero width, is `top`: its
    /// bits run up to the field's end, and extend as the field's do.
    ///
    /// Each repr of the value's [`ReprRange::from_top_field`] is one it [`holds`](Self::holds).
    #[inline]
    #[must_use]
    pub(super) const fn from_top_field(top: PackedField) -> Self {
        Self { width: top.next_offset(), signed: top.layout.signed }
    }

    /// The field's bits in a packed value: the low `width` of `bits`, a repr's unsigned bits, moved
    /// up to `offset`.
    #[inline]
    #[must_use]
    pub(crate) const fn pack(self, bits: u128, offset: u32) -> u128 {
        (bits & mask(self.width)).unbounded_shl(offset)
    }

    /// The field at `offset` in `bits`, read back as the unsigned bits of its repr, a primitive
    /// `repr_width` bits wide: zero-extended, or sign-extended where signed.
    #[inline]
    #[must_use]
    pub(crate) const fn unpack(self, bits: u128, offset: u32, repr_width: u32) -> u128 {
        let field = bits.unbounded_shr(offset) & mask(self.width);
        if self.signed {
            sign_extend(field, self.width).cast_unsigned() & mask(repr_width)
        } else {
            field
        }
    }

    /// Whether `bits`, a repr's, a primitive `repr_width` bits wide, are as the field reads them
    /// back: those above its width zeros or, where signed, copies of its top bit. A packed value's
    /// canonical check.
    #[inline]
    #[must_use]
    pub(crate) const fn holds(self, bits: u128, repr_width: u32) -> bool {
        self.unpack(bits, 0, repr_width) == bits
    }
}

/// Where a packed value stores a field: the reprs it holds, in their [`FieldLayout`] at an offset,
/// as [`new`](Self::new) places it.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedField {
    /// The reprs the field holds, numbers of its primitive's width.
    pub(super) reprs: Span,
    /// How it stores them.
    pub(super) layout: FieldLayout,
    /// Where its lowest bit lies in the value.
    pub(super) offset: u32,
}

impl PackedField {
    /// The top field of a value whose every field takes no bits: none of its bits, at bit 0, and
    /// zero alone, which is that value's one repr.
    const EMPTY: Self = Self {
        reprs: Span { start: 0, end: 0, width: u8::BITS },
        layout: FieldLayout { width: 0, signed: false },
        offset: 0,
    };

    /// A field of `reprs`, its type's, whose lowest bit lies at `offset`: bit 0 for the first
    /// field, else the [`next_offset`](Self::next_offset) of the one declared before it.
    #[inline]
    #[must_use]
    pub const fn new<F: Primitive>(reprs: ReprRange<F>, offset: u32) -> Self {
        let reprs = reprs.span();
        Self { reprs, layout: reprs.field_layout(), offset }
    }

    /// How the field stores its reprs.
    #[inline]
    #[must_use]
    pub const fn layout(self) -> FieldLayout {
        self.layout
    }

    /// The bit after the field's last: where the field declared after it lies.
    #[inline]
    #[must_use]
    pub const fn next_offset(self) -> u32 {
        self.offset.saturating_add(self.layout.width)
    }

    /// The field's bits in the value: those of `bits`, its repr's unsigned bits, that its layout
    /// keeps, moved up to its offset.
    #[inline]
    #[must_use]
    pub const fn pack(self, bits: u128) -> u128 {
        self.layout.pack(bits, self.offset)
    }

    /// The field in `bits`, the value's, read back as its repr's unsigned bits.
    #[inline]
    #[must_use]
    pub const fn unpack(self, bits: u128) -> u128 {
        self.layout.unpack(bits, self.offset, self.reprs.width)
    }
}

/// How a packed value lays out its fields: by its top field, the last of nonzero width, which
/// decides the value's range and how its bits extend above its width.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedLayout {
    /// The top field.
    top: PackedField,
    /// How the value stores itself: its bits up to the top field's last, which they extend.
    layout: FieldLayout,
}

impl PackedLayout {
    /// The layout of a value of `fields`, placed from bit 0 in declaration order.
    #[inline]
    #[must_use]
    pub const fn new(fields: &[PackedField]) -> Self {
        // A field of no bits holds zero alone, so it changes neither the range nor the bits above.
        let mut top = PackedField::EMPTY;
        let mut rest = fields;
        while let [field, after @ ..] = rest {
            if field.layout.width != 0 {
                top = *field;
            }
            rest = after;
        }
        Self { top, layout: FieldLayout::from_top_field(top) }
    }

    /// How many bits the value takes: up to its top field's last.
    #[inline]
    #[must_use]
    pub const fn width(self) -> u32 {
        self.layout.width
    }

    /// The value's range, in its repr `R`, which its top field decides.
    #[inline]
    #[must_use]
    pub const fn range<R: Primitive>(self) -> ReprRange<R> {
        ReprRange::from_top_field(self.top)
    }

    /// The repr whose fields' bits are `bits`, its fields' packed: those bits, extended above the
    /// value's width as its top field is read back.
    #[inline]
    #[must_use]
    pub const fn repr<R: const ExactBits>(self, bits: u128) -> R {
        R::from_bits(self.layout.unpack(bits, 0, R::BITS))
    }

    /// The bits of `repr`, or `None` where those above the value's width do not extend its top
    /// field as [`repr`](Self::repr) extends it: the canonical check, refusing a repr that no
    /// value encodes to.
    #[inline]
    #[must_use]
    pub const fn canonical_bits<R: const ExactBits>(self, repr: R) -> Option<u128> {
        let bits = repr.to_bits();
        if self.layout.holds(bits, R::BITS) { Some(bits) } else { None }
    }
}

/// Where an enum puts its unit variants beside its one payload variant: in reprs beside the
/// payload's range, as `Option`'s `None` takes one, so the enum is no wider than a tag above the
/// payload's field would make it.
///
/// The units' reprs and the payload's are reprs of the payload's primitive; the enum's own bits
/// are their low [`field_layout`](Self::field_layout) bits, which that layout reads back to the
/// payload's primitive before they are compared with a unit's.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NicheLayout {
    /// The payload's reprs and the units', of the payload's primitive.
    pub(super) span: Span,
    /// The first unit's repr; each next unit takes the repr after it.
    pub(super) first_unit_bits: u128,
}

impl NicheLayout {
    /// The layout of `unit_count` unit variants beside a payload whose reprs lie in `payload`, or
    /// `None` where fewer than `unit_count` reprs lie outside it, or where a tag above its
    /// field, numbering the payload and each unit, makes a narrower enum.
    ///
    /// The units take the reprs below the payload's start, or above its end, whichever leaves the
    /// narrower field; on a tie, the side that puts a lone unit at zero, then the side that does
    /// not wrap, then below.
    #[inline]
    #[must_use]
    pub const fn new<P: Primitive>(unit_count: u128, payload: ReprRange<P>) -> Option<Self> {
        Self::beside(payload.span(), unit_count)
    }

    /// The layout of `unit_count` unit variants beside a payload whose reprs lie in `payload`, as
    /// [`new`](Self::new).
    #[inline]
    #[must_use]
    pub(super) const fn beside(payload: Span, unit_count: u128) -> Option<Self> {
        let Some((span, first_unit_bits)) = payload.grown_by(unit_count) else {
            return None;
        };
        let tagged = payload.field_layout().width.saturating_add(bit_length(unit_count));
        if span.field_layout().width > tagged {
            return None;
        }
        Some(Self { span: span.normalized(), first_unit_bits })
    }

    /// The repr of the unit variant that is `unit`th among the units, in declaration order, as the
    /// payload primitive's unsigned bits.
    #[inline]
    #[must_use]
    pub const fn unit_bits(self, unit: u128) -> u128 {
        self.first_unit_bits.wrapping_add(unit) & self.span.largest()
    }

    /// How the enum stores its bits, a unit's or the payload's: the field of the range they share.
    #[inline]
    #[must_use]
    pub const fn field_layout(self) -> FieldLayout {
        self.span.field_layout()
    }

    /// The enum's range, in its repr `R`.
    #[inline]
    #[must_use]
    pub const fn range<R: Primitive>(self) -> ReprRange<R> {
        let span = Span::from_top_field(self.span, 0, R::BITS);
        ReprRange::from_bounds(span.start, span.end)
    }
}
