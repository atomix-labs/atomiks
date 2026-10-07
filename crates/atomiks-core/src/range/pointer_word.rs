//! [`PointeeAlignment`], the low bits a value stored as a pointer leaves clear for tags, and
//! [`PointerWordLayout`], how a struct of one pointer lays out its tags there: above the pointer's
//! own, below its address, whose provenance the word keeps.

use core::any::type_name;

use super::layout::{FieldLayout, PackedField, PackedLayout};
use super::{ReprRange, Span, mask};
use crate::atom::Atom;
use crate::message::{Message, refuse};
use crate::primitive::Primitive;

/// The alignment of what a value stored as a pointer points to, as the low bits it leaves clear in
/// each of the value's reprs: the value's own tags lie there, below [`Atom::TAG_WIDTH`], and above
/// them, those of a word that holds the value.
///
/// Only code and the checks read it, never a type's layout, which may hold the pointee: a list's
/// node holds an atomic link to the next node, so the link's layout asking the node's alignment
/// would ask its own. A word checks that the bits it sets are clear before it sets them, so a value
/// whose alignment is wrong is refused, at compile time or at run time, and never decodes wrong.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointeeAlignment {
    /// How many low bits it leaves clear.
    width: u32,
    /// The name of the type a plain pointer points to, whose alignment alone leaves the bits
    /// clear: what a refusal of too many tags names.
    pointee: Option<&'static str>,
}

impl PointeeAlignment {
    /// No bit clear: a value stored as bits, or a pointer whose impl says nothing of its pointees.
    pub const NONE: Self = Self { width: 0, pointee: None };

    /// The alignment of a `T`, as the low bits it leaves clear in a pointer to one.
    #[inline]
    #[must_use]
    pub const fn of<T>() -> Self {
        Self { width: align_of::<T>().trailing_zeros(), pointee: Some(type_name::<T>()) }
    }

    /// The least of `alignments`, a word's or an enum's pointers', which leaves clear only the bits
    /// each leaves clear: none where there is no pointer, which no derived value lacks.
    ///
    /// It names no pointee, so a refusal of too many tags names the value that holds them.
    #[inline]
    #[must_use]
    pub const fn least(alignments: &[Self]) -> Self {
        let [first, rest @ ..] = alignments else {
            return Self::NONE;
        };
        let mut width = first.width;
        let mut rest = rest;
        while let [alignment, after @ ..] = rest {
            rest = after;
            if alignment.width < width {
                width = alignment.width;
            }
        }
        Self { width, pointee: None }
    }

    /// How many low bits it leaves clear.
    #[inline]
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }
}

/// The tags a pointer word sets in the low bits of the pointer it holds, beside those of each word
/// that holds it: the bits they take, and what those bits are.
///
/// Each word passes its own on to its pointer field, so the pointer innermost takes every word's
/// tags in one offset, after one test that its bits under them are clear.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tags {
    /// The bits the tags take.
    mask: usize,
    /// The tags, each in its place: no bit outside `mask`.
    bits: usize,
}

impl Tags {
    /// No tag: a value no word holds.
    pub const EMPTY: Self = Self { mask: 0, bits: 0 };

    /// These tags, beside the tags of a word that take `mask` and are `bits`, each in its place.
    #[inline]
    #[must_use]
    pub(super) const fn with(self, mask: usize, bits: usize) -> Self {
        Self { mask: self.mask | mask, bits: self.bits | (bits & mask) }
    }

    /// `repr` with the tags set, beside the bits under them it had set already: none, unless it is
    /// a pointer misaligned for them, whose repr returned is then no value's, for the caller to
    /// refuse.
    ///
    /// A constant reads no pointer's address but null's, unless there is no tag to set.
    #[inline]
    #[must_use]
    pub const fn set_in<R: const Primitive>(self, repr: R) -> (R, usize) {
        if self.mask == 0 {
            return (repr, 0);
        }
        let address = repr.packed_bits();
        let misaligned = address.wrapping_cast::<usize>() & self.mask;
        // An add, not an or: the bits under the tags are clear where the repr is a value's, and
        // `with_packed_bits` offsets a pointer by the difference, which an add makes the tags.
        (repr.with_packed_bits(address.wrapping_add(self.bits.wrapping_cast())), misaligned)
    }
}

