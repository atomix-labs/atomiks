//! [`PointerEnumLayout`], how an enum whose variants hold a pointer lays them out in one pointer
//! word: its tag in the low bits, above the widest pointer's own tags; a pointer variant's tag
//! fields above the tag; and above the bits every pointer's alignment leaves clear, a pointer's
//! address, or a data variant's fields, an address without provenance.

use core::any::type_name;
use core::ptr;

use super::layout::{FieldLayout, PackedLayout};
use super::pointer_word::{PointeeAlignment, SecondWord, Tags, assert_low_bits};
use super::{bit_length, mask};
use crate::atom::Atom;
use crate::message::{Message, refuse};
use crate::primitive::{ExactBits, address, subtract_tags};

/// A variant of an enum whose variants hold a pointer, as its layout reads it: its discriminant,
/// which is its tag, and what it holds.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerEnumVariant {
    /// Its discriminant, as unsigned bits.
    discriminant: u128,
    /// What it holds.
    contents: Contents,
}

/// What a variant of an enum whose variants hold a pointer holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contents {
    /// Nothing: its repr is its tag alone.
    Unit,
    /// Fields that hold no pointer, packed from bit 0 as a packed value's are, and held above the
    /// enum's clear bits.
    Data,
    /// A pointer field beside tag fields, held above the enum's tag.
    Pointer {
        /// How many low bits the pointer field keeps its own tags in.
        tag_width: u32,
        /// Whether the pointer field's repr is never null.
        is_never_null: bool,
        /// The tag fields' layout, from bit 0.
        tag_fields: PackedLayout,
    },
}

impl PointerEnumVariant {
    /// A unit variant of `discriminant`.
    #[inline]
    #[must_use]
    pub const fn unit<D: const ExactBits>(discriminant: D) -> Self {
        Self { discriminant: discriminant.to_bits(), contents: Contents::Unit }
    }

    /// A variant of `discriminant` whose fields hold no pointer.
    #[inline]
    #[must_use]
    pub const fn data<D: const ExactBits>(discriminant: D) -> Self {
        Self { discriminant: discriminant.to_bits(), contents: Contents::Data }
    }

    /// A variant of `discriminant` whose pointer field is an `F`, beside tag fields laid out in
    /// `tag_fields`, from bit 0.
    ///
    /// It reads `F`'s tag width and range, never its pointees' alignment, so an enum laid out of it
    /// may be held in its own pointee.
    #[inline]
    #[must_use]
    pub const fn pointer<D: const ExactBits, F: Atom>(
        discriminant: D, tag_fields: PackedLayout,
    ) -> Self {
        let (tag_width, is_never_null) = (F::TAG_WIDTH, !F::REPRS.contains(0));
        let contents = Contents::Pointer { tag_width, is_never_null, tag_fields };
        Self { discriminant: discriminant.to_bits(), contents }
    }
}

/// How an enum whose variants hold a pointer lays them out in one pointer word, its repr.
///
/// Tagged, a variant's tag is its discriminant, in the bits from `discriminant_offset`, above the
/// widest pointer variant's own tags: a pointer variant's repr is its pointer field's, offset by
/// the tag and by its tag fields' bits above it, which keeps its provenance; a unit's is its tag
/// alone; a data variant's, its tag beside its fields' bits above the low bits its pointees'
/// alignment leaves clear, an address without provenance. Where the enum fills the niche of one
/// pointer that is never null, beside one unit and no data, the unit is null and the pointer
/// variant takes no tag.
///
/// The layout reads no pointee's alignment, so a type's layout may read it: the code that places a
/// data variant's fields, and the check that they fit, take the alignment, the least of the
/// pointer fields' [`PointeeAlignment`]s.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerEnumLayout {
    /// Where the tag, the discriminant, lies: above the widest pointer variant's own tags.
    discriminant_offset: u32,
    /// How many bits the tag takes: none where the enum fills its pointer's niche.
    discriminant_width: u32,
    /// How many bits the widest pointer variant's tag fields take, above the tag.
    tag_field_width: u32,
    /// The unit that is null, and the pointer variant beside it, where the enum fills its
    /// pointer's niche.
    niche: Option<Niche>,
}

/// The variants of an enum that fills its pointer's niche, by their discriminants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Niche {
    /// The unit's, which is null.
    unit_discriminant: u128,
    /// The pointer variant's, which is its pointer field's repr.
    pointer_discriminant: u128,
}

