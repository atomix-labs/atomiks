//! A pointer word's `Atom`: its tag fields packed, as a packed struct's fields are, into the low
//! bits its pointer's alignment leaves clear, above the pointer's own tags, or into an integer word
//! of their own beside it; its repr the pointer's, whose provenance it keeps, two pointers' or the
//! pointer's beside that word; and its projection onto every field, whose pointer's place reads
//! the pointer through the word.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{Ident, Type, parse_quote};

use super::field::{PackedFields, PointersWithTags, codec};
use super::layout::{AtomImpl, ImplRepr, LayoutCode};
use super::member_in_expansion;
use super::projection::{FieldSite, ProjectionCode, member_shown};
use super::repr::located_at;
use super::stub::stub;
use crate::model::{Field, Implementor, PointerWord, WordRepr, pointers_among_tags};

/// The code of a pointer word: its tags' code, and its layout's locals, its width, `word_count`,
/// where a constant counts it, each tag's placement, `placement_<index>`, then the word's layout,
/// `layout`, a `PointerWordLayout`.
struct WordCode<'a> {
    /// The word.
    implementor: &'a Implementor,
    /// Its fields.
    word: &'a PointerWord,
    /// The type its pointers are stored as, together: its one pointer field's, or the pair of its
    /// two.
    pointer_type: Type,
    /// Its tag fields' code.
    tag_fields: PackedFields<'a>,
    /// Its layout's locals.
    layout: LayoutCode<'a>,
    /// The name of the local that counts its words, where a constant counts them.
    word_count: Ident,
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
    // ends, the first above the pointer's own tags, or at bit 64 beside an integer word, where
    // `to_repr` packs the tag, each in bits of its own, as the layout says: below the pointer's
    // address, in a layout whose top field is the pointer, so no tag reaches the top and no bit
    // lies above the layout's width to extend; or in the integer word, in a layout of the tags
    // alone, which `join_words` extends as it says. The first pointer's placement is the low bits
    // of the word's repr that are the low bits of the pointer's, its own tags, below the word's; a
    // second's, those of the second word, at bit 64. `from_repr` splits the word's tags off and
    // decodes the pointers and each tag alone, refusing nothing else but an integer word's bits
    // that extend no tag, so a repr decodes wherever each field's bits decode and the layout
    // extends them; and `field` returns the field.
    let items = projection.implementations(code.field_sites());
    let implemented = code.layout.implement(word.fields(), code.atom_impl(items));
    let structure = projection.structure();
    match word.repr {
        WordRepr::One | WordRepr::Selected => quote!(#implemented #structure),
        // Two words whatever the fields: written where atomix holds two words, else refused once.
        WordRepr::Pointers | WordRepr::IntegerWord => {
            let atomix = &implementor.atomix;
            let name = implementor.ident.to_string();
            let stub = stub(implementor);
            quote! {
                #atomix::__private::in_two_words! {
                    #name { #implemented #structure } else { #stub }
                }
            }
        },
    }
}