/// How a pointer word lays out its fields: its pointer's own tags in the low bits, then each of its
/// tag fields, in declaration order, as a packed value's fields, then the pointer's address above
/// them all.
///
/// A repr is the pointer field's repr, offset by the tags' bits, which lands where
/// `map_addr(|address| address | tags)` would, since the pointer's bits under the tags are clear,
/// so it keeps the pointer's provenance; [`split`](Self::split) clears the tags off again.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerWordLayout {
    /// How many low bits the pointer field keeps its own tags in.
    pointer_tag_width: u32,
    /// The tag fields' layout, the first placed at `pointer_tag_width`.
    tag_fields: PackedLayout,
}

impl PointerWordLayout {
    /// The layout of a word whose pointer field keeps its own tags in `pointer_tag_width` low bits,
    /// its [`Atom::TAG_WIDTH`], and whose tag fields lie in `tag_fields`, the first placed there.
    #[inline]
    #[must_use]
    pub const fn new(pointer_tag_width: u32, tag_fields: PackedLayout) -> Self {
        Self { pointer_tag_width, tag_fields }
    }

    /// How many low bits hold the word's tags, its pointer's and its own: its [`Atom::TAG_WIDTH`].
    #[inline]
    #[must_use]
    pub const fn tag_width(self) -> u32 {
        let end = self.tag_fields.width();
        if end > self.pointer_tag_width { end } else { self.pointer_tag_width }
    }

    /// How many bits the word's own tag fields take, above its pointer's own tags.
    #[inline]
    #[must_use]
    pub const fn tag_field_width(self) -> u32 {
        self.tag_width().saturating_sub(self.pointer_tag_width)
    }

    /// The bits of the word's own tag fields, set.
    #[inline]
    #[must_use]
    const fn own_mask(self) -> usize {
        (mask(self.tag_width()) & !mask(self.pointer_tag_width)).wrapping_cast()
    }

    /// The pointer, as a field: every bit above the tags, the pointer's own and the word's,
    /// unsigned.
    #[inline]
    #[must_use]
    const fn pointer_field(self) -> PackedField {
        let width = usize::BITS.saturating_sub(self.tag_width());
        PackedField::from_span(
            Span { start: 0, end: mask(width), width: usize::BITS },
            self.tag_width(),
        )
    }

    /// How the pointer is stored, for what its validity promises of the word's reprs.
    #[inline]
    #[must_use]
    pub const fn pointer_layout(self) -> FieldLayout {
        self.pointer_field().layout()
    }

    /// Where the pointer field's place lies: the low bits of the word's repr that are the low bits
    /// of the pointer's, which hold the pointer's own tags, so a path through the place reaches
    /// them.
    #[inline]
    #[must_use]
    pub const fn pointer_placement(self) -> PackedField {
        let width = self.pointer_tag_width;
        PackedField::from_span(Span { start: 0, end: mask(width), width: usize::BITS }, 0)
    }

    /// How the word lays out its fields, as a packed value, which a tag's place is read against:
    /// its tag fields, then the pointer as its top field, so a tag, however signed, governs its own
    /// bits alone.
    #[inline]
    #[must_use]
    pub const fn packed_layout(self) -> PackedLayout {
        PackedLayout::from_top_field(self.pointer_field())
    }

    /// The word's range, of the pointer repr `R`, whose pointer field's range is `pointer`: every
    /// repr but zero where the pointer is never null, since a pointer that is not null keeps a bit
    /// set once tags are added to bits it leaves clear; else every repr.
    #[inline]
    #[must_use]
    pub const fn range<R: Primitive>(self, pointer: ReprRange<R>) -> ReprRange<R> {
        if pointer.contains(0) { ReprRange::FULL } else { ReprRange::NONZERO }
    }