impl PointerEnumLayout {
    /// The layout of an enum of `variants`, each tagged by its discriminant.
    #[inline]
    #[must_use]
    pub const fn tagged(variants: &[PointerEnumVariant]) -> Self {
        let (mut offset, mut tag_field_width, mut largest) = (0, 0, 0);
        let mut rest = variants;
        while let [variant, after @ ..] = rest {
            rest = after;
            if variant.discriminant > largest {
                largest = variant.discriminant;
            }
            let Contents::Pointer { tag_width, tag_fields, .. } = variant.contents else {
                continue;
            };
            if tag_width > offset {
                offset = tag_width;
            }
            if tag_fields.width() > tag_field_width {
                tag_field_width = tag_fields.width();
            }
        }
        Self {
            discriminant_offset: offset,
            discriminant_width: bit_length(largest),
            tag_field_width,
            niche: None,
        }
    }

    /// The layout of an enum of `variants`, whose discriminants are their indices: filling the
    /// pointer's niche where they are one unit and one variant whose pointer is never null, the
    /// unit null and the pointer untagged, as `Option<NonNull<T>>` is laid out; else tagged.
    #[inline]
    #[must_use]
    pub const fn niche_or_tagged(variants: &[PointerEnumVariant]) -> Self {
        let tagged = Self::tagged(variants);
        let niche = match variants {
            [
                unit @ PointerEnumVariant { contents: Contents::Unit, .. },
                pointer @ PointerEnumVariant {
                    contents: Contents::Pointer { is_never_null: true, .. },
                    ..
                },
            ]
            | [
                pointer @ PointerEnumVariant {
                    contents: Contents::Pointer { is_never_null: true, .. },
                    ..
                },
                unit @ PointerEnumVariant { contents: Contents::Unit, .. },
            ] => Niche {
                unit_discriminant: unit.discriminant,
                pointer_discriminant: pointer.discriminant,
            },
            _ => return tagged,
        };
        Self { discriminant_width: 0, niche: Some(niche), ..tagged }
    }

    /// The bit after the enum's own bits, its tag's and its tag fields'.
    #[inline]
    #[must_use]
    const fn end(self) -> u32 {
        self.tag_field_offset().saturating_add(self.tag_field_width)
    }

    /// Where a pointer variant's tag fields lie: above the tag.
    #[inline]
    #[must_use]
    const fn tag_field_offset(self) -> u32 {
        self.discriminant_offset.saturating_add(self.discriminant_width)
    }

    /// The enum's own bits, its tag's and its tag fields', set.
    #[inline]
    #[must_use]
    const fn own_mask(self) -> usize {
        (mask(self.end()) & !mask(self.discriminant_offset)).wrapping_cast()
    }

    /// How a pointer variant's pointer is stored, for what its validity promises of the enum's zero
    /// repr: every bit but the enum's own.
    #[inline]
    #[must_use]
    pub const fn pointer_layout(self) -> FieldLayout {
        FieldLayout::from_signed(0, mask(usize::BITS.saturating_sub(self.end())).cast_signed())
    }

    /// How many low bits hold the enum's tags, its pointers' own, its tag and its tag
    /// fields: its [`Atom::TAG_WIDTH`].
    #[inline]
    #[must_use]
    pub const fn tag_width(self) -> u32 {
        self.end()
    }

    /// The tag of the variant of `discriminant`, in place: none beside a niche.
    #[inline]
    #[must_use]
    const fn tag(self, discriminant: u128) -> usize {
        match self.niche {
            Some(_) => 0,
            None => discriminant.unbounded_shl(self.discriminant_offset).wrapping_cast(),
        }
    }

    /// The repr of the unit variant of `discriminant`: its tag alone, null beside a niche.
    #[inline]
    #[must_use]
    pub const fn unit_repr<D: const ExactBits>(self, discriminant: D) -> *mut () {
        ptr::without_provenance_mut(self.tag(discriminant.to_bits()))
    }

    /// The repr of the data variant of `discriminant`, whose fields' bits are `bits`: its tag,
    /// beside those bits above the low bits `alignment`, the enum's pointees', leaves clear, an
    /// address without provenance.
    #[inline]
    #[must_use]
    pub const fn data_repr<D: const ExactBits>(
        self, discriminant: D, bits: u128, alignment: PointeeAlignment,
    ) -> *mut () {
        let fields: usize = bits.unbounded_shl(alignment.width()).wrapping_cast();
        ptr::without_provenance_mut(fields | self.tag(discriminant.to_bits()))
    }

