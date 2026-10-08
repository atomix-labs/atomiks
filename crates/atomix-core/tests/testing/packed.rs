//! A packed struct written out as `#[derive(Atom)]` writes one, for the tests of field operations,
//! which atomix-core runs without the derive: its `Atom` impl, each field's `HasPackedField`, and
//! its projection, with its `Debug`.

/// Declares the packed struct `$name`, in the repr `$repr`, of each `$field` at `$index`, with the
/// `Atom` impl the derive writes, each field's `HasPackedField`, and its projection, `$fields`.
macro_rules! packed {
    (
        $(#[$attribute:meta])*
        $vis:vis struct $name:ident in $repr:ty, projected as $fields:ident {
            $($index:literal => $field:ident: $ty:ty),+ $(,)?
        }
    ) => {
        $(#[$attribute])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis struct $name {
            $(
                #[doc = concat!("The field at ", stringify!($index), ".")]
                $vis $field: $ty,
            )+
        }

        #[doc = concat!("The fields of an atomic `", stringify!($name), "`.")]
        #[derive(Clone, Copy)]
        $vis struct $fields<'a, P: ::atomix_core::FieldPath<Value = $name> + 'a> {
            $(
                #[doc = concat!("The field at ", stringify!($index), ".")]
                $vis $field: &'a ::atomix_core::AtomicField<
                    ::atomix_core::Join<P, ::atomix_core::Field<$name, $index, $ty>>,
                >,
            )+
        }

        const _: () = {
            use ::atomix_core::__private::{
                HasPackedField, PackedField, PackedLayout, Reach, from_bits, from_bits_unchecked,
                project_field, reaches_top, to_bits,
            };
            use ::atomix_core::{Atom, AtomicField, FieldPath, ProjectFields, ReprRange};

            /// Each field's placement, from bit 0 in declaration order.
            const PLACEMENTS: [PackedField; [$($index),+].len()] = {
                let mut placements = [PackedField::new(<u8 as Atom>::REPRS, 0); [$($index),+].len()];
                let mut offset = 0;
                $(
                    placements[$index] = PackedField::new(<$ty as Atom>::REPRS, offset);
                    offset = placements[$index].next_offset();
                )+
                let _ = offset;
                placements
            };

            /// How the value lays out its fields.
            const LAYOUT: PackedLayout = PackedLayout::new(&PLACEMENTS);

            // SAFETY: `to_repr` packs each field in bits of its own and extends the bits above as
            // the layout does, so its repr lies in the layout's range, and `from_repr` reads each
            // field back from those bits, so it decodes as the value; `from_repr` refuses any
            // other bits above and any field that does not decode, by the repr alone;
            // `from_repr_unchecked` reads the same bits back and decodes each field unchecked,
            // which gives what `from_repr` does; `Partial` promises no repr; and each field, an
            // `Atom`, may cross threads, so the struct of them may.
            #[expect(unsafe_code, reason = "the impl the derive writes, by hand")]
            const unsafe impl Atom for $name {
                type Repr = $repr;
                const REPRS: ReprRange<$repr> = LAYOUT.range();
                #[inline]
                fn to_repr(self) -> $repr {
                    LAYOUT.repr(0 $(| PLACEMENTS[$index].pack(to_bits(self.$field)))+)
                }
                #[inline]
                fn from_repr(repr: $repr) -> Option<Self> {
                    let Some(bits) = LAYOUT.canonical_bits(repr) else { return None };
                    match ($(from_bits::<$ty>(PLACEMENTS[$index].unpack(bits)),)+) {
                        ($(Some($field),)+) => Some(Self { $($field),+ }),
                        _ => None,
                    }
                }
                #[inline]
                unsafe fn from_repr_unchecked(repr: $repr) -> Self {
                    let bits = to_bits(repr);
                    Self {
                        $(
                            // SAFETY: the caller's repr decodes, so each field's bits do.
                            $field: unsafe {
                                from_bits_unchecked::<$ty>(PLACEMENTS[$index].unpack(bits))
                            },
                        )+
                    }
                }
            }

            $(
                // SAFETY: `to_repr` packs the field at this placement, each other at its own, and
                // extends the bits above as `LAYOUT` does; `from_repr` decodes wherever each
                // field's bits decode and the bits above extend it; `reaches_top` selects
                // `Reach<true>` only where the field ends at the repr's top bit; and `field`
                // returns the field.
                #[expect(unsafe_code, reason = "the impl the derive writes, by hand")]
                unsafe impl HasPackedField<$index, $ty> for $name {
                    const PLACEMENT: PackedField = PLACEMENTS[$index];
                    const LAYOUT: PackedLayout = LAYOUT;
                    type Reach = Reach<{ reaches_top::<$repr>(PLACEMENTS[$index]) }>;
                    #[inline]
                    fn field(self) -> $ty {
                        self.$field
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
                        $(
                            // SAFETY: the struct's own block declares the field, and the
                            // projection's field that holds the place has the field's visibility,
                            // so the place reaches only code the field is visible to.
                            $field: unsafe { project_field(place) },
                        )+
                    }
                }
            }

            impl<'a, P: FieldPath<Value = $name> + 'a> ::core::fmt::Debug for $fields<'a, P>
            where
                $(
                    &'a AtomicField<::atomix_core::Join<P, ::atomix_core::Field<$name, $index, $ty>>>:
                        ::core::fmt::Debug,
                )+
            {
                fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    formatter
                        .debug_struct(stringify!($fields))
                        $(.field(stringify!($field), &self.$field))+
                        .finish()
                }
            }
        };
    };
}

pub(crate) use packed;
