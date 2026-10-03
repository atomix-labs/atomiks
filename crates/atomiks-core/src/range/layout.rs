//! [`FieldLayout`], how a packed value stores a field, and [`NicheLayout`], where an enum puts its
//! unit variants beside its one payload.

use super::{ReprRange, Span, bit_length, mask, sign_extend};
use crate::primitive::Primitive;

/// How a packed value stores a field: the low `width` bits of its repr, read back zero-extended, or
/// sign-extended where `signed`, as [`ReprRange::field_layout`] picks them.
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
    pub const fn width(self) -> u32 {
        self.width
    }

    /// How a packed value stores itself, whose top field, the last of nonzero width, is stored in
    /// `top` at `offset`: its bits run up to the field's end, and extend as the field's do.
    ///
    /// Each repr of the value's [`ReprRange::from_top_field`] is one it [`holds`](Self::holds).
    #[inline]
    #[must_use]
    pub const fn from_top_field(top: Self, offset: u32) -> Self {
        Self { width: offset.saturating_add(top.width), signed: top.signed }
    }

    /// The field's bits in a packed value: the low `width` of `bits`, a repr's unsigned bits, moved
    /// up to `offset`.
    #[inline]
    #[must_use]
    pub const fn pack(self, bits: u128, offset: u32) -> u128 {
        (bits & mask(self.width)).unbounded_shl(offset)
    }

    /// The field at `offset` in `bits`, read back as the unsigned bits of its repr, a primitive
    /// `repr_width` bits wide: zero-extended, or sign-extended where signed.
    #[inline]
    #[must_use]
    pub const fn unpack(self, bits: u128, offset: u32, repr_width: u32) -> u128 {
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
    pub const fn holds(self, bits: u128, repr_width: u32) -> bool {
        self.unpack(bits, 0, repr_width) == bits
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
