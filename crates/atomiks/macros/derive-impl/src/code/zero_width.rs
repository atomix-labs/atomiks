//! A zero-width struct's `Atom`: its one value, stored as zero.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use super::bound::{thread_bounds, thread_checks, where_clause};
use super::field::built_with_markers;
use super::repr;
use crate::model::{Implementor, ZeroWidth};

/// `Atom` for the zero-width struct, its one value stored as zero in a `u8`, or in the repr stated,
/// with the checks that stand beside the impl.
///
/// Its locals are named at `def_site`, as a newtype's are.
pub(crate) fn zero_width(
    implementor: &Implementor, zero_width: &ZeroWidth, def_site: Span,
) -> TokenStream {
    let Implementor { ident, generics, atomiks, repr: stated, .. } = implementor;
    let (repr_type, repr_check) = stated.as_ref().map_or_else(
        || (quote!(::core::primitive::u8), TokenStream::new()),
        |stated| (repr::selected_from(atomiks, stated), repr::integer_check(implementor, stated)),
    );
    let value = built(zero_width);
    let parameter = Ident::new("repr", def_site);
    let thread_checks = thread_checks(implementor, zero_width.markers());
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let where_clause = where_clause(implementor, thread_bounds(implementor, zero_width.markers()));
    // The impl keeps each promise of `Atom`: its one value is zero, the one repr in its range,
    // which decodes, so its validity is `ZeroValid`; every value is alike, and may cross threads,
    // as `thread_checks` or the where clause check or bound the type.
    quote! {
        #thread_checks
        #repr_check
        #[automatically_derived]
        const unsafe impl #impl_generics #atomiks::Atom for #ident #ty_generics #where_clause {
            type Repr = #repr_type;
            type Validity = #atomiks::validity::ZeroValid;
            const REPRS: #atomiks::ReprRange<#repr_type> = #atomiks::ReprRange::new(0, 0);
            #[inline]
            fn to_repr(self) -> #repr_type {
                0
            }
            #[inline]
            fn from_repr(#parameter: #repr_type) -> ::core::option::Option<Self> {
                match #parameter {
                    0 => ::core::option::Option::Some(#value),
                    _ => ::core::option::Option::None,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(_: #repr_type) -> Self {
                #value
            }
        }
    }
}

/// `Self`, each marker `PhantomData`: the struct's one value.
fn built(zero_width: &ZeroWidth) -> TokenStream {
    match zero_width {
        ZeroWidth::Unit => quote!(Self),
        ZeroWidth::Markers(markers) => built_with_markers(markers, None, &[]),
    }
}