    /// The tags a word of this layout passes on to its pointer field: its tag fields', whose bits,
    /// each in its place, are `bits`, beside `outer`, those of each word that holds it.
    ///
    /// The pointer field sets them all, as [`Tags::set_in`] does, so a word's repr is its
    /// pointer's, offset by the tags, which keeps its provenance.
    #[inline]
    #[must_use]
    pub const fn pointer_tags(self, bits: u128, outer: Tags) -> Tags {
        outer.with(self.own_mask(), bits.wrapping_cast())
    }

    /// The pointer field's repr that `repr` holds, the word's own tags cleared, and those tags'
    /// bits, each in its place.
    ///
    /// A constant reads no pointer's address but null's, unless the word has no tag bits.
    #[inline]
    #[must_use]
    pub const fn split<R: const Primitive>(self, repr: R) -> (R, u128) {
        let own = self.own_mask();
        if own == 0 {
            return (repr, 0);
        }
        let own: u128 = own.wrapping_cast();
        (repr.clear_packed_bits(own), repr.packed_bits() & own)
    }

    /// Refuses the build of `T`, a word of this layout whose pointer field is an `F`, where its tag
    /// fields need more bits than `F` leaves clear above its own tags.
    ///
    /// `what` names the tag fields and says what they do: "tag fields `version`, `marked` need". It
    /// reads `F`'s [`PointeeAlignment`], so a constant beside `T` calls it, or code once `T` is
    /// laid out, never `T`'s layout.
    ///
    /// # Panics
    /// Where it refuses; in a constant, the build fails instead.
    #[inline]
    #[track_caller]
    pub const fn assert_tags_fit<T: ?Sized, F: Atom>(self, what: &str) {
        assert_low_bits::<T, F>(self.tag_field_width(), what);
    }
}

/// Refuses a value of `T`, a pointer word or a pointer enum, whose pointer had a bit set where its
/// tags go: where `misaligned`, the bits [`Tags::set_in`] found set there in each value encoded, is
/// not zero.
///
/// An exchange encodes two values, and tests both in one test, so one refusal stands on its path.
/// The refusal reads neither the bits nor `T`'s layout: the fast path keeps no more than the test,
/// and a generic instance whose tags do not fit is refused once, where its code is built.
///
/// # Panics
/// Where it refuses; in a constant, the build fails instead.
#[inline]
#[track_caller]
pub const fn assert_aligned<T: ?Sized>(misaligned: usize) {
    if misaligned != 0 {
        refuse_misaligned::<T>();
    }
}

/// Refuses a value of `T` whose pointer has a bit set where its tags go.
#[cold]
#[inline(never)]
#[track_caller]
const fn refuse_misaligned<T: ?Sized>() -> ! {
    let rest = concat!(
        "`: the pointer's address has a bit set where its tags go, so it is not aligned for ",
        "them"
    );
    refuse(&Message::new().text("`").name(type_name::<T>(), rest.len()).text(rest))
}

/// The low bits Rust's largest alignment, 2^29 bytes, leaves clear.
const MAX_ALIGNMENT_WIDTH: u32 = 29;