    /// The tags the pointer variant of `discriminant` passes on to its pointer field: the tag, and
    /// its tag fields above it, whose bits, from bit 0, are `bits`, beside `outer`, those of each
    /// word that holds the enum.
    ///
    /// The pointer field sets them all, as [`Tags::set_in`] does, so the variant's repr is its
    /// pointer's, offset by the tags, which keeps its provenance. An enum that fills a niche, its
    /// pointer variant beside no tag field, keeps no bits of its own, so a constant builds one of
    /// a pointer to a static.
    #[inline]
    #[must_use]
    pub const fn pointer_tags<D: const ExactBits>(
        self, discriminant: D, bits: u128, outer: Tags,
    ) -> Tags {
        let fields: usize = bits.unbounded_shl(self.tag_field_offset()).wrapping_cast();
        outer.with(self.own_mask(), self.tag(discriminant.to_bits()) | fields)
    }

    /// The discriminant, as unsigned bits, of the variant `repr` holds: beside a niche, the unit's
    /// where it is null, else the pointer variant's.
    #[inline]
    #[must_use]
    pub(crate) const fn discriminant_bits(self, repr: *mut ()) -> u128 {
        let address = address(repr);
        match self.niche {
            Some(Niche { unit_discriminant, .. }) if address == 0 => unit_discriminant,
            Some(Niche { pointer_discriminant, .. }) => pointer_discriminant,
            None => {
                let tag: u128 = address.unbounded_shr(self.discriminant_offset).wrapping_cast();
                tag & mask(self.discriminant_width)
            },
        }
    }

    /// The discriminant, of the enum's discriminants' integer `D`, of the variant `repr` holds.
    #[inline]
    #[must_use]
    pub const fn discriminant<D: const ExactBits>(self, repr: *mut ()) -> D {
        D::from_bits(self.discriminant_bits(repr))
    }

    /// The pointer field's repr, and the tag fields' bits from bit 0, of `repr`, a pointer
    /// variant's: the tag and the tag fields subtracted off, which keeps the pointer's provenance.
    ///
    /// Within the arm of a match that knows the variant, the tag is a constant, so a load
    /// through the pointer takes it into its own offset, where a word, whose tags no match fixes,
    /// masks them off instead.
    #[inline]
    #[must_use]
    pub const fn split(self, repr: *mut ()) -> (*mut (), u128) {
        let own = self.own_mask();
        if own == 0 {
            return (repr, 0);
        }
        let tags = (address(repr) & own).unbounded_shr(self.tag_field_offset()).wrapping_cast();
        (subtract_tags(repr, own), tags)
    }

    /// The fields' bits of `repr`, a data variant's, above the low bits `alignment`, the enum's
    /// pointees', leaves clear.
    #[inline]
    #[must_use]
    pub const fn data_bits(self, repr: *mut (), alignment: PointeeAlignment) -> u128 {
        address(repr).unbounded_shr(alignment.width()).wrapping_cast()
    }

    /// Whether `repr` is exactly the unit variant of `discriminant`'s.
    #[inline]
    #[must_use]
    pub const fn is_unit<D: const ExactBits>(self, repr: *mut (), discriminant: D) -> bool {
        address(repr) == self.tag(discriminant.to_bits())
    }

    /// Whether `repr` is a data variant of `discriminant`'s whose fields lie in `fields`, as
    /// [`data_repr`](Self::data_repr) writes one of `alignment`: its tag, no other bit set below
    /// the fields, and none above them.
    #[inline]
    #[must_use]
    pub const fn is_data<D: const ExactBits>(
        self, repr: *mut (), discriminant: D, fields: PackedLayout, alignment: PointeeAlignment,
    ) -> bool {
        let (address, width) = (address(repr), alignment.width());
        let clear: usize = mask(width).wrapping_cast();
        address & clear == self.tag(discriminant.to_bits())
            && address.unbounded_shr(width).unbounded_shr(fields.width()) == 0
    }

    /// Whether `tags`, the tag fields' bits [`split`](Self::split) gives, have no bit set above
    /// `fields`, those of the variant's own, as [`pointer_tags`](Self::pointer_tags) lays them
    /// out.
    #[inline]
    #[must_use]
    pub const fn is_clear_above_fields(self, tags: u128, fields: PackedLayout) -> bool {
        tags.unbounded_shr(fields.width()) == 0
    }

