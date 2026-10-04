//! A packed struct's `Atom`: each field in its own bits, from bit 0 in declaration order, and the
//! value's bits above them extending its top field.

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{Ident, Path};

use super::bound::{own_where_clause, thread_bounds, thread_checks, where_clause};
use super::field::PackedFields;
use super::repr;
use crate::model::{Field, Implementor};

/// `Atom` for the packed struct of `fields`, in a block beside its layout: constants where it has
/// no parameters, else a function each instance evaluates, the repr it states checked there.
///
/// The block names its items at `def_site`, so that the user's code reaches none of them.
pub(crate) fn packed(implementor: &Implementor, fields: &[Field], def_site: Span) -> TokenStream {
    let packed = PackedFields::new(&implementor.atomiks, fields, def_site);
    match &implementor.repr {
        // Parse refuses a struct with parameters that states no repr: no constant names an
        // instance's layout, so no repr could be selected from it.
        Some(stated) if !implementor.generics.params.is_empty() => {
            generic(implementor, &packed, stated, def_site)
        },
        _ => concrete(implementor, &packed, def_site),
    }
}

/// `Atom` for the struct without parameters, its layout constants: its repr the narrowest that
/// holds it, or the one stated, and its validity selected from its fields'.
fn concrete(implementor: &Implementor, packed: &PackedFields<'_>, def_site: Span) -> TokenStream {
    let Implementor { ident, atomiks, repr: stated, .. } = implementor;
    let private = quote!(#atomiks::__private);
    let alias = Ident::new("Repr", def_site);
    let layout = Ident::new("layout", def_site);
    let width = quote!(#layout.width());
    let (repr_type, repr_checks) = stated.as_ref().map_or_else(
        || (repr::narrowest(atomiks, &width), repr::width_check(atomiks, ident, &width)),
        |stated| {
            let integer_check = repr::integer_check(implementor, stated);
            let integer = quote!(::core::primitive::#stated);
            let width_check = repr::stated_width_check(atomiks, ident, &integer, stated, &width);
            (repr::selected_from(atomiks, stated), quote!(#integer_check #width_check))
        },
    );
    let placements = packed
        .placed()
        .map(|(placement, placed)| quote!(const #placement: #private::PackedField = #placed;));
    let packed_layout = packed.layout();
    let fields = packed.iter().map(|(Field { ty, .. }, placement)| {
        quote!(.with_field::<<#ty as #atomiks::Atom>::Validity>(#placement.layout()))
    });
    let validity = quote! {
        #private::PackedValidity::EMPTY #(#fields)*
            .code(#width, <#alias as #atomiks::Primitive>::BITS)
    };
    let thread_checks = thread_checks(implementor, packed.fields());
    let where_clause = where_clause(implementor, []);
    let conversions = conversions(atomiks, packed, &alias, &layout, None, def_site);
    // The impl keeps each promise of `Atom`: each field packs into bits of its own, and the value
    // is the repr of those bits, extended above its width as its top field is, so it lies in the
    // range, and `from_repr` reads each field's repr back from them, so it decodes as the value;
    // `from_repr` refuses any other bits above, and a field that does not decode, so each value
    // has one repr; `from_repr_unchecked` reads the same bits back and decodes each field
    // unchecked, which gives what the field's `from_repr` does, so it gives what `from_repr` does;
    // the validity is what the fields promise of the bits they take, and of the bits above them;
    // and the value may cross threads, as `thread_checks` checks the type, unless a field is
    // written as a pointer, which `to_bits` refuses.
    quote! {
        #thread_checks
        const _: () = {
            #(#placements)*
            const #layout: #private::PackedLayout = #packed_layout;
            type #alias = #repr_type;
            #repr_checks
            #[automatically_derived]
            const unsafe impl #atomiks::Atom for #ident #where_clause {
                type Repr = #alias;
                type Validity = <
                    #private::ValidityCode<{ #validity }> as #private::SelectValidity
                >::Validity;
                const REPRS: #atomiks::ReprRange<#alias> = #layout.range();
                #conversions
            }
        };
    }
}

/// `Atom` for the struct with parameters, in the repr `stated`: its layout a function of the
/// instance that refuses one wider than the repr, and its validity whether each field's zero
/// decodes.
fn generic(
    implementor: &Implementor, packed: &PackedFields<'_>, stated: &Ident, def_site: Span,
) -> TokenStream {
    let Implementor { ident, generics, atomiks, .. } = implementor;
    let private = quote!(#atomiks::__private);
    let alias = Ident::new("Repr", def_site);
    let layout = Ident::new("layout", def_site);
    let lay_out_fn = Ident::new("lay_out", def_site);
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let instance = quote!(#ident #ty_generics);
    let repr_type = repr::selected_from(atomiks, stated);
    let integer_check = repr::integer_check(implementor, stated);
    let width_assertion = repr::stated_width_assertion(
        atomiks,
        &instance,
        &quote!(::core::primitive::#stated),
        stated,
        &quote!(#layout.width()),
    );
    let placements = packed.placed().map(|(placement, placed)| quote!(let #placement = #placed;));
    let names = packed.placements();
    let count = Literal::usize_unsuffixed(names.len());
    let packed_layout = packed.layout();
    let generic_fields = packed.fields().iter().filter(|field| field.is_generic);
    let layout_where = own_where_clause(
        implementor,
        generic_fields.clone().map(|Field { ty, .. }| quote!(#ty: #atomiks::Atom)),
    );
    let field_bounds = generic_fields.map(|Field { ty, .. }| {
        quote!(#ty: [const] #atomiks::Atom<Repr: [const] #atomiks::__private::FieldRepr>)
    });
    let where_clause =
        where_clause(implementor, field_bounds.chain(thread_bounds(implementor, packed.fields())));
    let validity = packed.fields().iter().rev().fold(
        quote!(#atomiks::validity::ZeroValid),
        |beside, Field { ty, .. }| {
            quote! {
                <<#ty as #atomiks::Atom>::Validity as #atomiks::validity::Validity>
                    ::ZeroValidityWith<#beside>
            }
        },
    );
    let marker = quote!(::core::marker::PhantomData);
    let layout_call = quote!(#lay_out_fn(#marker::<Self>));
    let thread_checks = thread_checks(implementor, packed.fields());
    let conversions = conversions(atomiks, packed, &alias, &layout, Some(&layout_call), def_site);
    // The impl keeps each promise of `Atom` as the concrete one's does, its validity aside: no
    // constant knows an instance's layout, so it promises only that zero decodes, and only where
    // each field's does; and `lay_out`, which every conversion and the range evaluate, refuses an
    // instance wider than its repr, so no field's bits fall past the repr's.
    quote! {
        #thread_checks
        const _: () = {
            type #alias = #repr_type;
            #integer_check
            const fn #lay_out_fn #impl_generics(_: #marker<#instance>) -> (
                [#private::PackedField; #count], #private::PackedLayout
            ) #layout_where {
                #(#placements)*
                let #layout = #packed_layout;
                #width_assertion;
                ([#(#names),*], #layout)
            }
            #[automatically_derived]
            const unsafe impl #impl_generics #atomiks::Atom for #instance #where_clause {
                type Repr = #alias;
                type Validity = #validity;
                const REPRS: #atomiks::ReprRange<#alias> = #layout_call.1.range();
                #conversions
            }
        };
    }
}

/// `to_repr`, `from_repr` and `from_repr_unchecked` of the struct, whose repr is `alias`, of its
/// placements and `layout`: constants where `layout_call` is `None`, else bound at the start of
/// each from `layout_call`, evaluated once for each instance.
fn conversions(
    atomiks: &Path, packed: &PackedFields<'_>, alias: &Ident, layout: &Ident,
    layout_call: Option<&TokenStream>, def_site: Span,
) -> TokenStream {
    let names = packed.placements();
    let bind = |layout: TokenStream| {
        layout_call.map(|call| quote!(let ([#(#names),*], #layout) = const { #call };))
    };
    let (bind_layout, bind_placements) = (bind(quote!(#layout)), bind(quote!(_)));
    let repr = Ident::new("repr", def_site);
    let bits = Ident::new("bits", def_site);
    let encoded =
        packed.encode(packed.fields().iter().map(|Field { member, .. }| quote!(self.#member)));
    let decoded = packed.decode(&bits, &quote!(Self));
    let decoded_unchecked = packed.decode_unchecked(&bits, &quote!(Self));
    quote! {
        #[inline]
        fn to_repr(self) -> #alias {
            #bind_layout
            #layout.repr(#encoded)
        }
        #[inline]
        fn from_repr(#repr: #alias) -> ::core::option::Option<Self> {
            #bind_layout
            match #layout.canonical_bits(#repr) {
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
