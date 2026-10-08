//! A fieldless enum's `Atom`: each variant stored as its discriminant, which casting the variant
//! reads exactly as rustc evaluates it, whatever constant expression it is written as.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use super::bound::{thread_checks, where_clause};
use super::{repr, respan};
use crate::model::{EnumRepr, Fieldless, Implementor};

/// `Atom` for the fieldless enum, in a block beside its repr and each variant's discriminant in it,
/// a constant named by the variant's index, `discriminant_<index>`.
///
/// The block names its items at `def_site`, so that the user's code reaches none of them, and none
/// takes a name of the user's, so no variant's name clashes with one of them.
pub(crate) fn fieldless(
    implementor: &Implementor, fieldless: &Fieldless, def_site: Span,
) -> TokenStream {
    let Implementor { ident, atomix, .. } = implementor;
    let private = quote!(#atomix::__private);
    let alias = Ident::new("Repr", def_site);
    let parameter = Ident::new("repr", def_site);
    let variants = &fieldless.variants;
    let constants: Vec<Ident> = (0..variants.len())
        .map(|index| format_ident!("discriminant_{index}", span = def_site))
        .collect();
    // Each variant's path, which the derive's casts read: spanned at the call site, so that the
    // user's lints, `clippy::as_conversions` among them, read each cast as a macro's.
    let enumeration = respan(ident, Span::call_site());
    let paths: Vec<TokenStream> = variants
        .iter()
        .map(|variant| {
            let variant = respan(variant, Span::call_site());
            quote!(#enumeration::#variant)
        })
        .collect();
    let width = quote! {
        #private::discriminant_width([#(#paths as ::core::primitive::i128),*])
    };
    let (repr_type, checks) = match &fieldless.repr {
        EnumRepr::Integer(integer) => {
            (repr::selected_from(atomix, integer), repr::integer_check(implementor, integer))
        },
        // Rustc widens an enum past C's `int` with a warning alone, so the width is checked.
        EnumRepr::C(c) => {
            let c_int = quote!(::core::ffi::c_int);
            let width_check = repr::stated_width_check(atomix, ident, &c_int, c, &width);
            (quote!(<#c_int as #private::SelectRepr>::Repr), width_check)
        },
        EnumRepr::Stated(stated) => {
            let integer_check = repr::integer_check(implementor, stated);
            let integer = quote!(::core::primitive::#stated);
            let width_check = repr::stated_width_check(atomix, ident, &integer, stated, &width);
            (repr::selected_from(atomix, stated), quote!(#integer_check #width_check))
        },
        EnumRepr::Selected => (repr::narrowest(atomix, &width), TokenStream::new()),
    };
    let where_clause = where_clause(implementor, []);
    // The impl keeps each promise of `Atom`: a variant's repr is its discriminant, which
    // `from_repr` decodes as that variant alone, and the range encloses; the repr holds each
    // discriminant whole, as its integer `#[repr]` types it or as the width checked or selected
    // says, so no two share a repr; `from_repr_unchecked` matches the same discriminants, and the
    // caller's repr decodes, so it is one of them, and gives what `from_repr` does; the validity
    // says which reprs the discriminants take; and the enum may cross threads, as `thread_checks`
    // checks.
    let thread_checks = thread_checks(implementor, []);
    quote! {
        #thread_checks
        const _: () = {
            type #alias = #repr_type;
            #checks
            #(const #constants: #alias = #paths as #alias;)*
            #[automatically_derived]
            const unsafe impl #atomix::Atom for #ident #where_clause {
                type Repr = #alias;
                type Validity = <
                    #private::ValidityCode<{ #private::discriminant_validity_code([#(#constants),*]) }>
                    as #private::SelectValidity
                >::Validity;
                const REPRS: #atomix::ReprRange<#alias> =
                    #private::discriminant_range([#(#constants),*]);
                #[inline]
                fn to_repr(self) -> #alias {
                    match self {
                        #(Self::#variants => #constants,)*
                    }
                }
                #[inline]
                fn from_repr(#parameter: #alias) -> ::core::option::Option<Self> {
                    match #parameter {
                        #(#constants => ::core::option::Option::Some(Self::#variants),)*
                        _ => ::core::option::Option::None,
                    }
                }
                #[inline]
                unsafe fn from_repr_unchecked(#parameter: #alias) -> Self {
                    // The caller's repr decodes, so it is one of the discriminants above.
                    match #parameter {
                        #(#constants => Self::#variants,)*
                        _ => unsafe { ::core::hint::unreachable_unchecked() },
                    }
                }
            }
        };
    }
}
