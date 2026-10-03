//! A newtype's impls: `Atom`, each item the field's that holds the value, and each capability it
//! takes from that field.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Ident, Member, Path};

use crate::derive::Capability;
use crate::model::{Field, Implementor, Newtype};

/// `Atom` for the newtype, its repr, validity and range the value's field's and its conversions
/// that field's, with the checks that stand beside the impl.
///
/// Its locals are named at `def_site`, the derive's definition site, so that no name of the user's,
/// a constant `repr` among them, resolves in their place.
pub(crate) fn atom(implementor: &Implementor, newtype: &Newtype, def_site: Span) -> TokenStream {
    let Implementor { ident, generics, atomiks, .. } = implementor;
    let Field { member, ty, is_generic, .. } = &newtype.value;
    // A concrete field's conversions go through `__private`, whose bound is `const`, so the user's
    // crate needs no `const_trait_impl`; a generic field's are the trait's, the only calls that
    // keep the impl's `[const]` bound.
    let conversion = |name: &str| {
        let name = Ident::new(name, Span::call_site());
        if *is_generic {
            quote!(<#ty as #atomiks::Atom>::#name)
        } else {
            quote!(#atomiks::__private::#name::<#ty>)
        }
    };
    let to_repr = conversion("to_repr");
    let from_repr = conversion("from_repr");
    let from_repr_unchecked = conversion("from_repr_unchecked");
    let repr = Ident::new("repr", def_site);
    let field = Ident::new("field", def_site);
    let from_field = build(newtype, &field.to_token_stream());
    // The call keeps its contract: the caller's repr decodes as `Self`, which `from_repr` decodes
    // it as exactly where it decodes as the value's field.
    let decoded = build(newtype, &quote!(unsafe { #from_repr_unchecked(#repr) }));
    let repr_type = quote!(<#ty as #atomiks::Atom>::Repr);
    let checks = checks(implementor, newtype);
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let where_clause = where_clause(implementor, newtype, &quote!([const] #atomiks::Atom));
    // The impl keeps each promise of `Atom` as the value's field's does: the repr, range,
    // validity and conversions are its, and the rest of `Self` is `PhantomData` markers, built
    // alike on every decode. The value may cross threads: `checks` or the where clause check or
    // bound the type `Send` and `Sync`, or, where its field is written as a pointer, whose own
    // impl promises that it may cross, each marker.
    quote! {
        #checks
        #[automatically_derived]
        const unsafe impl #impl_generics #atomiks::Atom for #ident #ty_generics #where_clause {
            type Repr = #repr_type;
            type Validity = <#ty as #atomiks::Atom>::Validity;
            const REPRS: #atomiks::ReprRange<#repr_type> = <#ty as #atomiks::Atom>::REPRS;
            #[inline]
            fn to_repr(self) -> #repr_type {
                #to_repr(self.#member)
            }
            #[inline]
            fn from_repr(#repr: #repr_type) -> ::core::option::Option<Self> {
                match #from_repr(#repr) {
                    ::core::option::Option::Some(#field) => ::core::option::Option::Some(#from_field),
                    ::core::option::Option::None => ::core::option::Option::None,
                }
            }
            #[inline]
            unsafe fn from_repr_unchecked(#repr: #repr_type) -> Self {
                #decoded
            }
        }
    }
}

/// `capability` for the newtype, where its value's field has it.
///
/// A concrete field is checked beside the impl, since the capability's supertraits ask only for the
/// repr and validity it needs, and a field may have those without it: `Saturating<u32>` does,
/// though its add saturates where its repr's wraps.
pub(crate) fn capability(
    implementor: &Implementor, newtype: &Newtype, capability: Capability,
) -> TokenStream {
    let Implementor { ident, generics, atomiks, .. } = implementor;
    let name = Ident::new(capability.name(), Span::call_site());
    let Field { ty, is_generic, .. } = &newtype.value;
    let check = (!is_generic).then(|| {
        let assertion = Ident::new(capability.assertion(), Span::call_site());
        quote!(const _: () = #atomiks::__private::#assertion::<#ty>();)
    });
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let where_clause = where_clause(implementor, newtype, &quote!(#atomiks::#name));
    quote! {
        #check
        #[automatically_derived]
        impl #impl_generics #atomiks::#name for #ident #ty_generics #where_clause {}
    }
}

/// `Self`, its value's field `value` and each marker `PhantomData`.
fn build(newtype: &Newtype, value: &TokenStream) -> TokenStream {
    let marker = quote!(::core::marker::PhantomData);
    let values = (newtype.markers_before.iter().map(|_| &marker))
        .chain([value])
        .chain(newtype.markers_after.iter().map(|_| &marker));
    if let Member::Named(_) = newtype.value.member {
        let members = newtype.fields().map(|field| &field.member);
        quote!(Self { #(#members: #values),* })
    } else {
        quote!(Self(#(#values),*))
    }
}

/// The checks of the newtype, each a constant beside the impl: that it may cross threads, and
/// that its concrete value's repr is the one stated.
///
/// Where it has no parameters, the type is checked `Send` and `Sync`: a negative impl makes a type
/// of `Send` fields neither. But a field written as a pointer is neither, though its `Atom` impl
/// promises that it may cross, so beside one, or where the type has parameters, which the where
/// clause bounds instead, each other field that names none is checked.
fn checks(implementor: &Implementor, newtype: &Newtype) -> TokenStream {
    let Implementor { ident, generics, atomiks, .. } = implementor;
    let has_pointer = newtype.fields().any(|field| field.is_pointer);
    let threads: Vec<TokenStream> = if generics.params.is_empty() && !has_pointer {
        vec![send_and_sync_check(atomiks, ident)]
    } else {
        newtype
            .fields()
            .filter(|field| !field.is_generic && !field.is_pointer)
            .map(|Field { ty, .. }| send_and_sync_check(atomiks, ty))
            .collect()
    };
    let value = &newtype.value;
    let repr = implementor.repr.as_ref().filter(|_| !value.is_generic).map(|repr| {
        let ty = &value.ty;
        quote!(const _: () = #atomiks::__private::assert_repr::<#ty, ::core::primitive::#repr>();)
    });
    quote!(#(#threads)* #repr)
}

/// A constant beside the impl that compiles only where `ty` is `Send` and `Sync`.
fn send_and_sync_check<T: ToTokens>(atomiks: &Path, ty: &T) -> TokenStream {
    quote!(const _: () = #atomiks::__private::assert_send_and_sync::<#ty>();)
}

/// The where clause of an impl for the newtype: the type's own predicates; `bound`, with the repr
/// stated, on the value's field where it names a parameter; and, since an `Atomic` may cross
/// threads, `Send + Sync` on the type where it has parameters, or beside a field written as a
/// pointer, on each other field that names one.
fn where_clause(
    implementor: &Implementor, newtype: &Newtype, bound: &TokenStream,
) -> Option<TokenStream> {
    let own = implementor.generics.where_clause.iter().flat_map(|clause| &clause.predicates);
    // A derived `Copy` bounds each parameter by `Copy`, so a generic type is `Copy`, as `Atom`
    // asks, only where it says so.
    let copy =
        (!implementor.generics.params.is_empty()).then(|| quote!(Self: ::core::marker::Copy));
    let value = &newtype.value;
    // An instance whose field has another repr is refused at the repr stated.
    let repr = implementor.repr.as_ref().map(|repr| {
        let span = Span::call_site().located_at(repr.span());
        quote_spanned!(span=> <Repr = ::core::primitive::#repr>)
    });
    let value_bound = value.is_generic.then(|| {
        let ty = &value.ty;
        quote!(#ty: #bound #repr)
    });
    let send_and_sync = quote!(::core::marker::Send + ::core::marker::Sync);
    let threads: Vec<TokenStream> = if newtype.fields().any(|field| field.is_pointer) {
        newtype
            .fields()
            .filter(|field| field.is_generic && !field.is_pointer)
            .map(|Field { ty, .. }| quote!(#ty: #send_and_sync))
            .collect()
    } else {
        copy.is_some().then(|| quote!(Self: #send_and_sync)).into_iter().collect()
    };
    let predicates: Vec<TokenStream> =
        own.map(ToTokens::to_token_stream).chain(copy).chain(value_bound).chain(threads).collect();
    (!predicates.is_empty()).then(|| quote!(where #(#predicates),*))
}
