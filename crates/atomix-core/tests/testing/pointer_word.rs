//! A pointer word written out as `#[derive(Atom)]` writes one, for the tests of tagged pointers,
//! which atomix-core runs without the derive: its `Atom` impl, each field's `HasPackedField`, and
//! its projection, with its `Debug`.

/// Declares the pointer word `$name` of the pointer field `$pointer`, declared first, and each tag
/// field `$tag` at `$index`, with the `Atom` impl the derive writes, each field's `HasPackedField`,
/// and its projection, `$fields`.
macro_rules! pointer_word {
    (
        $(#[$attribute:meta])*
        $vis:vis struct $name:ident, projected as $fields:ident {
            0 => $pointer:ident: $pointer_type:ty,
            $($index:literal => $tag:ident: $ty:ty),+ $(,)?
        }
    ) => {
        $(#[$attribute])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis struct $name {
            /// The pointer.
            $vis $pointer: $pointer_type,
            $(
                #[doc = concat!("The tag field at ", stringify!($index), ".")]
                $vis $tag: $ty,
            )+
        }

        #[doc = concat!("The fields of an atomic `", stringify!($name), "`.")]
        #[derive(Clone, Copy)]
        $vis struct $fields<'a, P: ::atomix_core::FieldPath<Value = $name> + 'a> {
            /// The pointer, whose place reads it through the word, and reaches its own tags.
            $vis $pointer: &'a ::atomix_core::AtomicField<
                ::atomix_core::Join<P, ::atomix_core::Field<$name, 0, $pointer_type>>,
            >,
            $(
                #[doc = concat!("The tag field at ", stringify!($index), ".")]
                $vis $tag: &'a ::atomix_core::AtomicField<
                    ::atomix_core::Join<P, ::atomix_core::Field<$name, $index, $ty>>,
                >,
            )+
        }

        const _: () = {
            use ::atomix_core::__private::{
                HasPackedField, PackedField, PackedLayout, PackedValidity, PointeeAlignment,
                PointerWordLayout, Reach, SelectValidity, Tags, ValidityCode, assert_aligned, from_bits,
                from_bits_unchecked, from_repr, from_repr_unchecked, project_field, to_bits,
                to_tagged_repr,
            };
            use ::atomix_core::{Atom, AtomicField, FieldPath, Primitive, ProjectFields, ReprRange};

            /// Each tag field's placement, in declaration order, above the pointer's own tags: the
            /// field at `$index` at `$index - 1`, the pointer being the field at 0.
            const PLACEMENTS: [PackedField; [$($index),+].len()] = {
                let mut placements = [PackedField::new(<u8 as Atom>::REPRS, 0); [$($index),+].len()];
                let mut offset = <$pointer_type as Atom>::TAG_WIDTH;
                $(
                    placements[$index - 1] = PackedField::new(<$ty as Atom>::REPRS, offset);
                    offset = placements[$index - 1].next_offset();
                )+
                let _ = offset;
                placements
            };

            /// How the word lays out its tags, below its pointer's address: by its pointer's tag
            /// width alone, so the word may be held in its own pointee.
            const WORD_LAYOUT: PointerWordLayout =
                PointerWordLayout::new(<$pointer_type as Atom>::TAG_WIDTH, PackedLayout::new(&PLACEMENTS));

            /// The repr: the pointer field's.
            type Repr = <$pointer_type as Atom>::Repr;

            const _: () = WORD_LAYOUT.assert_tags_fit::<$name, $pointer_type>("tags need");

            // SAFETY: `to_tagged_repr` packs each tag in bits of its own, among the word's tag
            // bits, and passes them on, beside any of a word that holds this one, to the pointer
            // field, which sets them as `Tags::set_in` does, as its own impl promises, so the repr
            // is the pointer's offset by the tags; `to_repr` is that repr without tags beside,
            // refusing a pointer that had a tag bit set, and lies in the range, every repr, or
            // every one but zero where the pointer is never null; `from_repr` splits the same
            // bits back off and decodes the pointer and each tag, by the repr alone, so the repr
            // decodes as the value, and each repr that decodes is one value's, the pointer taking
            // every bit but the word's tags'; `from_repr_unchecked` splits alike and decodes each
            // unchecked, which gives what `from_repr` does; the validity is what the fields
            // promise of the bits they take; and each tag, an `Atom`, and the pointer, as its own
            // impl promises, may cross threads, so the word of them may.
            #[expect(unsafe_code, reason = "the impl the derive writes, by hand")]
            const unsafe impl Atom for $name {
                type Repr = Repr;
                type Validity = <ValidityCode<{
                    PackedValidity::EMPTY
                        .with_field::<<$pointer_type as Atom>::Validity>(WORD_LAYOUT.pointer_layout())
                        $(.with_field::<<$ty as Atom>::Validity>(PLACEMENTS[$index - 1].layout()))+
                        .code(<Repr as Primitive>::BITS, <Repr as Primitive>::BITS)
                }> as SelectValidity>::Validity;
                const REPRS: ReprRange<Repr> = WORD_LAYOUT.range(<$pointer_type as Atom>::REPRS);
                const TAG_WIDTH: u32 = WORD_LAYOUT.tag_width();
                const POINTEE_ALIGNMENT: PointeeAlignment =
                    PointeeAlignment::least(&[<$pointer_type as Atom>::POINTEE_ALIGNMENT]);
                #[inline]
                fn to_repr(self) -> Repr {
                    let (repr, misaligned) = to_tagged_repr::<Self>(self, Tags::EMPTY);
                    assert_aligned::<Self>(misaligned);
                    repr
                }
                #[inline]
                fn to_tagged_repr(self, tags: Tags) -> (Repr, usize) {
                    let bits = 0 $(| PLACEMENTS[$index - 1].pack(to_bits(self.$tag)))+;
                    to_tagged_repr::<$pointer_type>(self.$pointer, WORD_LAYOUT.pointer_tags(bits, tags))
                }
                #[inline]
                fn from_repr(repr: Repr) -> Option<Self> {
                    let (pointer, bits) = WORD_LAYOUT.split(repr);
                    match (
                        from_repr::<$pointer_type>(pointer),
                        $(from_bits::<$ty>(PLACEMENTS[$index - 1].unpack(bits)),)+
                    ) {
                        (Some($pointer), $(Some($tag),)+) => Some(Self { $pointer, $($tag),+ }),
                        _ => None,
                    }
                }
                #[inline]
                unsafe fn from_repr_unchecked(repr: Repr) -> Self {
                    let (pointer, bits) = WORD_LAYOUT.split(repr);
                    Self {
                        // SAFETY: the caller's repr decodes, so its pointer does.
                        $pointer: unsafe { from_repr_unchecked::<$pointer_type>(pointer) },
                        $(
                            // SAFETY: the caller's repr decodes, so each tag's bits do.
                            $tag: unsafe {
                                from_bits_unchecked::<$ty>(PLACEMENTS[$index - 1].unpack(bits))
                            },
                        )+
                    }
                }
            }

            // SAFETY: the pointer's own tags lie in the low bits of the word's repr, below the
            // word's tags, as `pointer_placement` says, and `field` returns the pointer.
            #[expect(unsafe_code, reason = "the impl the derive writes, by hand")]
            unsafe impl HasPackedField<0, $pointer_type> for $name {
                const PLACEMENT: PackedField = WORD_LAYOUT.pointer_placement();
                const LAYOUT: PackedLayout = WORD_LAYOUT.packed_layout();
                type Reach = Reach<false>;
                #[inline]
                fn field(self) -> $pointer_type {
                    self.$pointer
                }
            }

            $(
                // SAFETY: `to_repr` packs the tag at this placement, each other at its own, and
                // the pointer above them all, as the word's layout, whose top field is the pointer,
                // says; `from_repr` decodes wherever the pointer's and each tag's bits decode; no
                // tag reaches the repr's top bit, which is the pointer's; and `field` returns the
                // tag.
                #[expect(unsafe_code, reason = "the impl the derive writes, by hand")]
                unsafe impl HasPackedField<$index, $ty> for $name {
                    const PLACEMENT: PackedField = PLACEMENTS[$index - 1];
                    const LAYOUT: PackedLayout = WORD_LAYOUT.packed_layout();
                    type Reach = Reach<false>;
                    #[inline]
                    fn field(self) -> $ty {
                        self.$tag
                    }
                }
            )+

            // SAFETY: `project` lends only the place of each field, at its path from `place`, each
            // of the field's own visibility.
            #[expect(unsafe_code, reason = "the impl the derive writes, by hand")]
            const unsafe impl ProjectFields for $name {
                type Fields<'a, P: FieldPath<Value = Self> + 'a> = $fields<'a, P> where Self: 'a;
                fn project<P: FieldPath<Value = Self>>(place: &AtomicField<P>) -> $fields<'_, P> {
                    $fields {
                        // SAFETY: as each tag's below.
                        $pointer: unsafe { project_field(place) },
                        $(
                            // SAFETY: the word's own block declares the field, and the
                            // projection's field that holds the place has the field's visibility,
                            // so the place reaches only code the field is visible to.
                            $tag: unsafe { project_field(place) },
                        )+
                    }
                }
            }

            impl<'a, P: FieldPath<Value = $name> + 'a> ::core::fmt::Debug for $fields<'a, P>
            where
                &'a AtomicField<::atomix_core::Join<P, ::atomix_core::Field<$name, 0, $pointer_type>>>:
                    ::core::fmt::Debug,
                $(
                    &'a AtomicField<::atomix_core::Join<P, ::atomix_core::Field<$name, $index, $ty>>>:
                        ::core::fmt::Debug,
                )+
            {
                fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    formatter
                        .debug_struct(stringify!($fields))
                        .field(stringify!($pointer), &self.$pointer)
                        $(.field(stringify!($tag), &self.$tag))+
                        .finish()
                }
            }
        };
    };
}

pub(crate) use pointer_word;
