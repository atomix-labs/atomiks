//! A pointer word's `Atom`: its tag fields packed, as a packed struct's fields are, into the low
//! bits its pointer's alignment leaves clear, above the pointer's own tags, and its repr the
//! pointer's, whose provenance it keeps; and its projection onto every field, whose pointer's place
//! reads the pointer through the word.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::Ident;
use syn::spanned::Spanned;

use super::field::{PackedFields, PointerWithTags, codec, pointer_among_tags};
use super::layout::{AtomImpl, ImplRepr, LayoutCode};
use super::projection::{FieldSite, ProjectionCode, member_shown};
use super::repr::located_at;
use crate::model::{Field, Implementor, PointerWord};

/// The code of a pointer word: its tags' code, and its layout's locals, each tag's placement,
/// `placement_<index>`, then the word's layout, `layout`, a `PointerWordLayout`.
struct WordCode<'a> {
    /// The word.
    implementor: &'a Implementor,
    /// Its fields.
    word: &'a PointerWord,
    /// Its tag fields' code.
    tag_fields: PackedFields<'a>,
    /// Its layout's locals.
    layout: LayoutCode<'a>,
    /// The derive's definition site, where the locals it names take their names.
    def_site: Span,
}

/// `Atom` for the pointer word, in a block beside its layout and its projection's impls, and the
/// projection beside it.
///
/// The layout is constants where the word has no parameters, else a function each instance
/// evaluates, beside another its code calls, which checks that the tags fit the bits its pointer
/// leaves clear.
pub(crate) fn pointer_word(
    implementor: &Implementor, word: &PointerWord, def_site: Span,
) -> TokenStream {
    let code = WordCode::new(implementor, word, def_site);
    let projection = ProjectionCode::new(implementor, word.fields(), def_site);
    // Each `HasPackedField` keeps its promise: its type parameter is the field's type; a tag's
    // placement is `PackedField::new` of that type's reprs, at the offset where the tag before it
    // ends, the first above the pointer's own tags, where `to_repr` packs the tag, each in bits of
    // its own below the pointer's address, as the layout, whose top field is the pointer, says, so
    // no tag reaches the top and no bit lies above the layout's width to extend; the pointer's
    // placement is the low bits of the word's repr that are the low bits of the pointer's, its own
    // tags, below the word's; `from_repr` splits the word's tags off and decodes the pointer and
    // each tag alone, refusing nothing else, so a repr decodes wherever each field's bits decode;
    // and `field` returns the field.
    let items = projection.implementations(code.field_sites());
    let implemented = code.layout.implement(word.fields(), code.atom_impl(items));
    let structure = projection.structure();
    quote!(#implemented #structure)
}

impl<'a> WordCode<'a> {
    /// The code of `word`, of `implementor`, naming its locals at `def_site`.
    fn new(implementor: &'a Implementor, word: &'a PointerWord, def_site: Span) -> Self {
        let atomiks = &implementor.atomiks;
        let private = quote!(#atomiks::__private);
        let pointer = &word.pointer.ty;
        // The pointer's tag width alone, never its alignment, so the word may be held in its own
        // pointee.
        let pointer_tag_width = quote!(<#pointer as #atomiks::Atom>::TAG_WIDTH);
        let tag_fields = PackedFields::new(atomiks, &word.tag_fields, "placement", def_site)
            .starting_at(pointer_tag_width.clone());
        let mut layout = LayoutCode::new(implementor, def_site).stored_as_pointers([pointer]);
        for (placement, placed) in tag_fields.placed() {
            layout.push(placement, quote!(#private::PackedField), placed);
        }
        let name = layout.name().clone();
        let placements = tag_fields.placements();
        layout.push(
            &name,
            quote!(#private::PointerWordLayout),
            quote! {
                #private::PointerWordLayout::new(
                    #pointer_tag_width,
                    #private::PackedLayout::new(&[#(#placements),*]),
                )
            },
        );
        Self { implementor, word, tag_fields, layout, def_site }
    }

    /// Where each field lies, in declaration order, as its `HasPackedField` says: a tag at its
    /// placement, the pointer in the low bits that hold its own tags, each in the word's layout,
    /// whose top field is the pointer, so no field reaches the repr's top bit.
    fn field_sites(&self) -> Vec<FieldSite> {
        let atomiks = &self.implementor.atomiks;
        let layout = self.layout.local(self.layout.name());
        let tag_placements =
            self.tag_fields.placements().iter().map(|placement| self.layout.local(placement));
        let placements = pointer_among_tags(
            self.word.pointer_index,
            quote!(#layout.pointer_placement()),
            tag_placements,
        );
        placements
            .into_iter()
            .map(|placement| FieldSite {
                placement,
                layout: quote!(#layout.packed_layout()),
                reach: quote!(#atomiks::__private::Reach<false>),
            })
            .collect()
    }

    /// The checks of the word: where its pointer field's type names no parameter, that it is
    /// stored as a pointer, located at the type, the one check that asks it, so a field marked
    /// a pointer that is none is refused once; and that the tags of the word, or of an instance of
    /// it, fit the bits its pointer leaves clear above its own tags, located at the word's name
    /// where it has no parameters, and wherever an instance's code is built where it has.
    fn checks(&self) -> Vec<TokenStream> {
        let pointer = &self.word.pointer;
        let atomiks = &self.implementor.atomiks;
        let stored_as_a_pointer = (!pointer.is_generic).then(|| {
            let ty = &pointer.ty;
            located_at(quote!(#atomiks::__private::assert_pointer::<#ty>()), ty.span())
        });
        stored_as_a_pointer.into_iter().chain([self.tags_fit_check()]).collect()
    }

    /// The check that the tags of the word, or of an instance of it, fit the bits its pointer
    /// leaves clear above its own tags: concrete, located at the word's name; generic, wherever
    /// an instance's code is built.
    fn tags_fit_check(&self) -> TokenStream {
        let Implementor { ident, generics, .. } = self.implementor;
        let (_, ty_generics, _) = generics.split_for_impl();
        let (pointer, layout) = (&self.word.pointer.ty, self.layout.name());
        let named = self
            .word
            .tag_fields
            .iter()
            .map(|Field { member, .. }| format!("`{}`", member_shown(member)))
            .collect::<Vec<_>>()
            .join(", ");
        let tags_named = if self.word.tag_fields.len() == 1 {
            format!("tag field {named} needs")
        } else {
            format!("tag fields {named} need")
        };
        let check = quote!(#layout.assert_tags_fit::<#ident #ty_generics, #pointer>(#tags_named));
        if self.layout.is_generic() { check } else { located_at(check, ident.span()) }
    }

    /// `Atom` for the word, beside `items`: of the pointer's repr, its tags' width, and the
    /// strongest validity its fields promise, or, for a generic word, that zero decodes, where each
    /// field's does.
    fn atom_impl(&self, items: TokenStream) -> AtomImpl {
        let atomiks = &self.implementor.atomiks;
        let pointer = &self.word.pointer.ty;
        let layout = self.layout.local(self.layout.name());
        let validity = if self.layout.is_generic() {
            self.word.fields().collect::<Vec<_>>().into_iter().rev().fold(
                quote!(#atomiks::validity::ZeroValid),
                |beside, Field { ty, .. }| {
                    quote! {
                        <<#ty as #atomiks::Atom>::Validity as #atomiks::validity::Validity>
                            ::ZeroValidityWith<#beside>
                    }
                },
            )
        } else {
            let (promises, alias) = (self.tag_fields.validity(), self.layout.repr_alias());
            let bits = quote!(<#alias as #atomiks::Primitive>::BITS);
            self.layout.selected_validity(&quote! {
                #promises
                    .with_field::<<#pointer as #atomiks::Atom>::Validity>(#layout.pointer_layout())
                    .code(#bits, #bits)
            })
        };
        let repr = quote!(<#pointer as #atomiks::Atom>::Repr);
        let named = if self.layout.is_generic() {
            repr.clone()
        } else {
            self.layout.repr_alias().to_token_stream()
        };
        AtomImpl {
            items,
            repr: ImplRepr::Pointer { ty: repr, tag_width: quote!(#layout.tag_width()) },
            validity,
            reprs: quote!(#layout.range(<#pointer as #atomiks::Atom>::REPRS)),
            checks: self.checks(),
            conversions: self.conversions(&named),
        }
    }

    /// `to_repr`, `to_tagged_repr`, `from_repr` and `from_repr_unchecked` of the word, whose repr
    /// is `repr`, each started by binding the layout's locals where they are an instance's.
    fn conversions(&self, repr: &TokenStream) -> TokenStream {
        let Implementor { atomiks, .. } = self.implementor;
        let private = quote!(#atomiks::__private);
        let layout = self.layout.name();
        // No conversion reads the alignment: only the check does.
        let bind = self.layout.bind(&[self.layout.alignment_name()]);
        let local = |name| Ident::new(name, self.def_site);
        let (repr_value, pointer_value, bits, tags, misaligned) =
            (local("repr"), local("pointer"), local("bits"), local("tags"), local("misaligned"));
        let pointer = &self.word.pointer;
        let member = &pointer.member;
        let tag_bits = self.tag_fields.encode(
            self.tag_fields.fields().iter().map(|Field { member, .. }| quote!(self.#member)),
        );
        let to_tagged = codec(atomiks, pointer, "to_tagged_repr", "to_tagged_repr");
        let to_tagged_self = if self.layout.is_generic() {
            quote!(<Self as #atomiks::Atom>::to_tagged_repr)
        } else {
            quote!(#private::to_tagged_repr::<Self>)
        };
        let pointer_with_tags = PointerWithTags {
            fields: self.word.fields().collect(),
            pointer_index: self.word.pointer_index,
            tag_fields: &self.tag_fields,
        };
        let decoded = pointer_with_tags.decode(
            &quote!(Self),
            &codec(atomiks, pointer, "from_repr", "from_repr"),
            &pointer_value.to_token_stream(),
            &bits,
            self.def_site,
        );
        let decoded_unchecked = pointer_with_tags.decode_unchecked(
            &quote!(Self),
            &codec(atomiks, pointer, "from_repr_unchecked", "from_repr_unchecked"),
            &pointer_value.to_token_stream(),
            &bits,
        );
        // The impl keeps each promise of `Atom`: each tag packs into bits of its own, among the
        // word's tag bits, which `to_tagged_repr` passes on, beside those of any word that holds
        // this one, to the pointer field, which sets them as `Tags::set_in` does, as its own impl
        // promises, so the repr is the pointer's offset by the tags, keeping its provenance;
        // `to_repr` is that repr with no tags beside, which refuses a pointer that had a tag bit
        // set, and lies in the range, every repr, or every one but zero where the pointer is never
        // null; `from_repr` splits the same bits back off and decodes the pointer and each tag as
        // its field does, by the repr alone, so the repr decodes as the value, and each repr that
        // decodes is one value's, the pointer taking every bit but the word's tags';
        // `from_repr_unchecked` splits alike and decodes each unchecked, which gives what
        // `from_repr` does; the validity is what the fields promise of the bits they take, or, for
        // a generic word, of zero alone; and the value may cross threads, as the impl checks or
        // bounds each field but the pointer, whose own impl promises that it may.
        quote! {
            #[inline]
            fn to_repr(self) -> #repr {
                let (#repr_value, #misaligned) = #to_tagged_self(self, #private::Tags::EMPTY);
                #private::assert_aligned::<Self>(#misaligned);
                #repr_value
            }
            #[inline]
            fn to_tagged_repr(self, #tags: #private::Tags) -> (#repr, ::core::primitive::usize) {
                #bind
                #to_tagged(self.#member, #layout.pointer_tags(#tag_bits, #tags))
            }
            #[inline]
            fn from_repr(#repr_value: #repr) -> ::core::option::Option<Self> {
                #bind
                let (#pointer_value, #bits) = #layout.split(#repr_value);
                #decoded
            }
            #[inline]
            unsafe fn from_repr_unchecked(#repr_value: #repr) -> Self {
                // The caller's repr decodes, so its pointer and each tag do.
                #bind
                let (#pointer_value, #bits) = #layout.split(#repr_value);
                #decoded_unchecked
            }
        }
    }
}
