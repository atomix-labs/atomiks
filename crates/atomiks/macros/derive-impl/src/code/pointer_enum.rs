//! `Atom` for a pointer enum, one variant or more holding a pointer, stored in one `*mut ()`.
//!
//! Each variant's discriminant is its tag, in the low bits above its pointers' own tags; a pointer
//! variant's tag fields lie above the tag, and its pointer's address above them, its provenance
//! kept; a data variant's fields lie above the bits its pointers' alignment leaves clear, an
//! address without provenance; a unit is its tag alone. One unit beside one pointer that is never
//! null takes null instead, and no tag.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::Ident;
use syn::spanned::Spanned;

use super::enum_with_fields::discriminants;
use super::field::{PackedFields, PointerWithTags, build, pointer_among_tags, private_codec};
use super::layout::{AtomImpl, ImplRepr, LayoutCode};
use super::repr::located_at;
use crate::model::{EnumWithFields, Field, Implementor, Variant};

/// What a variant holds, as its code names it.
enum Contents<'a> {
    /// Nothing.
    Unit,
    /// Fields that hold no pointer, packed from bit 0.
    Data(PackedFields<'a>),
    /// A pointer, beside tag fields packed from bit 0.
    Pointer {
        /// The pointer field.
        pointer: &'a Field,
        /// Its index among the variant's fields.
        pointer_index: usize,
        /// The tag fields' code.
        tag_fields: PackedFields<'a>,
    },
}

/// A variant's code: its discriminant's name, `discriminant_<index>`, what it holds, and the name
/// of its fields' layout, `variant_<index>`, a local where it holds any.
struct VariantCode<'a> {
    /// The variant.
    variant: &'a Variant,
    /// Its discriminant's name.
    discriminant: Ident,
    /// What it holds.
    contents: Contents<'a>,
    /// Its fields' layout's name: its data's, or its tags'.
    variant_layout: Ident,
}

/// The code of a pointer enum: its variants' code, and its layout's locals, each variant's
/// placements and fields' layout, then the enum's layout, `layout`, a `PointerEnumLayout`, then
/// what it promises of zero, `promises`, a `PointerEnumValidity`.
struct EnumCode<'a> {
    /// The enum.
    implementor: &'a Implementor,
    /// Its variants.
    enum_with_fields: &'a EnumWithFields,
    /// Its variants' code.
    variants: Vec<VariantCode<'a>>,
    /// Its layout's locals.
    layout: LayoutCode<'a>,
    /// The name of what it promises of zero.
    promises: Ident,
    /// The derive's definition site, where the locals it names take their names.
    def_site: Span,
}

/// The locals the conversions name, each at the derive's definition site.
struct Locals {
    /// The repr decoded, or encoded.
    repr: Ident,
    /// A variant's fields' bits.
    bits: Ident,
    /// A pointer variant's pointer.
    pointer: Ident,
    /// The tags `to_tagged_repr` sets.
    tags: Ident,
    /// The bits under the tags an encode found set.
    misaligned: Ident,
}

/// `Atom` for the enum, in a block beside its discriminants and its layout: constants where it has
/// no parameters, else a function each instance evaluates, and another its code calls, which checks
/// that every variant fits.
pub(crate) fn pointer_enum(
    implementor: &Implementor, enum_with_fields: &EnumWithFields, def_site: Span,
) -> TokenStream {
    let code = EnumCode::new(implementor, enum_with_fields, def_site);
    code.layout.implement(code.fields(), code.atom_impl())
}