/// Refuses the build of `T`, which keeps `required` bits of its own above the tags of `F`, a value
/// it holds, where `F` leaves fewer clear; `what` names what needs them: "tag fields `version`,
/// `marked` need".
///
/// A plain pointer's refusal names its pointee's alignment; a word's, the bits it leaves clear. An
/// `F` stored as bits is no pointer, which `__private::assert_pointer` refuses, so this refuses
/// nothing of it, and the build fails once.
#[inline]
#[track_caller]
pub(super) const fn assert_low_bits<T: ?Sized, F: Atom>(required: u32, what: &str) {
    let (tag_width, alignment) = (F::TAG_WIDTH, F::POINTEE_ALIGNMENT);
    let free = alignment.width.saturating_sub(tag_width);
    if required <= free || <F::Repr as Primitive>::IS_BITS_EXACT {
        return;
    }
    let required_alignment_width = tag_width.saturating_add(required);
    // No alignment holds the tags past Rust's largest, 2^29 bytes: the advice is to shrink them.
    let can_raise = required_alignment_width <= MAX_ALIGNMENT_WIDTH;
    let (named, tail) = if let Some(pointee) = alignment.pointee {
        let tail = Message::new()
            .text("` is ")
            .number(1_u128.unbounded_shl(alignment.width))
            .text("-byte aligned (")
            .number(u128::from(free))
            .text(if free == 1 { " free bit)" } else { " free bits)" });
        let tail = if can_raise {
            tail.text(": shrink the tags, or raise the pointee's alignment with `#[repr(align(")
                .number(1_u128.unbounded_shl(required_alignment_width))
                .text("))]`")
        } else {
            tail.text(": shrink the tags, which no alignment holds")
        };
        (pointee, tail)
    } else {
        let tail = Message::new()
            .text("` keeps ")
            .number(u128::from(tag_width))
            .text(" for its own tags and leaves ")
            .number(u128::from(free))
            .text(" clear above them");
        let tail = if can_raise {
            tail.text(": shrink the tags, or raise its pointees' alignment to ")
                .number(1_u128.unbounded_shl(required_alignment_width))
                .text(" bytes")
        } else {
            tail.text(": shrink the tags, which no alignment holds")
        };
        (type_name::<F>(), tail)
    };
    let tail = tail.as_str();
    let head = Message::new()
        .text("`: ")
        .text(what)
        .text(" ")
        .number(u128::from(required))
        .text(if required == 1 { " low bit, but `" } else { " low bits, but `" });
    let head = head.as_str();
    // The word's name leaves the other's room for up to 48 bytes of it.
    let named_room = if named.len() < 48 { named.len() } else { 48 };
    let room = head.len().saturating_add(tail.len()).saturating_add(named_room);
    refuse(
        &Message::new()
            .text("`")
            .name(type_name::<T>(), room)
            .text(head)
            .name(named, tail.len())
            .text(tail),
    )
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::{PointeeAlignment, PointerWordLayout, Tags, assert_aligned};
    use crate::atom::Atom;
    use crate::range::{PackedField, PackedLayout, ReprRange};

    /// A word of two tag fields above the `pointer_tag_width` bits of its pointer's own tags, a
    /// `bool` and a field of the reprs 0 to 3: three bits.
    fn three_bits(pointer_tag_width: u32) -> PointerWordLayout {
        let flag = PackedField::new(ReprRange::<bool>::FULL, pointer_tag_width);
        let count = PackedField::new(ReprRange::<u8>::new(0, 3), flag.next_offset());
        PointerWordLayout::new(pointer_tag_width, PackedLayout::new(&[flag, count]))
    }

    /// A pointee aligned to 8, leaving three low bits clear.
    #[repr(align(8))]
    struct Eight;

    /// A pointee aligned to 16, leaving four low bits clear.
    #[repr(align(16))]
    struct Sixteen;

    /// A value stored as a pointer to a `P` that keeps one tag of its own in bit 0.
    struct Tagged<P>(*mut P);

    impl<P> Clone for Tagged<P> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<P> Copy for Tagged<P> {}

    // SAFETY: `to_repr` gives the pointer, any repr, and `from_repr` decodes every repr as the
    // pointer it is, by the repr alone, as the default `from_repr_unchecked` does; `Partial`
    // promises nothing; and the pointer's address may cross threads, as `AtomicPtr`'s does.
    #[expect(unsafe_code, reason = "a value stored as a pointer, with a tag of its own")]
    unsafe impl<P> Atom for Tagged<P> {
        type Repr = *mut P;
        const REPRS: ReprRange<*mut P> = ReprRange::FULL;
        const TAG_WIDTH: u32 = 1;
        const POINTEE_ALIGNMENT: PointeeAlignment =
            PointeeAlignment::least(&[PointeeAlignment::of::<P>()]);
        fn to_repr(self) -> *mut P {
            self.0
        }
        fn from_repr(repr: *mut P) -> Option<Self> {
            Some(Self(repr))
        }
    }

    #[test]
    fn a_pointee_leaves_the_bits_its_alignment_does_and_a_holder_the_fewest_of_its_pointees() {
        let (eight, sixteen) = (PointeeAlignment::of::<Eight>(), PointeeAlignment::of::<Sixteen>());
        assert_eq!((eight.width(), sixteen.width()), (3, 4), "three bits, and four");
        assert_eq!(PointeeAlignment::least(&[sixteen, eight]).width(), 3, "two pointers, three");
        assert_eq!(PointeeAlignment::least(&[]), PointeeAlignment::NONE, "and no pointer, none");
        assert_eq!(<u64 as Atom>::POINTEE_ALIGNMENT, PointeeAlignment::NONE, "nor an integer");
    }

    #[test]
    fn the_pointer_takes_every_bit_above_the_tags() {
        let word = three_bits(0);
        assert_eq!(word.tag_field_width(), 3, "the tags take three bits");
        assert_eq!(word.pointer_layout().width(), usize::BITS - 3, "the pointer the rest");
        assert!(!word.pointer_layout().is_signed(), "unsigned");
        assert_eq!(word.packed_layout().width(), usize::BITS, "so the word fills its repr");
        assert_eq!(word.tag_width(), 3, "and keeps its tags in three low bits");
    }

    #[test]
    fn a_word_is_nonzero_only_where_its_pointer_is() {
        let word = three_bits(0);
        let never_null = word.range::<*mut u8>(ReprRange::NONZERO);
        assert_eq!(never_null, ReprRange::NONZERO, "every repr but zero");
        assert_eq!(word.range::<*mut u8>(ReprRange::FULL), ReprRange::FULL, "else every repr");
    }

    #[test]
    fn tags_offset_the_pointer_and_split_off_again() {
        let word = three_bits(0);
        let mut node = 0_u64;
        let pointer = ptr::from_mut(&mut node);
        let (tagged, misaligned) = word.pointer_tags(0b101, Tags::EMPTY).set_in(pointer);
        assert_eq!(misaligned, 0, "an aligned pointer");
        assert_eq!(tagged.addr(), pointer.addr() | 0b101, "the tags in the low bits");
        assert_eq!(word.split(tagged), (pointer, 0b101), "and split off again");
        let (untagged, _) = word.split(tagged);
        // SAFETY: `untagged` is `pointer`, offset by the tags and cleared of them, so it keeps its
        // provenance over `node`, which is live.
        #[expect(unsafe_code, reason = "reads through the pointer, so Miri checks its provenance")]
        let read = unsafe { untagged.read() };
        assert_eq!(read, 0, "and reads the value it points to");
    }

    #[test]
    fn a_words_tags_stack_above_its_pointers_own() {
        let word = three_bits(1);
        assert_eq!(word.tag_field_width(), 3, "three tag bits of its own");
        assert_eq!(word.tag_width(), 4, "above the pointer's one");
        assert_eq!(word.pointer_placement().layout().width(), 1, "whose tag a place reaches");
        assert_eq!(word.packed_layout().width(), usize::BITS, "and the pointer fills the rest");
        let pointer = ptr::without_provenance_mut::<u64>(0x1001);
        let (tagged, _) = word.pointer_tags(0b1010, Tags::EMPTY).set_in(pointer);
        assert_eq!(tagged.addr(), 0x100B, "the word's tags beside the pointer's");
        assert_eq!(word.split(tagged), (pointer, 0b1010), "and the pointer's tag stays with it");
    }

    #[test]
    fn a_tag_past_its_width_reaches_no_bit_of_the_pointer() {
        let word = three_bits(0);
        let pointer = ptr::without_provenance_mut::<u64>(0x1000);
        let (tagged, _) = word.pointer_tags(0xFF, Tags::EMPTY).set_in(pointer);
        assert_eq!(tagged.addr(), 0x1007, "only the tag bits");
    }

    #[test]
    fn a_word_of_no_tag_bits_reads_no_address() {
        let word = PointerWordLayout::new(0, PackedLayout::new(&[]));
        let pointer = ptr::without_provenance_mut::<u64>(0x1003);
        let set = word.pointer_tags(0b111, Tags::EMPTY).set_in(pointer);
        assert_eq!(set, (pointer, 0), "nothing to set, nor to find set");
        assert_eq!(word.split(pointer), (pointer, 0), "nor to split off");
    }

    #[test]
    fn a_word_passes_its_tags_on_beside_those_of_the_words_that_hold_it() {
        let inner = three_bits(0);
        let outer = Tags::EMPTY.with(0b1000, 0b1000);
        let pointer = ptr::without_provenance_mut::<u64>(0x1000);
        let (tagged, misaligned) = inner.pointer_tags(0b011, outer).set_in(pointer);
        assert_eq!((tagged.addr(), misaligned), (0x100B, 0), "both words' tags in one offset");
        let misplaced = ptr::without_provenance_mut::<u64>(0x1008);
        let (_, misaligned) = inner.pointer_tags(0b011, outer).set_in(misplaced);
        assert_eq!(misaligned, 0b1000, "and the outer word's bit, found set, in one test");
    }

    #[test]
    #[should_panic(
        expected = "`u8`: the pointer's address has a bit set where its tags go, so it is not \
                    aligned for them"
    )]
    fn a_pointer_with_a_tag_bit_set_is_refused() {
        let word = three_bits(0);
        let pointer = ptr::without_provenance_mut::<u64>(0x1004);
        let (tagged, misaligned) = word.pointer_tags(0, Tags::EMPTY).set_in(pointer);
        assert_eq!(misaligned, 0b100, "the bit found set");
        assert_aligned::<u8>(misaligned);
        panic!("tagged as {tagged:?}, not refused");
    }

    #[test]
    fn tags_fit_a_pointee_aligned_for_them() {
        three_bits(0).assert_tags_fit::<(), *mut Eight>("tags need");
        let above = three_bits(<Tagged<Sixteen> as Atom>::TAG_WIDTH);
        above.assert_tags_fit::<(), Tagged<Sixteen>>("tags need");
    }

    #[test]
    #[should_panic(
        expected = "`()`: tag fields `flag`, `count` need 3 low bits, but `u32` is 4-byte aligned \
                    (2 free bits): shrink the tags, or raise the pointee's alignment with \
                    `#[repr(align(8))]`"
    )]
    fn tags_past_the_pointees_alignment_are_refused() {
        let word = three_bits(0);
        word.assert_tags_fit::<(), *mut u32>("tag fields `flag`, `count` need");
    }

    #[test]
    #[should_panic(expected = "`()`: tag field `wide` needs 32 low bits, but \
                    `atomiks_core::range::pointer_word::tests::Eight` is 8-byte aligned (3 free \
                    bits): shrink the tags, which no alignment holds")]
    fn tags_past_every_alignment_are_refused_without_one() {
        let wide = PackedField::new(ReprRange::<u32>::FULL, 0);
        let word = PointerWordLayout::new(0, PackedLayout::new(&[wide]));
        word.assert_tags_fit::<(), *mut Eight>("tag field `wide` needs");
    }

    #[test]
    #[should_panic(expected = "(1 free bit)")]
    fn one_free_bit_is_one_bit() {
        three_bits(0).assert_tags_fit::<(), *mut u16>("tags need");
    }

    #[test]
    #[should_panic(expected = "`()`: tags need 3 low bits, but \
                    `atomiks_core::range::pointer_word::tests::Tagged<atomiks_core::range::\
                    pointer_word::tests::Eight>` keeps 1 for its own tags and leaves 2 clear \
                    above them: shrink the tags, or raise its pointees' alignment to 16 bytes")]
    fn tags_past_what_a_tagged_pointer_leaves_are_refused() {
        let above = three_bits(<Tagged<Eight> as Atom>::TAG_WIDTH);
        above.assert_tags_fit::<(), Tagged<Eight>>("tags need");
    }
}