    /// Refuses the build of `T`, the enum, where its tag and tag fields need more bits than `F`, a
    /// pointer variant's field, leaves clear; `variant` names the variant: "`Child::Leaf`".
    ///
    /// It reads `F`'s [`PointeeAlignment`], so a constant beside `T` calls it, or code once `T` is
    /// laid out, never `T`'s layout.
    ///
    /// # Panics
    /// Where it refuses; in a constant, the build fails instead.
    #[inline]
    #[track_caller]
    pub const fn assert_tags_fit<T: ?Sized, F: Atom>(self, variant: &str) {
        let required = self.end().saturating_sub(F::TAG_WIDTH);
        let what = Message::new().text(variant).text("'s tag and tag fields need");
        assert_low_bits::<T, F>(required, what.as_str(), SecondWord::RuledOut);
    }

    /// Refuses the build of `T`, the enum, where a data variant's fields, laid out in `fields`,
    /// need more bits than lie above the low bits `alignment`, its pointees', leaves clear;
    /// `variant` names the variant: "`Slot::Inline`".
    ///
    /// # Panics
    /// Where it refuses; in a constant, the build fails instead.
    #[inline]
    #[track_caller]
    pub const fn assert_data_fits<T: ?Sized>(
        self, fields: PackedLayout, alignment: PointeeAlignment, variant: &str,
    ) {
        let room = usize::BITS.saturating_sub(alignment.width());
        if fields.width() <= room {
            return;
        }
        let rest = Message::new()
            .text("`: ")
            .text(variant)
            .text("'s fields need ")
            .number(u128::from(fields.width()))
            .text(" bits, but above the ")
            .number(u128::from(alignment.width()))
            .text(" low bits that hold its tags and its pointers' alignment, a word holds ")
            .number(u128::from(room))
            .text(": narrow the fields");
        let rest = rest.as_str();
        refuse(&Message::new().text("`").name(type_name::<T>(), rest.len()).text(rest))
    }
}

#[cfg(test)]
mod tests {
    use core::ptr::{self, NonNull};

    use super::{PointerEnumLayout, PointerEnumVariant};
    use crate::range::{PackedField, PackedLayout, PointeeAlignment, ReprRange, Tags};

    /// A node aligned to 8, so a pointer to one leaves three low bits clear.
    #[repr(align(8))]
    struct Node(u64);

    /// A node's alignment, which leaves three low bits clear.
    const NODE: PointeeAlignment = PointeeAlignment::of::<Node>();

    /// No tag fields.
    const NO_TAGS: PackedLayout = PackedLayout::new(&[]);

    /// `enum Slot { Empty, Inline(u32), Node(NonNull<Node>) }`'s layout, and its value's fields.
    fn slot() -> (PointerEnumLayout, PackedLayout) {
        let value = PackedLayout::new(&[PackedField::new(ReprRange::<u32>::FULL, 0)]);
        let variants = [
            PointerEnumVariant::unit(0_u8),
            PointerEnumVariant::data(1_u8),
            PointerEnumVariant::pointer::<_, NonNull<Node>>(2_u8, NO_TAGS),
        ];
        (PointerEnumLayout::niche_or_tagged(&variants), value)
    }

    #[test]
    fn one_unit_beside_a_pointer_never_null_fills_its_niche() {
        let next = [
            PointerEnumVariant::unit(0_u8),
            PointerEnumVariant::pointer::<_, NonNull<Node>>(1_u8, NO_TAGS),
        ];
        let layout = PointerEnumLayout::niche_or_tagged(&next);
        assert_eq!(layout.tag_width(), 0, "no tag");
        assert_eq!(layout.unit_repr(0_u8), ptr::null_mut(), "the unit is null");
        let node = ptr::without_provenance_mut(8);
        assert_eq!(layout.discriminant::<u8>(node), 1, "else the pointer");
        assert_eq!(layout.discriminant::<u8>(ptr::null_mut()), 0, "and null the unit");
        let tagged = PointerEnumLayout::tagged(&next);
        assert_eq!(tagged.tag_width(), 1, "but tagged where tagging is asked");
        let pointer_first = [next[1], next[0]];
        let niche = PointerEnumLayout::niche_or_tagged(&pointer_first).tag_width();
        assert_eq!(niche, 0, "in either order");
        let nullable = [
            PointerEnumVariant::unit(0_u8),
            PointerEnumVariant::pointer::<_, *mut Node>(1_u8, NO_TAGS),
        ];
        let nullable = PointerEnumLayout::niche_or_tagged(&nullable).tag_width();
        assert_eq!(nullable, 1, "and beside a pointer that may be null");
    }

