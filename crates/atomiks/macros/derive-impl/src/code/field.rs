//! A value's fields: how it is built of them, and for a packed value, from bit 0 in declaration
//! order, where each lies and how each converts to and from its repr's bits.

use core::iter;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::spanned::Spanned;
use syn::{Ident, Member, Path};

use super::repr::located_at;
use crate::model::Field;

/// The code of a packed value's fields, which names each field's placement, a `PackedField`,
/// `<prefix>_<index>`, and its value `value_<index>`, both at the derive's definition site: the
/// derive names nothing of the user's there, so no name of the user's clashes with either.
pub(super) struct PackedFields<'a> {
    /// The path to atomiks.
    atomiks: &'a Path,
    /// The fields, in declaration order.
    fields: Vec<&'a Field>,
    /// Each field's placement.
    placements: Vec<Ident>,
    /// Each field's value, bound where it is matched or read back.
    values: Vec<Ident>,
    /// Where the first field lies: bit 0, or above a pointer's own tags.
    start: TokenStream,
}

impl<'a> PackedFields<'a> {
    /// The code of `fields`, of a value that names atomiks `atomiks`, naming each placement after
    /// `prefix` and its locals at `def_site`.
    pub(super) fn new<F: IntoIterator<Item = &'a Field>>(
        atomiks: &'a Path, fields: F, prefix: &str, def_site: Span,
    ) -> Self {
        let fields: Vec<&'a Field> = fields.into_iter().collect();
        let count = fields.len();
        let names = |noun: &str| -> Vec<Ident> {
            (0..count).map(|index| format_ident!("{noun}_{index}", span = def_site)).collect()
        };
        Self {
            atomiks,
            placements: names(prefix),
            values: names("value"),
            fields,
            start: quote!(0),
        }
    }

    /// The fields placed from `start`, a constant expression, rather than from bit 0: a pointer
    /// word's tags, above its pointer's own.
    pub(super) fn starting_at(self, start: TokenStream) -> Self {
        Self { start, ..self }
    }

    /// The path to atomiks.
    pub(super) const fn atomiks(&self) -> &'a Path {
        self.atomiks
    }

    /// Each field, in declaration order.
    pub(super) fn fields(&self) -> &[&'a Field] {
        &self.fields
    }

    /// Each field's placement, in declaration order.
    pub(super) fn placements(&self) -> &[Ident] {
        &self.placements
    }

    /// Each field's value, as a match binds it, in declaration order.
    pub(super) fn values(&self) -> &[Ident] {
        &self.values
    }

    /// Each field beside its placement.
    fn iter(&self) -> impl Iterator<Item = (&'a Field, &Ident)> {
        self.fields.iter().copied().zip(&self.placements)
    }

    /// Each placement beside what it is: a `PackedField` of its field's reprs, at the start for the
    /// first field and where the one before it ends for each other.
    pub(super) fn placed(&self) -> impl Iterator<Item = (&Ident, TokenStream)> {
        let atomiks = self.atomiks;
        let offsets = iter::once(self.start.clone())
            .chain(self.placements.iter().map(|before| quote!(#before.next_offset())));
        self.iter().zip(offsets).map(move |((Field { ty, .. }, placement), offset)| {
            let placed = quote! {
                #atomiks::__private::PackedField::new(<#ty as #atomiks::Atom>::REPRS, #offset)
            };
            (placement, placed)
        })
    }

    /// The layout the placements make, a `PackedLayout`.
    pub(super) fn layout(&self) -> TokenStream {
        let (atomiks, placements) = (self.atomiks, &self.placements);
        quote!(#atomiks::__private::PackedLayout::new(&[#(#placements),*]))
    }

    /// What the fields promise of their bits, a `PackedValidity`, each field's validity folded in
    /// with its layout.
    pub(super) fn validity(&self) -> TokenStream {
        let atomiks = self.atomiks;
        let fields = self.iter().map(|(Field { ty, .. }, placement)| {
            quote!(.with_field::<<#ty as #atomiks::Atom>::Validity>(#placement.layout()))
        });
        quote!(#atomiks::__private::PackedValidity::EMPTY #(#fields)*)
    }

    /// The bits of `values`, the fields', each in its place: zero where there are none.
    pub(super) fn encode<I: IntoIterator<Item: ToTokens>>(&self, values: I) -> TokenStream {
        if self.fields.is_empty() {
            return quote!(0);
        }
        let packed = self.iter().zip(values).map(|((field, placement), value)| {
            let bits = self.field_bits(field, &value);
            quote!(#placement.pack(#bits))
        });
        quote!(#(#packed)|*)
    }

    /// What `constructor` builds of each field read back from `bits`, or `None` where one does not
    /// decode.
    pub(super) fn decode(&self, bits: &Ident, constructor: &TokenStream) -> TokenStream {
        let decoded: Vec<TokenStream> = self
            .iter()
            .map(|(field, placement)| {
                self.field_value(field, &quote!(#placement.unpack(#bits)), "from_bits", "from_repr")
            })
            .collect();
        let values = &self.values;
        let built = build(constructor, self.fields.iter().copied(), values);
        let (some, none) =
            (quote!(::core::option::Option::Some), quote!(::core::option::Option::None));
        // One field's decode is matched alone; several, as a tuple of each.
        if let ([decoded], [value]) = (decoded.as_slice(), values.as_slice()) {
            return quote! {
                match #decoded {
                    #some(#value) => #some(#built),
                    #none => #none,
                }
            };
        }
        quote! {
            match (#(#decoded,)*) {
                (#(#some(#values),)*) => #some(#built),
                _ => #none,
            }
        }
    }

    /// What `constructor` builds of each field read back from `bits` without its check, the bits
    /// of a repr that decodes, so each field's do.
    pub(super) fn decode_unchecked(&self, bits: &Ident, constructor: &TokenStream) -> TokenStream {
        let decoded = self.iter().map(|(field, placement)| {
            let unpacked = quote!(#placement.unpack(#bits));
            let decoded =
                self.field_value(field, &unpacked, "from_bits_unchecked", "from_repr_unchecked");
            quote!(unsafe { #decoded })
        });
        build(constructor, self.fields.iter().copied(), decoded)
    }

    /// The bits of `value`, `field`'s, as its repr's.
    ///
    /// A concrete field's go through `__private`, whose bound is `const`, so the crate that derives
    /// needs no `const_trait_impl`; a generic field's through the trait, the only calls that keep
    /// the impl's `[const]` bound.
    fn field_bits<V: ToTokens>(&self, field: &Field, value: &V) -> TokenStream {
        let (atomiks, ty) = (self.atomiks, &field.ty);
        if field.is_generic {
            let repr = quote!(<<#ty as #atomiks::Atom>::Repr as #atomiks::ExactBits>);
            quote!(#repr::to_bits(<#ty as #atomiks::Atom>::to_repr(#value)))
        } else {
            // Located at the field's type, where a refusal of its repr points.
            located_at(quote!(#atomiks::__private::to_bits::<#ty>(#value)), ty.span())
        }
    }

    /// The value of `field` whose repr's bits are `bits`: `__private`'s `codec` where it is
    /// concrete, else `Atom`'s `method` of the repr they are, as [`field_bits`](Self::field_bits)
    /// says.
    pub(super) fn field_value(
        &self, field: &Field, bits: &TokenStream, codec: &str, method: &str,
    ) -> TokenStream {
        let (atomiks, ty) = (self.atomiks, &field.ty);
        if field.is_generic {
            let method = Ident::new(method, Span::call_site());
            let repr = quote!(<<#ty as #atomiks::Atom>::Repr as #atomiks::Primitive>);
            quote!(<#ty as #atomiks::Atom>::#method(#repr::from_bits(#bits)))
        } else {
            let codec = Ident::new(codec, Span::call_site());
            quote!(#atomiks::__private::#codec::<#ty>(#bits))
        }
    }
}

/// `Self`, of a struct of `PhantomData` markers beside at most one field that holds its value:
/// `markers_before`, then that field, of `value`, where it has one, then `markers_after`, each
/// marker built as `PhantomData`.
pub(super) fn built_with_markers(
    markers_before: &[Field], value: Option<(&Field, &TokenStream)>, markers_after: &[Field],
) -> TokenStream {
    let marker = quote!(::core::marker::PhantomData);
    let (field, value) = value.unzip();
    let fields = markers_before.iter().chain(field).chain(markers_after);
    let values = (markers_before.iter().map(|_| &marker))
        .chain(value)
        .chain(markers_after.iter().map(|_| &marker));
    build(&quote!(Self), fields, values)
}

/// The codec of `field`: `__private`'s `codec`, or `Atom`'s `method`.
///
/// `codec` where its type names no parameter, located at the type, where a refusal of its bound
/// points: its bound is `const`, so the crate that derives needs no `const_trait_impl`. Else
/// `method`, which keeps the impl's `[const]` bound.
pub(super) fn codec(atomiks: &Path, field: &Field, codec: &str, method: &str) -> TokenStream {
    let ty = &field.ty;
    if field.is_generic {
        let method = Ident::new(method, Span::call_site());
        quote!(<#ty as #atomiks::Atom>::#method)
    } else {
        private_codec(atomiks, field, codec)
    }
}

/// `__private`'s `codec` of `field`, whose type names no parameter, located at the type.
pub(super) fn private_codec(atomiks: &Path, field: &Field, codec: &str) -> TokenStream {
    let (ty, codec) = (&field.ty, Ident::new(codec, Span::call_site()));
    located_at(quote!(#atomiks::__private::#codec::<#ty>), ty.span())
}

/// A pointer beside its tags, a pointer word's fields or a pointer enum's variant's, decoded from
/// the pointer field's repr and the tags' bits, as a layout's `split` gives them.
pub(super) struct PointerWithTags<'a, 'f> {
    /// Each field, the pointer among the tags, in declaration order.
    pub(super) fields: Vec<&'a Field>,
    /// The pointer's index among them.
    pub(super) pointer_index: usize,
    /// The tag fields' code.
    pub(super) tag_fields: &'f PackedFields<'a>,
}

impl PointerWithTags<'_, '_> {
    /// What `constructor` builds of the pointer `decode` gives of `pointer`, and each tag read back
    /// from `bits`: `Some` where each decodes, else `None`.
    pub(super) fn decode(
        &self, constructor: &TokenStream, decode: &TokenStream, pointer: &TokenStream,
        bits: &Ident, def_site: Span,
    ) -> TokenStream {
        let tags = self.tag_fields.iter().map(|(tag, placement)| {
            self.tag_fields.field_value(
                tag,
                &quote!(#placement.unpack(#bits)),
                "from_bits",
                "from_repr",
            )
        });
        let decoded = pointer_among_tags(self.pointer_index, quote!(#decode(#pointer)), tags);
        let values: Vec<Ident> = (0..decoded.len())
            .map(|index| format_ident!("value_{index}", span = def_site))
            .collect();
        let built = build(constructor, self.fields.iter().copied(), &values);
        let (some, none) =
            (quote!(::core::option::Option::Some), quote!(::core::option::Option::None));
        quote! {
            match (#(#decoded,)*) {
                (#(#some(#values),)*) => #some(#built),
                _ => #none,
            }
        }
    }

    /// What `constructor` builds of the pointer `decode_unchecked` gives of `pointer`, and each
    /// tag read back from `bits`, each unchecked: the fields of a repr that decodes.
    pub(super) fn decode_unchecked(
        &self, constructor: &TokenStream, decode_unchecked: &TokenStream, pointer: &TokenStream,
        bits: &Ident,
    ) -> TokenStream {
        let tags = self.tag_fields.iter().map(|(tag, placement)| {
            let unpacked = quote!(#placement.unpack(#bits));
            let value = self.tag_fields.field_value(
                tag,
                &unpacked,
                "from_bits_unchecked",
                "from_repr_unchecked",
            );
            quote!(unsafe { #value })
        });
        let pointer = quote!(unsafe { #decode_unchecked(#pointer) });
        let decoded = pointer_among_tags(self.pointer_index, pointer, tags);
        build(constructor, self.fields.iter().copied(), decoded)
    }
}

/// Each part of a struct or a variant of one pointer field beside tags, in declaration order:
/// `tags`, each tag's, with `pointer`'s put at `pointer_index`.
pub(super) fn pointer_among_tags<T, I: IntoIterator<Item = T>>(
    pointer_index: usize, pointer: T, tags: I,
) -> Vec<T> {
    let mut parts: Vec<T> = tags.into_iter().collect();
    parts.insert(pointer_index, pointer);
    parts
}

/// What `constructor` builds of `values`, one for each of `fields` in declaration order: by name,
/// or by position where the fields have no names.
pub(super) fn build<'a, F, V>(constructor: &TokenStream, fields: F, values: V) -> TokenStream
where
    F: IntoIterator<Item = &'a Field>,
    V: IntoIterator<Item: ToTokens>,
{
    let mut fields = fields.into_iter().peekable();
    let values = values.into_iter();
    if let Some(Field { member: Member::Unnamed(_), .. }) = fields.peek() {
        quote!(#constructor(#(#values),*))
    } else {
        let members = fields.map(|field| &field.member);
        quote!(#constructor { #(#members: #values),* })
    }
}
