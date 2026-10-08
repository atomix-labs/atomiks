//! The `Atom` impl written in place of a refused one.

use proc_macro2::TokenStream;
use quote::quote;

use crate::model::Implementor;

/// An `Atom` impl that compiles whatever the type is, written beside the errors that refuse the
/// type's own, so that each use of the type as an atom raises no error of its own.
///
/// It is never run: the build fails with the errors beside it. Were one missing, its decode would
/// panic rather than claim, as `None`, that its one repr holds no value.
pub(crate) fn stub(implementor: &Implementor) -> TokenStream {
    stub_asking(implementor, &quote!(Self: ::core::marker::Copy))
}

/// [`stub`], for a type with a field written as a place, an atomic, a cell or a lock.
///
/// Such a type is `Copy` only where it derives `Copy` beside the error that refuses that impl, so
/// its stub asks `Copy` under a binder: rustc refuses a false bound that names no parameter, but
/// not one under a binder, so where the type is not `Copy` the stub adds no error to the field's.
pub(crate) fn stub_of_a_type_holding_a_place(implementor: &Implementor) -> TokenStream {
    stub_asking(implementor, &quote!(for<'__atomix> Self: ::core::marker::Copy))
}

/// The stub, its where clause asking `copy` beside the type's own predicates.
fn stub_asking(implementor: &Implementor, copy: &TokenStream) -> TokenStream {
    let Implementor { ident, generics, atomix, .. } = implementor;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let predicates = where_clause.into_iter().flat_map(|clause| &clause.predicates);
    quote! {
        #[automatically_derived]
        const unsafe impl #impl_generics #atomix::Atom for #ident #ty_generics
        where
            #(#predicates,)*
            #copy,
        {
            type Repr = ::core::primitive::u8;
            const REPRS: #atomix::ReprRange<::core::primitive::u8> = #atomix::ReprRange::new(0, 0);
            #[inline]
            fn to_repr(self) -> ::core::primitive::u8 {
                0
            }
            #[inline]
            fn from_repr(_: ::core::primitive::u8) -> ::core::option::Option<Self> {
                ::core::panic!("`#[derive(Atom)]` refused this type")
            }
        }
    }
}