impl<'a> WordCode<'a> {
    /// The code of `word`, of `implementor`, naming its locals at `def_site`.
    fn new(implementor: &'a Implementor, word: &'a PointerWord, def_site: Span) -> Self {
        let atomix = &implementor.atomix;
        let private = quote!(#atomix::__private);
        let pointer_types = word.pointer_fields().map(|Field { ty, .. }| ty);
        let pointer_type: Type = if let [_] = word.pointers.as_slice() {
            parse_quote!(#(#pointer_types)*)
        } else {
            parse_quote!((#(#pointer_types),*))
        };
        // The pointer's tag width alone, never its alignment, so the word may be held in its own
        // pointee.
        let pointer_tag_width = quote!(<#pointer_type as #atomix::Atom>::TAG_WIDTH);
        let mut layout = LayoutCode::new(implementor, def_site).stored_as_pointers([&pointer_type]);
        let word_count = Ident::new("word_count", def_site);
        let start = match word.repr {
            WordRepr::One | WordRepr::Pointers => pointer_tag_width.clone(),
            WordRepr::IntegerWord => {
                quote!(#private::PointerWordLayout::tag_fields_offset(2, #pointer_tag_width))
            },
            WordRepr::Selected => {
                let tags = word.tag_fields.iter().map(|Field { ty, .. }| {
                    quote!(#private::PackedField::new(<#ty as #atomix::Atom>::REPRS, 0))
                });
                layout.push(
                    &word_count,
                    quote!(::core::primitive::u32),
                    quote! {
                        #private::PointerWordLayout::word_count(#pointer_tag_width, &[#(#tags),*])
                    },
                );
                quote!(#private::PointerWordLayout::tag_fields_offset(#word_count, #pointer_tag_width))
            },
        };
        let tag_fields =
            PackedFields::new(atomix, &word.tag_fields, "placement", def_site).starting_at(start);
        for (placement, placed) in tag_fields.placed() {
            layout.push(placement, quote!(#private::PackedField), placed);
        }
        let name = layout.name().clone();
        let placements = tag_fields.placements();
        let tag_layout = quote!(#private::PackedLayout::new(&[#(#placements),*]));
        let word_layout = match word.repr {
            WordRepr::One | WordRepr::Pointers => {
                quote!(#private::PointerWordLayout::new(#pointer_tag_width, #tag_layout))
            },
            WordRepr::IntegerWord => {
                quote!(#private::PointerWordLayout::in_words(2, #pointer_tag_width, #tag_layout))
            },
            WordRepr::Selected => quote! {
                #private::PointerWordLayout::in_words(#word_count, #pointer_tag_width, #tag_layout)
            },
        };
        layout.push(&name, quote!(#private::PointerWordLayout), word_layout);
        Self { implementor, word, pointer_type, tag_fields, layout, word_count, def_site }
    }

    /// Whether the word may lay its tags out in an integer word beside its pointer, so its code
    /// joins and splits words.
    const fn may_have_integer_word(&self) -> bool {
        matches!(self.word.repr, WordRepr::IntegerWord | WordRepr::Selected)
    }

    /// Where each field lies, in declaration order, as its `HasPackedField` says: a tag at its
    /// placement, the first pointer in the low bits that hold its own tags, and a second pointer
    /// in the second word's. Each lies in the word's layout, whose top field is the pointer, or,
    /// beside an integer word, the tags', so no field reaches the repr's top bit but the last tag
    /// of an integer word that ends at bit 128, which `reaches_top` finds where the word has no
    /// parameters, as a packed struct's top field.
    fn field_sites(&self) -> Vec<FieldSite> {
        let atomix = &self.implementor.atomix;
        let layout = self.layout.local(self.layout.name());
        let below_top = quote!(#atomix::__private::Reach<false>);
        let tag_sites = self.tag_fields.placements().iter().map(|placement| {
            let reach = if self.layout.is_generic() {
                below_top.clone()
            } else {
                let alias = self.layout.repr_alias();
                quote!(#atomix::__private::Reach<{
                    #atomix::__private::reaches_top::<#alias>(#placement)
                }>)
            };
            (self.layout.local(placement), reach)
        });
        let pointer_sites = self.word.pointers.iter().enumerate().map(|(rank, (index, field))| {
            let placement = if rank == 0 {
                quote!(#layout.pointer_placement())
            } else {
                let ty = &field.ty;
                quote! {
                    #atomix::__private::PointerWordLayout::second_pointer_placement(
                        <#ty as #atomix::Atom>::TAG_WIDTH,
                    )
                }
            };
            (*index, (placement, below_top.clone()))
        });
        pointers_among_tags(pointer_sites, tag_sites)
            .into_iter()
            .map(|(placement, reach)| FieldSite {
                placement,
                layout: quote!(#layout.packed_layout()),
                reach,
            })
            .collect()
    }

    /// The checks of the word: where a thin pointer field's type names no parameter, that it is
    /// stored as a pointer, located at the type, the one check that asks it, so a field marked a
    /// pointer that is none is refused once; and that the tags of the word, or of an instance of
    /// it, fit the bits its pointer leaves clear above its own tags, or the integer word beside it,
    /// located at the word's name where it has no parameters, and wherever an instance's code is
    /// built where it has.
    fn checks(&self) -> Vec<TokenStream> {
        let atomix = &self.implementor.atomix;
        let stored_as_pointers = self
            .word
            .pointer_fields()
            .filter(|pointer| !pointer.is_generic && !pointer.is_wide_pointer)
            .map(|Field { ty, .. }| {
                located_at(quote!(#atomix::__private::assert_pointer::<#ty>()), ty.span())
            });
        stored_as_pointers.chain([self.tags_fit_check()]).collect()
    }

    /// The check that the tags of the word, or of an instance of it, fit the bits its pointer
    /// leaves clear above its own tags, or the integer word beside it: concrete, located at the
    /// word's name; generic, wherever an instance's code is built.
    fn tags_fit_check(&self) -> TokenStream {
        let Implementor { ident, generics, .. } = self.implementor;
        let (_, ty_generics, _) = generics.split_for_impl();
        let (pointer, layout) = (&self.pointer_type, self.layout.name());
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

    /// `Atom` for the word, beside `items`: of its pointers' repr, or that beside an integer word,
    /// its tags' width, and the strongest validity its fields promise, or, for a generic word, that
    /// zero decodes, where each field's does.
    fn atom_impl(&self, items: TokenStream) -> AtomImpl {
        let atomix = &self.implementor.atomix;
        let private = quote!(#atomix::__private);
        let pointer = &self.pointer_type;
        let layout = self.layout.local(self.layout.name());
        let validity = if self.layout.is_generic() {
            self.word.fields().collect::<Vec<_>>().into_iter().rev().fold(
                quote!(#atomix::validity::ZeroValid),
                |beside, Field { ty, .. }| {
                    quote! {
                        <<#ty as #atomix::Atom>::Validity as #atomix::validity::Validity>
                            ::ZeroValidityWith<#beside>
                    }
                },
            )
        } else {
            let (promises, alias) = (self.tag_fields.validity(), self.layout.repr_alias());
            let pointers = self.word.pointer_fields().map(|Field { ty, .. }| {
                quote! {
                    .with_field::<<#ty as #atomix::Atom>::Validity>(#layout.pointer_layout())
                }
            });
            self.layout.selected_validity(&quote! {
                #promises
                    #(#pointers)*
                    .code(#layout.width::<#alias>(), <#alias as #atomix::Primitive>::BITS)
            })
        };
        let pointer_repr = quote!(<#pointer as #atomix::Atom>::Repr);
        let repr = match self.word.repr {
            WordRepr::One | WordRepr::Pointers => pointer_repr,
            WordRepr::IntegerWord => {
                quote!(#atomix::DoubleWord<#pointer_repr, ::core::primitive::usize>)
            },
            WordRepr::Selected => {
                let word_count = &self.word_count;
                quote!(<#private::Words<{ #word_count }> as #private::SelectPointerWordRepr<#pointer_repr>>::Repr)
            },
        };
        let named = if self.layout.is_generic() {
            repr.clone()
        } else {
            self.layout.repr_alias().to_token_stream()
        };
        AtomImpl {
            items,
            repr: ImplRepr::Pointer { ty: repr, tag_width: quote!(#layout.tag_width()) },
            validity,
            reprs: quote!(#layout.range(<#pointer as #atomix::Atom>::REPRS)),
            checks: self.checks(),
            conversions: self.conversions(&named),
        }
    }

    /// The codec `codec_name` of the pointers together, `__private`'s or `Atom`'s `method`, as
    /// [`codec`] gives a field's.
    fn pointer_codec(&self, codec_name: &str, method: &str) -> TokenStream {
        let atomix = &self.implementor.atomix;
        match self.word.pointers.as_slice() {
            [(_, pointer)] => codec(atomix, pointer, codec_name, method),
            pointers => {
                let pointer = &self.pointer_type;
                if pointers.iter().any(|(_, field)| field.is_generic) {
                    let method = Ident::new(method, Span::call_site());
                    quote!(<#pointer as #atomix::Atom>::#method)
                } else {
                    let codec_name = Ident::new(codec_name, Span::call_site());
                    quote!(#atomix::__private::#codec_name::<#pointer>)
                }
            },
        }
    }

    /// `to_repr`, `to_tagged_repr`, `from_repr` and `from_repr_unchecked` of the word, whose repr
    /// is `repr`, each started by binding the layout's locals where they are an instance's.
    fn conversions(&self, repr: &TokenStream) -> TokenStream {
        let Implementor { atomix, .. } = self.implementor;
        let private = quote!(#atomix::__private);
        let layout = self.layout.name();
        // No conversion reads the alignment: only the check does.
        let bind = self.layout.bind(&[self.layout.alignment_name()]);
        let local = |name| Ident::new(name, self.def_site);
        let (repr_value, pointer_value, bits, tags, misaligned) =
            (local("repr"), local("pointer"), local("bits"), local("tags"), local("misaligned"));
        let field_read = |Field { member, .. }: &Field| {
            let member = member_in_expansion(member);
            quote!(self.#member)
        };
        let members = self.word.pointer_fields().map(field_read);
        let pointers = if let [_] = self.word.pointers.as_slice() {
            quote!(#(#members)*)
        } else {
            quote!((#(#members),*))
        };
        let tag_bits =
            self.tag_fields.encode(self.tag_fields.fields().iter().map(|field| field_read(field)));
        let to_tagged = self.pointer_codec("to_tagged_repr", "to_tagged_repr");
        let to_tagged_self = if self.layout.is_generic() {
            quote!(<Self as #atomix::Atom>::to_tagged_repr)
        } else {
            quote!(#private::to_tagged_repr::<Self>)
        };
        let pointer_indices: Vec<usize> =
            self.word.pointers.iter().map(|(index, _)| *index).collect();
        let pointers_with_tags = PointersWithTags {
            fields: self.word.fields().collect(),
            pointer_indices: &pointer_indices,
            tag_fields: &self.tag_fields,
        };
        let decoded = pointers_with_tags.decode(
            &quote!(Self),
            &self.pointer_codec("from_repr", "from_repr"),
            &pointer_value.to_token_stream(),
            &bits,
            self.def_site,
        );
        let decoded_unchecked = pointers_with_tags.decode_unchecked(
            &quote!(Self),
            &self.pointer_codec("from_repr_unchecked", "from_repr_unchecked"),
            &pointer_value.to_token_stream(),
            &bits,
            self.def_site,
        );
        let (to_tagged_repr, split, split_unchecked) = if self.may_have_integer_word() {
            (
                quote! {
                    let #bits = #tag_bits;
                    let (#pointer_value, #misaligned) =
                        #to_tagged(#pointers, #layout.pointer_tags(#bits, #tags));
                    (#layout.join_words(#pointer_value, #bits), #misaligned)
                },
                quote! {
                    let ::core::option::Option::Some((#pointer_value, #bits)) =
                        #layout.split_words(#repr_value)
                    else {
                        return ::core::option::Option::None;
                    };
                },
                quote! {
                    let (#pointer_value, #bits) =
                        unsafe { #layout.split_words(#repr_value).unwrap_unchecked() };
                },
            )
        } else {
            (
                quote!(#to_tagged(#pointers, #layout.pointer_tags(#tag_bits, #tags))),
                quote!(let (#pointer_value, #bits) = #layout.split(#repr_value);),
                quote!(let (#pointer_value, #bits) = #layout.split(#repr_value);),
            )
        };
        // The impl keeps each promise of `Atom`: each tag packs into bits of its own, among the
        // word's tag bits, which `to_tagged_repr` passes on, beside those of any word that holds
        // this one, to the pointer field, which sets them as `Tags::set_in` does, as its own impl
        // promises, so the repr is the pointer's offset by the tags, keeping its provenance, or,
        // beside an integer word, the pointer's beside the tags' bits, which `join_words` writes;
        // `to_repr` is that repr with no tags beside, which refuses a pointer that had a tag bit
        // set, and lies in the range, every repr, or every one but zero where the pointer is never
        // null; `from_repr` splits the same bits back off and decodes the pointers and each tag as
        // its field does, by the repr alone, so the repr decodes as the value, and each repr that
        // decodes is one value's, the pointers taking every bit but the word's tags', and an
        // integer word's bits above the tags extending the last as `split_words` checks;
        // `from_repr_unchecked` splits alike and decodes each unchecked, which gives what
        // `from_repr` does, since a repr that decodes is one `split_words` accepts; the validity is
        // what the fields promise of the bits they take, or, for a generic word, of zero alone; and
        // the value may cross threads, as the impl checks or bounds each field but the pointers,
        // whose own impls promise that they may.
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
                #to_tagged_repr
            }
            #[inline]
            fn from_repr(#repr_value: #repr) -> ::core::option::Option<Self> {
                #bind
                #split
                #decoded
            }
            #[inline]
            unsafe fn from_repr_unchecked(#repr_value: #repr) -> Self {
                // The caller's repr decodes, so its pointers and each tag do.
                #bind
                #split_unchecked
                #decoded_unchecked
            }
        }
    }
}
