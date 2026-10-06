//! [`FieldLayout`], how a packed value stores a field, [`PackedField`] and [`PackedLayout`], where
//! its fields lie and how it extends them, and [`EnumLayout`], how an enum with fields says which
//! variant a repr holds: by a tag, or by [`NicheLayout`], the reprs its unit variants take beside
//! its one variant with fields.

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

    /// Whether the field is two's complement, sign-extended when read.
    #[inline]
    #[must_use]
    pub(crate) const fn is_signed(self) -> bool {
        self.signed
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
        Self::from_span(reprs.span(), offset)
    }

    /// A field of the numbers of `reprs`, whose lowest bit lies at `offset`.
    #[inline]
    #[must_use]
    pub(super) const fn from_span(reprs: Span, offset: u32) -> Self {
        Self { reprs, layout: reprs.field_layout(), offset }
    }

    /// How the field stores its reprs.
    #[inline]
    #[must_use]
    pub const fn layout(self) -> FieldLayout {
        self.layout
    }

    /// Where its lowest bit lies in the value.
    #[inline]
    #[must_use]
    pub(crate) const fn offset(self) -> u32 {
        self.offset
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

    /// Whether each pattern of the field's bits reads back as one of its reprs.
    #[inline]
    #[must_use]
    const fn every_pattern_is_a_repr(self) -> bool {
        // The field's bits have one pattern for each repr exactly where the reprs left out of the
        // primitive's are those no pattern reads back as.
        self.reprs.spare_count() == self.reprs.largest().wrapping_sub(mask(self.layout.width))
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
        Self::from_top_field(top)
    }

    /// The layout of a value whose top field is `top`, beside any fields below it.
    #[inline]
    #[must_use]
    pub(super) const fn from_top_field(top: PackedField) -> Self {
        Self { top, layout: FieldLayout::from_top_field(top) }
    }

    /// How many bits the value takes: up to its top field's last.
    #[inline]
    #[must_use]
    pub const fn width(self) -> u32 {
        self.layout.width
    }

    /// Whether the value's bits above its width, up to `repr_width`, copy `field`'s top bit: where
    /// `field` is the last field of any bits, signed, and the repr is wider than the value.
    #[inline]
    #[must_use]
    pub(crate) const fn is_extended_by(self, field: PackedField, repr_width: u32) -> bool {
        field.layout.width != 0
            && field.offset == self.top.offset
            && field.layout.signed
            && self.layout.width < repr_width
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

/// How an enum with fields lays out its variants: each one's fields from bit 0, as a packed
/// value's, and a selector, the enum's top field, which says which variant a repr holds.
///
/// Tagged, the selector is a tag above the widest variant's fields, numbering each variant by its
/// discriminant, and the bits between a variant's fields and the tag are clear. Niche-filling,
/// where one variant alone has fields, the selector is that variant's top field, grown into the
/// reprs beside its range that the unit variants take, as `Option`'s `None` takes one, and the bits
/// below a unit's are clear.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnumLayout {
    /// The selector, alone, as a packed value's top field: it decides the enum's width, its range
    /// and how its bits extend.
    selector: PackedLayout,
    /// The niche the unit variants fill; `None` where the enum is tagged.
    niche: Option<Niche>,
}

impl EnumLayout {
    /// The layout of an enum tagged by `tags`, which enclose its discriminants, above the widest of
    /// `payloads`, the layouts of its variants with fields.
    #[inline]
    #[must_use]
    pub const fn tagged<D: Primitive>(tags: ReprRange<D>, payloads: &[PackedLayout]) -> Self {
        let mut widest = 0;
        let mut rest = payloads;
        while let [payload, after @ ..] = rest {
            widest = if payload.width() > widest { payload.width() } else { widest };
            rest = after;
        }
        // A tag of no bits, a lone discriminant of zero, lies above the payloads all the same.
        Self { selector: PackedLayout::from_top_field(PackedField::new(tags, widest)), niche: None }
    }

    /// The layout of an enum of `unit_count` unit variants and one with fields, laid out in
    /// `payload`, whose discriminants are their indices, `discriminant` the payload's:
    /// niche-filling where the units fit beside the payload's top field and leave the enum no
    /// wider than a tag would, else tagged.
    #[inline]
    #[must_use]
    pub const fn niche_or_tagged<D: const ExactBits>(
        unit_count: u128, payload: PackedLayout, discriminant: D,
    ) -> Self {
        let top = payload.top;
        match NicheLayout::beside(top.reprs, unit_count) {
            Some(niche) => Self {
                selector: PackedLayout::from_top_field(PackedField::from_span(
                    niche.span, top.offset,
                )),
                niche: Some(Niche {
                    layout: niche,
                    payload,
                    payload_discriminant: discriminant.to_bits(),
                }),
            },
            None => Self::tagged(ReprRange::<D>::from_bounds(0, unit_count), &[payload]),
        }
    }

    /// How many bits the enum takes: up to its selector's last.
    #[inline]
    #[must_use]
    pub const fn width(self) -> u32 {
        self.selector.width()
    }

    /// The enum's range, in its repr `R`, which its selector decides.
    #[inline]
    #[must_use]
    pub const fn range<R: Primitive>(self) -> ReprRange<R> {
        self.selector.range()
    }

    /// The repr of the variant whose discriminant is `discriminant`, its fields packed into `bits`.
    #[inline]
    #[must_use]
    pub const fn repr<R: const ExactBits, D: const ExactBits>(
        self, discriminant: D, bits: u128,
    ) -> R {
        let discriminant = discriminant.to_bits();
        let selector = match self.niche {
            None => discriminant,
            Some(niche) if discriminant == niche.payload_discriminant => {
                niche.payload.top.unpack(bits)
            },
            Some(niche) => niche.unit_bits(discriminant),
        };
        let below = bits & mask(self.selector.top.offset);
        self.selector.repr(self.selector.top.pack(selector) | below)
    }

    /// The bits of `repr`, or `None` where no value encodes to it whatever its variant: where those
    /// above the enum's width do not extend its selector, or, beside a niche, where the selector is
    /// no unit's and the payload's top field does not read it back as it is.
    #[inline]
    #[must_use]
    pub const fn canonical_bits<R: const ExactBits>(self, repr: R) -> Option<u128> {
        let Some(bits) = self.selector.canonical_bits(repr) else {
            return None;
        };
        if let Some(niche) = self.niche
            && !niche.has_variant(self.selector.top.unpack(bits))
        {
            return None;
        }
        Some(bits)
    }

    /// The discriminant, in the enum's discriminants' integer `D`, of the variant whose value
    /// `bits`, canonical, hold.
    #[inline]
    #[must_use]
    pub const fn discriminant<D: const ExactBits>(self, bits: u128) -> D {
        D::from_bits(self.discriminant_bits(bits))
    }

    /// Whether `bits` are clear below the selector, as a unit variant's value is.
    #[inline]
    #[must_use]
    pub const fn is_clear_below_selector(self, bits: u128) -> bool {
        bits & mask(self.selector.top.offset) == 0
    }

    /// Whether `bits` are clear above `variant`'s fields, up to the selector, as a value of that
    /// variant is: beside a niche, the payload's fields take every bit below its top field.
    #[inline]
    #[must_use]
    pub const fn is_clear_above_fields(self, bits: u128, variant: PackedLayout) -> bool {
        (bits & mask(self.selector.top.offset)).unbounded_shr(variant.width()) == 0
    }

    /// The discriminant of the variant whose value `bits`, canonical, hold, as unsigned bits.
    #[inline]
    #[must_use]
    pub(crate) const fn discriminant_bits(self, bits: u128) -> u128 {
        let selector = self.selector.top.unpack(bits);
        match self.niche {
            Some(niche) => niche.discriminant(selector),
            None => selector,
        }
    }

    /// Whether each pattern of the selector's bits says one of the enum's `variant_count` variants,
    /// each of a discriminant of its own: a tag's range encloses the discriminants, so a pattern
    /// may lie between two of them, unless there are as many as patterns; a niche's range holds
    /// the units' selectors and the payload's top field's range alone.
    #[inline]
    #[must_use]
    pub(crate) const fn is_selector_full(self, variant_count: u128) -> bool {
        let selector = self.selector.top;
        match self.niche {
            Some(_) => selector.every_pattern_is_a_repr(),
            None => mask(selector.layout.width) == variant_count.wrapping_sub(1),
        }
    }

    /// Whether `variant`'s fields take every bit below the selector.
    #[inline]
    #[must_use]
    pub(crate) const fn is_full_below_selector(self, variant: PackedLayout) -> bool {
        variant.width() >= self.selector.top.offset
    }
}

/// Where an enum's unit variants fill a niche beside its one variant with fields, its payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Niche {
    /// Where the units' selectors lie, beside the payload's top field's range.
    layout: NicheLayout,
    /// The payload's layout, as a packed value's.
    payload: PackedLayout,
    /// The payload's discriminant, its index among the variants.
    payload_discriminant: u128,
}

