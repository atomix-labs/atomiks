//! `Atom` for an enum with fields: each variant's fields packed from bit 0, as a struct's, beside a
//! selector that says which variant a repr holds: a tag above the widest variant's fields, each
//! variant's discriminant; or, where one variant alone has fields and none states a discriminant,
//! that variant's top field, grown into the reprs beside its range that the unit variants take,
//! wherever that is no wider than a tag.

use proc_macro2::{Group, Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::Ident;

use super::field::{PackedFields, build};
use super::layout::{AtomImpl, ImplRepr, LayoutCode};
use super::respan;
use crate::model::{EnumWithFields, Implementor, Variant};

/// The code of a variant: its discriminant's name, and for one with fields, their code and its
/// layout's name.
struct VariantCode<'a> {
    /// The variant.
    variant: &'a Variant,
    /// Its discriminant's name, `discriminant_<index>`.
    discriminant: Ident,
    /// Its fields' code and its layout's name, `variant_<index>`; `None` for a unit variant.
    fields: Option<(PackedFields<'a>, Ident)>,
}

/// `Atom` for the enum, in a block beside its discriminants, each a constant named by its variant's
/// index, and its layout: constants where it has no parameters, else a function each instance
/// evaluates, the repr it states checked there.
///
/// The block names its items at `def_site`, and none takes a name of the user's, so no variant's
/// name clashes with one of them. A concrete enum's validity is what its variants promise of their
/// bits; a generic one's promises that zero decodes only where its discriminants put a unit variant
/// there.
pub(crate) fn enum_with_fields(
    implementor: &Implementor, enum_with_fields: &EnumWithFields, def_site: Span,
) -> TokenStream {
    let atomix = &implementor.atomix;
    let private = quote!(#atomix::__private);
    let mut layout = LayoutCode::new(implementor, def_site);
    let variants: Vec<VariantCode<'_>> = (0..)
        .zip(&enum_with_fields.variants)
        .map(|(index, variant)| {
            let fields = (!variant.fields.is_empty()).then(|| {
                let prefix = format!("placement_{index}");
                let fields = PackedFields::new(atomix, &variant.fields, &prefix, def_site);
                for (placement, placed) in fields.placed() {
                    layout.push(placement, quote!(#private::PackedField), placed);
                }
                let name = format_ident!("variant_{index}", span = def_site);
                layout.push(&name, quote!(#private::PackedLayout), fields.layout());
                (fields, name)
            });
            let discriminant = format_ident!("discriminant_{index}", span = def_site);
            VariantCode { variant, discriminant, fields }
        })
        .collect();
    let integer = Ident::new("Discriminant", def_site);
    let items = discriminants(
        &implementor.ident,
        enum_with_fields,
        variants.iter().map(|code| (code.variant, &code.discriminant)),
        &integer,
    );
    let names: Vec<&Ident> = variants.iter().map(|code| &code.discriminant).collect();
    let with_fields: Vec<(&Ident, &Ident)> = variants
        .iter()
        .filter_map(|code| code.fields.as_ref().map(|(_, name)| (&code.discriminant, name)))
        .collect();
    let has_stated_discriminant = enum_with_fields.has_stated_discriminant();
    let enum_layout = match with_fields.as_slice() {
        // Each variant's discriminant is its index: the units are numbered beside the payload.
        [(discriminant, fields)] if !has_stated_discriminant => {
            let unit_count = Literal::usize_unsuffixed(names.len().saturating_sub(1));
            quote!(#private::EnumLayout::niche_or_tagged(#unit_count, #fields, #discriminant))
        },
        _ => {
            let fields = with_fields.iter().map(|(_, fields)| fields);
            quote! {
                #private::EnumLayout::tagged(
                    #private::discriminant_range([#(#names),*]), &[#(#fields),*]
                )
            }
        },
    };
    let name = layout.name().clone();
    layout.push(&name, quote!(#private::EnumLayout), enum_layout);
    let validity = validity(&layout, &variants, has_stated_discriminant);
    let conversions = conversions(&layout, &variants, &integer, def_site);
    // The impl keeps each promise of `Atom`: each variant's value is its fields, packed from bit 0
    // as a struct's, beside the selector of its discriminant, which the layout reads back, with
    // every other bit below the selector clear, and above the enum's width, the bits extend the
    // selector, so the value lies in the range; a unit's selector lies outside the range of the
    // payload's top field where it fills a niche, and the units' are distinct from each other's
    // and from every tag else, so `from_repr` decodes a repr as its value alone, refusing other
    // bits above the width, an unclear bit, a selector of no variant, and a field that does not
    // decode, so each value has one repr; `from_repr_unchecked` matches the same discriminant and
    // decodes each field unchecked, which gives what the field's `from_repr` does, so it gives
    // what `from_repr` does; the validity is what the variants promise of the zero repr and of
    // every pattern, or, for a generic enum, of zero where a unit takes it; `lay_out`, which every
    // conversion and the range of a generic enum evaluate, refuses an instance wider than its
    // repr; and the value may cross threads, as the impl checks or bounds the type, unless a field
    // is written as a pointer, which `to_bits` refuses.
    let fields = enum_with_fields.variants.iter().flat_map(|variant| &variant.fields);
    let reprs = {
        let layout = layout.local(&name);
        quote!(#layout.range())
    };
    let atom = AtomImpl {
        items,
        repr: ImplRepr::Integer,
        validity,
        reprs,
        checks: Vec::new(),
        conversions,
    };
    layout.implement(fields, atom)
}

/// The discriminants of `enumeration`, each a constant of the name `variants` gives its variant,
/// of the integer `integer` names, the enum's `#[repr]`'s or `isize`: each the expression its
/// variant states, else one past the one before, else its index.
///
/// The user's expressions keep their spans, so each name resolves as it does in the enum, and
/// none to the derive's locals, but `Self`, which names the enum there; reading them so lints
/// nothing the user did not write, where a mirror enum of them would lint them again as an enum.
pub(super) fn discriminants<'a, I: IntoIterator<Item = (&'a Variant, &'a Ident)>>(
    enumeration: &Ident, enum_with_fields: &EnumWithFields, variants: I, integer: &Ident,
) -> TokenStream {
    let integer_type = enum_with_fields.integer.as_ref().map_or_else(
        || quote!(::core::primitive::isize),
        |discriminant| quote!(::core::primitive::#discriminant),
    );
    let mut after_stated: Option<&Ident> = None;
    let constants = (0..).zip(variants).map(|(index, (variant, name))| {
        let value = match (&variant.discriminant, after_stated) {
            (Some(expression), _) => with_self_as(expression.to_token_stream(), enumeration),
            // Wrapping, so that an enum whose discriminant overflows raises rustc's error alone.
            (None, Some(before)) => quote!(#before.wrapping_add(1)),
            (None, None) => Literal::usize_unsuffixed(index).into_token_stream(),
        };
        if variant.discriminant.is_some() || after_stated.is_some() {
            after_stated = Some(name);
        }
        quote!(const #name: #integer = #value;)
    });
    quote! {
        type #integer = #integer_type;
        #(#constants)*
    }
}

/// `tokens`, each `Self` among them `enumeration`, spanned as it was: the type it names in the
/// enum's discriminants, outside of which they are read.
fn with_self_as(tokens: TokenStream, enumeration: &Ident) -> TokenStream {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(group) => {
                let mut named =
                    Group::new(group.delimiter(), with_self_as(group.stream(), enumeration));
                named.set_span(group.span());
                TokenTree::Group(named)
            },
            TokenTree::Ident(word) if word == "Self" => {
                TokenTree::Ident(respan(enumeration, word.span()))
            },
            other @ (TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_)) => other,
        })
        .collect()
}

/// The enum's validity: a concrete enum's what its layout and its variants' fields promise; a
/// generic one's `ZeroValid` where its discriminants are stated and a unit's is zero, else
/// `Partial`.
fn validity(
    layout: &LayoutCode<'_>, variants: &[VariantCode<'_>], has_stated_discriminant: bool,
) -> TokenStream {
    let atomix = layout.atomix();
    let private = quote!(#atomix::__private);
    if layout.instance_repr().is_some() {
        let units: Vec<&Ident> = variants
            .iter()
            .filter(|code| code.fields.is_none())
            .map(|code| &code.discriminant)
            .collect();
        if !has_stated_discriminant || units.is_empty() {
            return quote!(#atomix::validity::Partial);
        }
        return layout.selected_validity(&quote! {
            if #(#units == 0)||* { #private::ZERO_VALID } else { #private::PARTIAL }
        });
    }
    let folded = variants.iter().map(|code| {
        let discriminant = &code.discriminant;
        if let Some((fields, name)) = &code.fields {
            let promises = fields.validity();
            quote!(.with_variant(#discriminant, #name, #promises))
        } else {
            quote!(.with_unit(#discriminant))
        }
    });
    let (name, repr) = (layout.name(), layout.repr_alias());
    layout.selected_validity(&quote! {
        #private::EnumValidity::new(#name)
            #(#folded)*
            .code(<#repr as #atomix::Primitive>::BITS)
    })
}

/// `to_repr`, `from_repr` and `from_repr_unchecked` of the enum, through its `layout`, whose
/// discriminants are of the integer `integer` names: each matches the value's variant, or the
/// discriminant the layout reads from the repr.
fn conversions(
    layout: &LayoutCode<'_>, variants: &[VariantCode<'_>], integer: &Ident, def_site: Span,
) -> TokenStream {
    let atomix = layout.atomix();
    let (name, alias) = (layout.name(), layout.repr_alias());
    let repr = Ident::new("repr", def_site);
    let bits = Ident::new("bits", def_site);
    let (some, none) = (quote!(::core::option::Option::Some), quote!(::core::option::Option::None));
    let (mut encoded, mut decoded, mut decoded_unchecked) = (Vec::new(), Vec::new(), Vec::new());
    for VariantCode { variant, discriminant, fields } in variants {
        let ident = &variant.ident;
        let path = quote!(Self::#ident);
        if let Some((fields, variant_layout)) = fields {
            let pattern = build(&path, fields.fields().iter().copied(), fields.values());
            let packed = fields.encode(fields.values());
            encoded.push(quote!(#pattern => #name.repr(#discriminant, #packed),));
            let value = fields.decode(&bits, &path);
            decoded.push(quote!(#discriminant if #name.is_clear_above_fields(#bits, #variant_layout) => #value,));
            let value = fields.decode_unchecked(&bits, &path);
            decoded_unchecked.push(quote!(#discriminant => #value,));
        } else {
            let unit = if variant.is_written_as_unit { path } else { quote!(#path {}) };
            encoded.push(quote!(#unit => #name.repr(#discriminant, 0),));
            decoded.push(
                quote!(#discriminant if #name.is_clear_below_selector(#bits) => #some(#unit),),
            );
            decoded_unchecked.push(quote!(#discriminant => #unit,));
        }
    }
    let variant_layouts: Vec<&Ident> =
        variants.iter().filter_map(|code| code.fields.as_ref().map(|(_, name)| name)).collect();
    let (bind, bind_fields) = (layout.bind(&[]), layout.bind(&variant_layouts));
    let discriminant = quote!(#name.discriminant::<#integer>(#bits));
    quote! {
        #[inline]
        fn to_repr(self) -> #alias {
            #bind_fields
            match self {
                #(#encoded)*
            }
        }
        #[inline]
        fn from_repr(#repr: #alias) -> ::core::option::Option<Self> {
            #bind
            match #name.canonical_bits(#repr) {
                #some(#bits) => match #discriminant {
                    #(#decoded)*
                    _ => #none,
                },
                #none => #none,
            }
        }
        #[inline]
        unsafe fn from_repr_unchecked(#repr: #alias) -> Self {
            // The caller's repr decodes, so it holds one of the variants below, whose fields' bits
            // decode.
            #bind_fields
            let #bits = #atomix::__private::to_bits(#repr);
            match #discriminant {
                #(#decoded_unchecked)*
                _ => unsafe { ::core::hint::unreachable_unchecked() },
            }
        }
    }
}