impl<'a> EnumCode<'a> {
    /// The code of `enum_with_fields`, of `implementor`, naming its locals at `def_site`.
    fn new(
        implementor: &'a Implementor, enum_with_fields: &'a EnumWithFields, def_site: Span,
    ) -> Self {
        let atomiks = &implementor.atomiks;
        let private = quote!(#atomiks::__private);
        let pointers = enum_with_fields.variants.iter().flat_map(|variant| &variant.fields);
        let pointers = pointers.filter(|field| field.is_pointer).map(|field| &field.ty);
        let mut layout = LayoutCode::new(implementor, def_site).stored_as_pointers(pointers);
        let variants: Vec<VariantCode<'a>> = (0_usize..)
            .zip(&enum_with_fields.variants)
            .map(|(index, variant)| {
                // Each field but the pointer packs as bits: a pointer variant's tags, a data
                // variant's data.
                let packed = variant.fields.iter().filter(|field| !field.is_pointer);
                let prefix = format!("placement_{index}");
                let fields = PackedFields::new(atomiks, packed, &prefix, def_site);
                let pointer = variant.fields.iter().enumerate().find(|(_, field)| field.is_pointer);
                let contents = match pointer {
                    Some((pointer_index, pointer)) => {
                        Contents::Pointer { pointer, pointer_index, tag_fields: fields }
                    },
                    None if variant.fields.is_empty() => Contents::Unit,
                    None => Contents::Data(fields),
                };
                let variant_layout = format_ident!("variant_{index}", span = def_site);
                if let Contents::Data(fields) | Contents::Pointer { tag_fields: fields, .. } =
                    &contents
                {
                    for (placement, placed) in fields.placed() {
                        layout.push(placement, quote!(#private::PackedField), placed);
                    }
                    layout.push(&variant_layout, quote!(#private::PackedLayout), fields.layout());
                }
                let discriminant = format_ident!("discriminant_{index}", span = def_site);
                VariantCode { variant, discriminant, contents, variant_layout }
            })
            .collect();
        let entries = variants.iter().map(|code| {
            let (discriminant, variant_layout) = (&code.discriminant, &code.variant_layout);
            match &code.contents {
                Contents::Unit => quote!(#private::PointerEnumVariant::unit(#discriminant)),
                Contents::Data(_) => quote!(#private::PointerEnumVariant::data(#discriminant)),
                Contents::Pointer { pointer: Field { ty, .. }, .. } => quote! {
                    #private::PointerEnumVariant::pointer::<_, #ty>(#discriminant, #variant_layout)
                },
            }
        });
        // Each discriminant is its variant's index where none is stated, so a niche may take one.
        let constructor = if enum_with_fields.has_stated_discriminant() {
            quote!(tagged)
        } else {
            quote!(niche_or_tagged)
        };
        let name = layout.name().clone();
        layout.push(
            &name,
            quote!(#private::PointerEnumLayout),
            quote!(#private::PointerEnumLayout::#constructor(&[#(#entries),*])),
        );
        let promises = Ident::new("promises", def_site);
        let folded = variants.iter().map(|code| {
            let discriminant = &code.discriminant;
            let variant_promises = match &code.contents {
                Contents::Unit => quote!(#private::PackedValidity::EMPTY),
                Contents::Data(fields) => fields.validity(),
                Contents::Pointer { pointer: Field { ty, .. }, tag_fields, .. } => {
                    let tag_promises = tag_fields.validity();
                    quote! {
                        #tag_promises.with_field::<<#ty as #atomiks::Atom>::Validity>(
                            #name.pointer_layout()
                        )
                    }
                },
            };
            quote!(.with_variant(#discriminant, #variant_promises))
        });
        layout.push(
            &promises,
            quote!(#private::PointerEnumValidity),
            quote!(#private::PointerEnumValidity::new(#name) #(#folded)*),
        );
        Self { implementor, enum_with_fields, variants, layout, promises, def_site }
    }

    /// Every field of every variant.
    fn fields(&self) -> impl Iterator<Item = &'a Field> + Clone {
        self.enum_with_fields.variants.iter().flat_map(|variant| &variant.fields)
    }

    /// Each check that a variant of the enum, or of an instance of it, fits: a pointer variant's
    /// tag and tag fields in the bits its pointer leaves clear, located at the pointer's type, and
    /// a data variant's fields above the bits the pointers' alignment leaves clear, located at the
    /// variant.
    fn checks(&self) -> Vec<TokenStream> {
        let Implementor { ident, generics, .. } = self.implementor;
        let (_, ty_generics, _) = generics.split_for_impl();
        let instance = quote!(#ident #ty_generics);
        let (layout, alignment) = (self.layout.name(), self.layout.alignment_name());
        self.variants
            .iter()
            .filter_map(|code| {
                let shown = format!("`{ident}::{}`", code.variant.ident);
                let variant_layout = &code.variant_layout;
                match &code.contents {
                    Contents::Unit => None,
                    Contents::Data(_) => Some(located_at(
                        quote! {
                            #layout.assert_data_fits::<#instance>(
                                #variant_layout, #alignment, #shown
                            )
                        },
                        code.variant.ident.span(),
                    )),
                    Contents::Pointer { pointer: Field { ty, .. }, .. } => Some(located_at(
                        quote!(#layout.assert_tags_fit::<#instance, #ty>(#shown)),
                        ty.span(),
                    )),
                }
            })
            .collect()
    }

    /// `Atom` for the enum, beside its discriminants, of `*mut ()`, its tags' width, and the
    /// validity and range its zero repr's variant promises.
    ///
    /// A generic enum's validity promises that zero decodes where its first variant is a unit and
    /// no discriminant is stated, so that the unit takes tag 0 or the niche's null, and nothing
    /// else.
    fn atom_impl(&self) -> AtomImpl {
        let Implementor { ident, atomiks, .. } = self.implementor;
        let integer = Ident::new("Discriminant", self.def_site);
        let items = discriminants(
            ident,
            self.enum_with_fields,
            self.variants.iter().map(|code| (code.variant, &code.discriminant)),
            &integer,
        );
        let (layout, promises) =
            (self.layout.local(self.layout.name()), self.layout.local(&self.promises));
        let validity = if !self.layout.is_generic() {
            self.layout.selected_validity(&quote!(#promises.code()))
        } else if matches!(
            self.variants.first(),
            Some(VariantCode { contents: Contents::Unit, .. })
        ) && !self.enum_with_fields.has_stated_discriminant()
        {
            quote!(#atomiks::validity::ZeroValid)
        } else {
            quote!(#atomiks::validity::Partial)
        };
        AtomImpl {
            items,
            repr: ImplRepr::Pointer { ty: quote!(*mut ()), tag_width: quote!(#layout.tag_width()) },
            validity,
            reprs: quote!(#promises.range()),
            checks: self.checks(),
            conversions: self.conversions(&integer),
        }
    }

    /// The arms of one variant: its `to_tagged_repr`'s, its `from_repr`'s and its
    /// `from_repr_unchecked`'s, each naming its locals as `locals` does.
    fn arms(&self, code: &VariantCode<'_>, locals: &Locals) -> [TokenStream; 3] {
        let (layout, alignment) = (self.layout.name(), self.layout.alignment_name());
        let Locals { repr, bits, pointer: pointer_value, tags, .. } = locals;
        let (some, none) =
            (quote!(::core::option::Option::Some), quote!(::core::option::Option::None));
        let (variant, discriminant, variant_layout) =
            (code.variant, &code.discriminant, &code.variant_layout);
        let ident = &variant.ident;
        let path = quote!(Self::#ident);
        match &code.contents {
            Contents::Unit => {
                let unit = if variant.is_written_as_unit { path } else { quote!(#path {}) };
                [
                    quote!(#unit => #tags.set_in(#layout.unit_repr(#discriminant)),),
                    quote! {
                        #discriminant if #layout.is_unit(#repr, #discriminant) => #some(#unit),
                    },
                    quote!(#discriminant => #unit,),
                ]
            },
            Contents::Data(fields) => {
                let pattern = build(&path, fields.fields().iter().copied(), fields.values());
                let packed = fields.encode(fields.values());
                let (value, value_unchecked) =
                    (fields.decode(bits, &path), fields.decode_unchecked(bits, &path));
                let is_data =
                    quote!(#layout.is_data(#repr, #discriminant, #variant_layout, #alignment));
                [
                    quote! {
                        #pattern => #tags.set_in(#layout.data_repr(#discriminant, #packed, #alignment)),
                    },
                    quote! {
                        #discriminant if #is_data => {
                            let #bits = #layout.data_bits(#repr, #alignment);
                            #value
                        },
                    },
                    quote! {
                        #discriminant => {
                            let #bits = #layout.data_bits(#repr, #alignment);
                            #value_unchecked
                        },
                    },
                ]
            },
            Contents::Pointer { pointer, pointer_index, tag_fields } => {
                let bindings = pointer_among_tags(
                    *pointer_index,
                    pointer_value.to_token_stream(),
                    tag_fields.values().iter().map(ToTokens::to_token_stream),
                );
                let pattern = build(&path, &variant.fields, &bindings);
                let tag_bits = tag_fields.encode(tag_fields.values());
                let pointer_tags = quote!(#layout.pointer_tags(#discriminant, #tag_bits, #tags));
                let [encode, decode, decode_unchecked, pointer_argument] =
                    self.pointer_codecs(pointer, &pointer_tags, locals);
                let pointer_with_tags = PointerWithTags {
                    fields: variant.fields.iter().collect(),
                    pointer_index: *pointer_index,
                    tag_fields,
                };
                let value = pointer_with_tags.decode(
                    &path,
                    &decode,
                    &pointer_argument,
                    bits,
                    self.def_site,
                );
                let value_unchecked = pointer_with_tags.decode_unchecked(
                    &path,
                    &decode_unchecked,
                    &pointer_argument,
                    bits,
                );
                // Unread where the variant has no tag fields.
                let bits_binding =
                    if tag_fields.fields().is_empty() { quote!(_) } else { bits.to_token_stream() };
                // Split in the arm, where the tag is known, unless the decode split before the
                // match.
                let splits_in_arm = !self.has_several_pointer_variants();
                let split = splits_in_arm
                    .then(|| quote!(let (#pointer_value, #bits) = #layout.split(#repr);));
                let split_unchecked = splits_in_arm
                    .then(|| quote!(let (#pointer_value, #bits_binding) = #layout.split(#repr);));
                [
                    quote!(#pattern => #encode,),
                    quote! {
                        #discriminant => {
                            #split
                            if !#layout.is_clear_above_fields(#bits, #variant_layout) {
                                return #none;
                            }
                            #value
                        },
                    },
                    quote! {
                        #discriminant => {
                            #split_unchecked
                            #value_unchecked
                        },
                    },
                ]
            },
        }
    }

    /// Whether several variants hold a pointer, so a decode splits the repr once, before its
    /// match, rather than in each pointer variant's arm.
    ///
    /// Split in its arm, a pointer's tag is a constant, which a load through it takes into its own
    /// offset, and which an update's encode adds back to give the repr it read, testing nothing.
    /// Several pointer variants split so would each subtract a tag of their own, and an arm that
    /// LLVM merges with another would choose between them; split once, they share one pointer,
    /// which LLVM still offsets by each arm's tag where an arm loads through it alone.
    fn has_several_pointer_variants(&self) -> bool {
        let mut pointers =
            self.variants.iter().filter(|code| matches!(code.contents, Contents::Pointer { .. }));
        pointers.next().is_some() && pointers.next().is_some()
    }

    /// How a pointer variant converts its field `pointer`, as `locals` names them: its encode, of
    /// the tags `pointer_tags` gives, its checked and unchecked decodes of a pointer, and the
    /// pointer it decodes, typed as the decodes take it.
    ///
    /// A concrete pointer's codecs cast it to and from `*mut ()` and bound it `const`; a generic
    /// one's are `Atom`'s, which keep the impl's `[const]` bound, and cast here.
    fn pointer_codecs(
        &self, pointer: &Field, pointer_tags: &TokenStream, locals: &Locals,
    ) -> [TokenStream; 4] {
        let atomiks = &self.implementor.atomiks;
        let Locals { pointer: pointer_value, misaligned, .. } = locals;
        let ty = &pointer.ty;
        if pointer.is_generic {
            let tagged = quote!(<#ty as #atomiks::Atom>::to_tagged_repr);
            return [
                quote! {{
                    let (#pointer_value, #misaligned) = #tagged(#pointer_value, #pointer_tags);
                    (#pointer_value.cast(), #misaligned)
                }},
                quote!(<#ty as #atomiks::Atom>::from_repr),
                quote!(<#ty as #atomiks::Atom>::from_repr_unchecked),
                quote!(#pointer_value.cast()),
            ];
        }
        let tagged = private_codec(atomiks, pointer, "to_tagged_pointer");
        [
            quote!(#tagged(#pointer_value, #pointer_tags)),
            private_codec(atomiks, pointer, "from_pointer"),
            private_codec(atomiks, pointer, "from_pointer_unchecked"),
            pointer_value.to_token_stream(),
        ]
    }

    /// `to_repr`, `to_tagged_repr`, `from_repr` and `from_repr_unchecked`, each started by binding
    /// the layout's locals where they are an instance's, each a match of the value's variant or of
    /// the discriminant, of `integer`, that the layout reads from the repr.
    fn conversions(&self, integer: &Ident) -> TokenStream {
        let Implementor { atomiks, .. } = self.implementor;
        let private = quote!(#atomiks::__private);
        let layout = self.layout.name();
        let local = |name| Ident::new(name, self.def_site);
        let locals = Locals {
            repr: local("repr"),
            bits: local("bits"),
            pointer: local("pointer"),
            tags: local("tags"),
            misaligned: local("misaligned"),
        };
        let Locals { repr, tags, misaligned, .. } = &locals;
        let none = quote!(::core::option::Option::None);
        let (mut encoded, mut decoded, mut decoded_unchecked) =
            (Vec::new(), Vec::new(), Vec::new());
        for code in &self.variants {
            let [to_tagged, checked, unchecked] = self.arms(code, &locals);
            encoded.push(to_tagged);
            decoded.push(checked);
            decoded_unchecked.push(unchecked);
        }
        let to_tagged_self = if self.layout.is_generic() {
            quote!(<Self as #atomiks::Atom>::to_tagged_repr)
        } else {
            quote!(#private::to_tagged_repr::<Self>)
        };
        // Each conversion binds only the locals it reads: no fields' layout but where `from_repr`
        // checks a variant's bits, never the promises, and the alignment only beside data.
        let has_data = self.variants.iter().any(|code| matches!(code.contents, Contents::Data(_)));
        let alignment = (!has_data).then_some(self.layout.alignment_name());
        let bind_all_but = |unread: &[&Ident]| {
            let unread: Vec<&Ident> = unread.iter().copied().chain(alignment).collect();
            self.layout.bind(&unread)
        };
        let variant_layouts = self
            .variants
            .iter()
            .filter(|code| !matches!(code.contents, Contents::Unit))
            .map(|code| &code.variant_layout);
        let unread: Vec<&Ident> = variant_layouts.chain([&self.promises]).collect();
        let (bind, bind_checked) = (bind_all_but(&unread), bind_all_but(&[&self.promises]));
        let (split, split_unchecked) = if self.has_several_pointer_variants() {
            let Locals { pointer, bits, .. } = &locals;
            // Unread where no variant has tag fields.
            let has_tag_fields = self.variants.iter().any(|code| {
                matches!(&code.contents, Contents::Pointer { tag_fields, .. } if !tag_fields.fields().is_empty())
            });
            let bits_binding = if has_tag_fields { bits.to_token_stream() } else { quote!(_) };
            (
                Some(quote!(let (#pointer, #bits) = #layout.split(#repr);)),
                Some(quote!(let (#pointer, #bits_binding) = #layout.split(#repr);)),
            )
        } else {
            (None, None)
        };
        // The impl keeps each promise of `Atom`: each variant's repr is its tag, the discriminant,
        // in the bits the layout gives it, beside a pointer variant's pointer, to which
        // `to_tagged_repr` passes the tag and the tag fields' bits above it, beside those of any
        // word that holds the enum, and which sets them as `Tags::set_in` does, as its own impl
        // promises, so it keeps its provenance; or beside a data variant's fields' bits above the
        // clear bits, an address without provenance; or alone, a unit's; a word's tags set in
        // either of those as `set_in` sets them; `to_repr` is the repr with no tags beside, which
        // refuses a pointer that had a tag bit set; `from_repr` reads the tag back and decodes
        // the variant it names, refusing a unit's or a data variant's repr with any other bit set,
        // a pointer variant's with a tag field's bit set past its own, and a field that does not
        // decode, so each value has one repr, by the repr alone; `from_repr_unchecked` matches the
        // same tag and decodes each field unchecked, which gives what `from_repr` does; the
        // validity and range are what the variant zero holds promises of zero; and the value may
        // cross threads, as the impl checks or bounds each field but the pointers, whose own impls
        // promise that they may.
        quote! {
            #[inline]
            fn to_repr(self) -> *mut () {
                let (#repr, #misaligned) = #to_tagged_self(self, #private::Tags::EMPTY);
                #private::assert_aligned::<Self>(#misaligned);
                #repr
            }
            #[inline]
            fn to_tagged_repr(self, #tags: #private::Tags) -> (*mut (), ::core::primitive::usize) {
                #bind
                match self {
                    #(#encoded)*
                }
            }
            #[inline]
            fn from_repr(#repr: *mut ()) -> ::core::option::Option<Self> {
                #bind_checked
                #split
                match #layout.discriminant::<#integer>(#repr) {
                    #(#decoded)*
                    _ => #none,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(#repr: *mut ()) -> Self {
                // The caller's repr decodes, so its tag is one of the variants below, whose fields
                // decode.
                #bind
                #split_unchecked
                match #layout.discriminant::<#integer>(#repr) {
                    #(#decoded_unchecked)*
                    _ => unsafe { ::core::hint::unreachable_unchecked() },
                }
            }
        }
    }
}