impl Niche {
    /// The selector of the unit variant whose discriminant, its index, is `discriminant`: the units
    /// are numbered in declaration order past the payload, as if it were not among them.
    #[inline]
    #[must_use]
    const fn unit_bits(self, discriminant: u128) -> u128 {
        let unit = if discriminant < self.payload_discriminant {
            discriminant
        } else {
            discriminant.wrapping_sub(1)
        };
        self.layout.unit_bits(unit)
    }

    /// Whether `selector`, a repr of the payload's top field's primitive, is a unit's, or the
    /// payload's top field reads it back as it is.
    #[inline]
    #[must_use]
    const fn has_variant(self, selector: u128) -> bool {
        let top = self.payload.top;
        self.layout.unit(selector).is_some() || top.layout.holds(selector, top.reprs.width)
    }

    /// The discriminant of the variant whose selector is `selector`: a unit's, or else the
    /// payload's.
    #[inline]
    #[must_use]
    const fn discriminant(self, selector: u128) -> u128 {
        match self.layout.unit(selector) {
            Some(unit) if unit < self.payload_discriminant => unit,
            Some(unit) => unit.wrapping_add(1),
            None => self.payload_discriminant,
        }
    }
}

/// Where an enum puts its unit variants beside a field of its one variant with fields: in reprs
/// beside the field's range, as `Option`'s `None` takes one, so the enum is no wider than a tag
/// above the field would make it.
///
/// The units' reprs and the field's are reprs of the field's primitive; the enum stores them as a
/// field of the range they share, which reads them back to that primitive before they are compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NicheLayout {
    /// The field's reprs and the units', of the field's primitive.
    pub(super) span: Span,
    /// The first unit's repr; each next unit takes the repr after it.
    first_unit_bits: u128,
    /// How many units there are.
    unit_count: u128,
}

