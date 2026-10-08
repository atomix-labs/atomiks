//! A value's fields: how it is built of them, and for a packed value, from bit 0 in declaration
//! order, where each lies and how each converts to and from its repr's bits.

use core::iter;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::spanned::Spanned;
use syn::{Ident, Member, Path};

use super::member_in_expansion;
use super::repr::located_at;
use crate::model::{Field, pointers_among_tags};

/// The code of a packed value's fields, which names each field's placement, a `PackedField`,
/// `<prefix>_<index>`, and its value `value_<index>`, both at the derive's definition site: the
/// derive names nothing of the user's there, so no name of the user's clashes with either.
pub(super) struct PackedFields<'a> {
    /// The path to atomix.
    atomix: &'a Path,
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
    /// The code of `fields`, of a value that names atomix `atomix`, naming each placement after
    /// `prefix` and its locals at `def_site`.
    pub(super) fn new<F: IntoIterator<Item = &'a Field>>(
        atomix: &'a Path, fields: F, prefix: &str, def_site: Span,
    ) -> Self {
        let fields: Vec<&'a Field> = fields.into_iter().collect();
        let count = fields.len();
        let names = |noun: &str| -> Vec<Ident> {
            (0..count).map(|index| format_ident!("{noun}_{index}", span = def_site)).collect()
        };
        Self { atomix, placements: names(prefix), values: names("value"), fields, start: quote!(0) }
    }

    /// The fields placed from `start`, a constant expression, rather than from bit 0: a pointer
    /// word's tags, above its pointer's own.
    pub(super) fn starting_at(self, start: TokenStream) -> Self {
        Self { start, ..self }
    }

    /// The path to atomix.
    pub(super) const fn atomix(&self) -> &'a Path {
        self.atomix
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
        let atomix = self.atomix;
        let offsets = iter::once(self.start.clone())
            .chain(self.placements.iter().map(|before| quote!(#before.next_offset())));
        self.iter().zip(offsets).map(move |((field, placement), offset)| {
            let ty = laid_out_type(field);
            let placed = quote! {
                #atomix::__private::PackedField::new(<#ty as #atomix::Atom>::REPRS, #offset)
            };
            (placement, placed)
        })
    }

    /// The layout the placements make, a `PackedLayout`.
    pub(super) fn layout(&self) -> TokenStream {
        let (atomix, placements) = (self.atomix, &self.placements);
        quote!(#atomix::__private::PackedLayout::new(&[#(#placements),*]))
    }

    /// What the fields promise of their bits, a `PackedValidity`, each field's validity folded in
    /// with its layout.
    pub(super) fn validity(&self) -> TokenStream {
        let atomix = self.atomix;
        let fields = self.iter().map(|(field, placement)| {
            let ty = laid_out_type(field);
            quote!(.with_field::<<#ty as #atomix::Atom>::Validity>(#placement.layout()))
        });
        quote!(#atomix::__private::PackedValidity::EMPTY #(#fields)*)
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
    /// A field laid out alike in every instance, a marker among them, goes through `__private`,
    /// whose bound is `const`, so the crate that derives needs no `const_trait_impl`; any other
    /// through the trait, the only calls that keep the impl's `[const]` bound.
    fn field_bits<V: ToTokens>(&self, field: &Field, value: &V) -> TokenStream {
        let atomix = self.atomix;
        let ty = codec_type(field);
        if field.is_laid_out_per_instance() {
            let repr = quote!(<<#ty as #atomix::Atom>::Repr as #atomix::ExactBits>);
            quote!(#repr::to_bits(<#ty as #atomix::Atom>::to_repr(#value)))
        } else {
            // Located at the field's type, where a refusal of its repr points.
            located_at(quote!(#atomix::__private::to_bits::<#ty>(#value)), field.ty.span())
        }
    }

    /// The value of `field` whose repr's bits are `bits`: `__private`'s `codec` where it is laid
    /// out alike in every instance, else `Atom`'s `method` of the repr they are, as
    /// [`field_bits`](Self::field_bits) says.
    pub(super) fn field_value(
        &self, field: &Field, bits: &TokenStream, codec: &str, method: &str,
    ) -> TokenStream {
        let atomix = self.atomix;
        let ty = codec_type(field);
        if field.is_laid_out_per_instance() {
            let method = Ident::new(method, Span::call_site());
            let repr = quote!(<<#ty as #atomix::Atom>::Repr as #atomix::Primitive>);
            quote!(<#ty as #atomix::Atom>::#method(#repr::from_bits(#bits)))
        } else {
            let codec = Ident::new(codec, Span::call_site());
            quote!(#atomix::__private::#codec::<#ty>(#bits))
        }
    }
}

/// The type a layout reads `field`'s reprs of: its own, or a marker's, `PhantomData<()>`, which are
/// every marker's, so that a constant names no parameter a marker does.
///
/// A marker's codecs take its own type, through `__private`, whose `const` bound every marker
/// meets, so the crate that derives needs no `const_trait_impl` for one that names a parameter.
fn laid_out_type(field: &Field) -> TokenStream {
    if field.is_marker() {
        quote!(::core::marker::PhantomData<()>)
    } else {
        field.ty.to_token_stream()
    }
}

/// The type `field`'s codecs convert: its own, or for a marker core's `PhantomData` of what the
/// field's value infers, so a field written `PhantomData` that is no marker, by an alias or
/// `use … as`, fails to build rather than take a marker's layout.
fn codec_type(field: &Field) -> TokenStream {
    if field.is_marker() {
        quote!(::core::marker::PhantomData<_>)
    } else {
        field.ty.to_token_stream()
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
pub(super) fn codec(atomix: &Path, field: &Field, codec: &str, method: &str) -> TokenStream {
    let ty = &field.ty;
    if field.is_generic {
        let method = Ident::new(method, Span::call_site());
        quote!(<#ty as #atomix::Atom>::#method)
    } else {
        private_codec(atomix, field, codec)
    }
}

/// `__private`'s `codec` of `field`, whose type names no parameter, located at the type.
pub(super) fn private_codec(atomix: &Path, field: &Field, codec: &str) -> TokenStream {
    let (ty, codec) = (&field.ty, Ident::new(codec, Span::call_site()));
    located_at(quote!(#atomix::__private::#codec::<#ty>), ty.span())
}

/// One pointer or two beside their tags, a pointer word's fields or a pointer enum's variant's,
/// decoded from the pointers' repr and the tags' bits, as a layout's `split` or `split_words` gives
/// them: two together, as a pair, from a double word.
pub(super) struct PointersWithTags<'a, 'f> {
    /// Each field, the pointers among the tags, in declaration order.
    pub(super) fields: Vec<&'a Field>,
    /// The pointers' indices among them, ascending: one, or two decoded as a pair.
    pub(super) pointer_indices: &'f [usize],
    /// The tag fields' code.
    pub(super) tag_fields: &'f PackedFields<'a>,
}

impl PointersWithTags<'_, '_> {
    /// What `constructor` builds of the pointers `decode` gives of `pointer`, and each tag read
    /// back from `bits`: `Some` where each decodes, else `None`.
    ///
    /// Each is decoded in declaration order, the pointers where the first lies.
    pub(super) fn decode(
        &self, constructor: &TokenStream, decode: &TokenStream, pointer: &TokenStream,
        bits: &Ident, def_site: Span,
    ) -> TokenStream {
        let some = quote!(::core::option::Option::Some);
        let value = |index| format_ident!("value_{index}", span = def_site);
        let pointer_values = self.pointer_indices.iter().copied().map(value);
        let pointers = if let [_] = self.pointer_indices {
            quote!(#some(#(#pointer_values)*))
        } else {
            quote!(#some((#(#pointer_values),*)))
        };
        let tags =
            self.tag_fields.iter().zip(self.tag_indices()).map(|((tag, placement), index)| {
                let decoded = self.tag_fields.field_value(
                    tag,
                    &quote!(#placement.unpack(#bits)),
                    "from_bits",
                    "from_repr",
                );
                let value = value(index);
                (decoded, quote!(#some(#value)))
            });
        let first = self.pointer_indices.iter().take(1);
        let pointers = first.map(|index| (*index, (quote!(#decode(#pointer)), pointers.clone())));
        let (decoded, patterns): (Vec<TokenStream>, Vec<TokenStream>) =
            pointers_among_tags(pointers, tags).into_iter().unzip();
        let values = (0..self.fields.len()).map(value);
        let built = build(constructor, self.fields.iter().copied(), values);
        quote! {
            match (#(#decoded,)*) {
                (#(#patterns,)*) => #some(#built),
                _ => ::core::option::Option::None,
            }
        }
    }

    /// What `constructor` builds of the pointers `decode_unchecked` gives of `pointer`, and each
    /// tag read back from `bits`, each unchecked: the fields of a repr that decodes.
    pub(super) fn decode_unchecked(
        &self, constructor: &TokenStream, decode_unchecked: &TokenStream, pointer: &TokenStream,
        bits: &Ident, def_site: Span,
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
        let decoded_pointers = quote!(unsafe { #decode_unchecked(#pointer) });
        if let [index] = self.pointer_indices {
            let decoded = pointers_among_tags([(*index, decoded_pointers)], tags);
            return build(constructor, self.fields.iter().copied(), decoded);
        }
        let values: Vec<Ident> = self
            .pointer_indices
            .iter()
            .map(|index| format_ident!("value_{index}", span = def_site))
            .collect();
        let pointers =
            self.pointer_indices.iter().copied().zip(values.iter().map(ToTokens::to_token_stream));
        let decoded = pointers_among_tags(pointers, tags);
        let built = build(constructor, self.fields.iter().copied(), decoded);
        quote! {{
            let (#(#values),*) = #decoded_pointers;
            #built
        }}
    }

    /// The tags' indices among the fields, ascending.
    fn tag_indices(&self) -> impl Iterator<Item = usize> {
        (0..self.fields.len()).filter(|index| !self.pointer_indices.contains(index))
    }
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
        let members = fields.map(|field| member_in_expansion(&field.member));
        quote!(#constructor { #(#members: #values),* })
    }
}
