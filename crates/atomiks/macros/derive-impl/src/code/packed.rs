//! A packed struct's `Atom`: each field in its own bits, from bit 0 in declaration order, and the
//! value's bits above them extending its top field; and its projection onto its fields.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use super::field::PackedFields;
use super::layout::{AtomImpl, ImplRepr, LayoutCode};
use super::projection::{FieldSite, ProjectionCode};
use crate::model::{Field, Implementor};

/// `Atom` for the packed struct of `fields`, in a block beside its layout: constants where it has
/// no parameters, else a function each instance evaluates, the repr it states checked there; and
/// its projection, the struct `fields()` lends, beside it.
///
/// A concrete struct's validity is what its fields promise of their bits; a generic one's, which
/// no constant knows the layout of, promises only that zero decodes, and only where each field's
/// does.
pub(crate) fn packed(implementor: &Implementor, fields: &[Field], def_site: Span) -> TokenStream {
    let atomiks = &implementor.atomiks;
    let packed = PackedFields::new(atomiks, fields, "placement", def_site);
    let mut layout = LayoutCode::new(implementor, def_site);
    for (placement, placed) in packed.placed() {
        layout.push(placement, quote!(#atomiks::__private::PackedField), placed);
    }
    let name = layout.name().clone();
    layout.push(&name, quote!(#atomiks::__private::PackedLayout), packed.layout());
    let validity = if layout.instance_repr().is_some() {
        fields.iter().rev().fold(
            quote!(#atomiks::validity::ZeroValid),
            |beside, Field { ty, .. }| {
                quote! {
                    <<#ty as #atomiks::Atom>::Validity as #atomiks::validity::Validity>
                        ::ZeroValidityWith<#beside>
                }
            },
        )
    } else {
        let (fields, repr) = (packed.validity(), layout.repr_alias());
        layout.selected_validity(&quote! {
            #fields.code(#name.width(), <#repr as #atomiks::Primitive>::BITS)
        })
    };
    let conversions = conversions(&packed, &layout, def_site);
    let projection = ProjectionCode::new(implementor, fields, def_site);
    // The impl keeps each promise of `Atom`: each field packs into bits of its own, and the value
    // is the repr of those bits, extended above its width as its top field is, so it lies in the
    // range, and `from_repr` reads each field's repr back from them, so it decodes as the value;
    // `from_repr` refuses any other bits above, and a field that does not decode, so each value
    // has one repr; `from_repr_unchecked` reads the same bits back and decodes each field
    // unchecked, which gives what the field's `from_repr` does, so it gives what `from_repr` does;
    // the validity is what the fields promise of the bits they take, and of the bits above them,
    // or, for a generic struct, of zero alone; `lay_out`, which every conversion and the range of
    // a generic struct evaluate, refuses an instance wider than its repr, so no field's bits fall
    // past the repr's; and the value may cross threads, as the impl checks or bounds the type,
    // unless a field is written as a pointer, which `to_bits` refuses.
    //
    // Each `HasPackedField` keeps its promise: its type parameter is the field's type, and its
    // placement is `PackedField::new` of that type's reprs, at the offset where the field before
    // it ends, where `to_repr` packs the field, each field in bits of its own; the layout is the
    // struct's, whose extension `to_repr` writes above the width; `from_repr` decodes each field
    // alone and checks only the bits above the width, so a repr decodes wherever each field's bits
    // decode and the bits above extend the layout; `Reach` is `Reach<true>` only where
    // `reaches_top` finds the field ending at the repr's top bit, and never for a generic struct;
    // and `field` returns the field.
    let atom = AtomImpl {
        items: projection.implementations(field_sites(&packed, &layout)),
        repr: ImplRepr::Integer,
        validity,
        reprs: {
            let layout = layout.local(&name);
            quote!(#layout.range())
        },
        checks: Vec::new(),
        conversions,
    };
    let implemented = layout.implement(fields, atom);
    let structure = projection.structure();
    quote!(#implemented #structure)
}

/// Where each of `packed`'s fields lies, as its `HasPackedField` says.
///
/// A concrete struct's placements and layout are constants, and a field that ends at the repr's
/// top bit reaches it; a generic struct's are what each instance lays out, which no constant knows,
/// so no field reaches the top.
fn field_sites<'a>(
    packed: &'a PackedFields<'_>, layout: &'a LayoutCode<'_>,
) -> impl Iterator<Item = FieldSite> + 'a {
    let atomiks = packed.atomiks();
    let (name, alias) = (layout.name(), layout.repr_alias());
    packed.placements().iter().map(move |placement| {
        let reach = if layout.is_generic() {
            quote!(#atomiks::__private::Reach<false>)
        } else {
            let reaches_top = quote!(#atomiks::__private::reaches_top::<#alias>(#placement));
            quote!(#atomiks::__private::Reach<{ #reaches_top }>)
        };
        FieldSite { placement: layout.local(placement), layout: layout.local(name), reach }
    })
}

/// `to_repr`, `from_repr` and `from_repr_unchecked` of the struct, through its `layout`.
fn conversions(packed: &PackedFields<'_>, layout: &LayoutCode<'_>, def_site: Span) -> TokenStream {
    let atomiks = packed.atomiks();
    let (name, alias) = (layout.name(), layout.repr_alias());
    let repr = Ident::new("repr", def_site);
    let bits = Ident::new("bits", def_site);
    let encoded =
        packed.encode(packed.fields().iter().map(|Field { member, .. }| quote!(self.#member)));
    let decoded = packed.decode(&bits, &quote!(Self));
    let decoded_unchecked = packed.decode_unchecked(&bits, &quote!(Self));
    let (bind, bind_placements) = (layout.bind(&[]), layout.bind(&[name]));
    quote! {
        #[inline]
        fn to_repr(self) -> #alias {
            #bind
            #name.repr(#encoded)
        }
        #[inline]
        fn from_repr(#repr: #alias) -> ::core::option::Option<Self> {
            #bind
            match #name.canonical_bits(#repr) {
                ::core::option::Option::Some(#bits) => #decoded,
                ::core::option::Option::None => ::core::option::Option::None,
            }
        }
        #[inline]
        unsafe fn from_repr_unchecked(#repr: #alias) -> Self {
            // The caller's repr decodes, so its bits above the width are canonical, and each
            // field's decode.
            #bind_placements
            let #bits = #atomiks::__private::to_bits(#repr);
            #decoded_unchecked
        }
    }
}