impl NicheLayout {
    /// The layout of `unit_count` unit variants beside a field whose reprs lie in `field`, or
    /// `None` where fewer than `unit_count` reprs lie outside it, or where a tag above the
    /// field, numbering the field's variant and each unit, makes a narrower enum.
    ///
    /// The units take the reprs below the field's start, or above its end, whichever leaves the
    /// narrower field; on a tie, the side that puts a lone unit at zero, then the side that does
    /// not wrap, then below.
    #[inline]
    #[must_use]
    pub(super) const fn beside(field: Span, unit_count: u128) -> Option<Self> {
        let Some((span, first_unit_bits)) = field.grown_by(unit_count) else {
            return None;
        };
        let tagged = field.field_layout().width.saturating_add(bit_length(unit_count));
        if span.field_layout().width > tagged {
            return None;
        }
        Some(Self { span: span.normalized(), first_unit_bits, unit_count })
    }

    /// The repr of the unit variant that is `unit`th among the units, in declaration order, as the
    /// field primitive's unsigned bits.
    #[inline]
    #[must_use]
    pub(super) const fn unit_bits(self, unit: u128) -> u128 {
        self.first_unit_bits.wrapping_add(unit) & self.span.largest()
    }

    /// Which unit, in declaration order, takes `bits`, a repr of the field's primitive; `None`
    /// where none does.
    #[inline]
    #[must_use]
    pub(super) const fn unit(self, bits: u128) -> Option<u128> {
        let unit = bits.wrapping_sub(self.first_unit_bits) & self.span.largest();
        if unit < self.unit_count { Some(unit) } else { None }
    }
}