    #[test]
    fn a_tagged_enum_keeps_its_tag_low_and_its_data_above_the_alignment() {
        let (layout, value) = slot();
        assert_eq!(layout.tag_width(), 2, "two tag bits");
        let repr = layout.data_repr(1_u8, 5, NODE);
        assert_eq!(repr.addr(), 5 << 3 | 1, "the value above a node's three clear bits");
        let decoded = (layout.discriminant::<u8>(repr), layout.data_bits(repr, NODE));
        assert_eq!(decoded, (1, 5), "and back");
        assert!(layout.is_data(repr, 1_u8, value, NODE), "canonical");
        let clear_bit_set = ptr::without_provenance_mut(0b101);
        assert!(!layout.is_data(clear_bit_set, 1_u8, value, NODE), "not with a clear bit set");
        assert!(layout.is_unit(ptr::null_mut(), 0_u8), "a unit exactly");
        assert!(!layout.is_unit(ptr::without_provenance_mut(8), 0_u8), "and no more");
    }

    #[test]
    fn a_pointer_variant_keeps_its_provenance() {
        let (layout, _) = slot();
        let mut node = Node(7);
        let pointer = ptr::from_mut(&mut node);
        let (repr, misaligned) = layout.pointer_tags(2_u8, 0, Tags::EMPTY).set_in(pointer);
        assert_eq!((repr.addr(), misaligned), (pointer.addr() | 2, 0), "tagged 2");
        let (back, tags) = layout.split(repr.cast());
        assert_eq!((back, tags), (pointer.cast(), 0), "split off again");
        // SAFETY: `back` is `pointer`, offset by the tag and cleared of it, so it keeps its
        // provenance over `node`, which is live.
        #[expect(unsafe_code, reason = "reads through the pointer, so Miri checks its provenance")]
        let read = unsafe { back.cast::<Node>().read().0 };
        assert_eq!(read, 7, "and reads the node");
    }

    #[test]
    fn a_pointer_variants_tag_fields_lie_above_the_tag() {
        let deleted = PackedLayout::new(&[PackedField::new(ReprRange::<bool>::FULL, 0)]);
        let link = [
            PointerEnumVariant::data(0_u8),
            PointerEnumVariant::pointer::<_, NonNull<Node>>(1_u8, deleted),
        ];
        let layout = PointerEnumLayout::niche_or_tagged(&link);
        assert_eq!(layout.tag_width(), 2, "the tag, then the mark");
        let pointer = ptr::without_provenance_mut::<Node>(0x1000);
        let (repr, _) = layout.pointer_tags(1_u8, 1, Tags::EMPTY).set_in(pointer);
        assert_eq!(repr.addr(), 0x1003, "the mark at bit 1");
        assert_eq!(layout.split(repr.cast()), (pointer.cast(), 1), "and split off to bit 0");
        assert!(!layout.is_clear_above_fields(0b10, deleted), "a bit past the mark is no value's");
    }

    #[test]
    fn a_pointer_misaligned_for_the_tag_is_found() {
        let (layout, _) = slot();
        let misaligned = ptr::without_provenance_mut::<Node>(0x1002);
        let (_, set) = layout.pointer_tags(2_u8, 0, Tags::EMPTY).set_in(misaligned);
        assert_eq!(set, 0b10, "the tag's bit, set already");
    }

    #[test]
    #[should_panic(
        expected = "`()`: `Slot::Node`'s tag and tag fields need 3 low bits, but `u32` \
                               is 4-byte aligned (2 free bits): shrink the tags, or raise the \
                               pointee's alignment with `#[repr(align(8))]`"
    )]
    fn a_tag_past_a_pointer_variants_alignment_is_refused() {
        let variants = [
            PointerEnumVariant::unit(0_u8),
            PointerEnumVariant::pointer::<_, NonNull<u32>>(4_u8, NO_TAGS),
        ];
        PointerEnumLayout::tagged(&variants).assert_tags_fit::<(), NonNull<u32>>("`Slot::Node`");
    }

    #[test]
    #[should_panic(
        expected = "`()`: `Slot::Inline`'s fields need 64 bits, but above the 3 low bits \
                               that hold its tags and its pointers' alignment, a word holds 61: \
                               narrow the fields"
    )]
    fn data_wider_than_the_bits_above_the_alignment_is_refused() {
        let (layout, _) = slot();
        let wide = PackedLayout::new(&[PackedField::new(ReprRange::<u64>::FULL, 0)]);
        layout.assert_data_fits::<()>(wide, NODE, "`Slot::Inline`");
    }
}
