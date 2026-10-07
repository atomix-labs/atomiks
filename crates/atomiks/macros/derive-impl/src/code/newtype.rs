//! A newtype's impls: `Atom`, each item the field's that holds the value, and each capability it
//! takes from that field.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::Ident;
use syn::spanned::Spanned;

use super::bound::{thread_bounds, thread_checks, where_clause};
use super::field::built_with_markers;
use super::repr::located_at;
use crate::derive::Capability;
use crate::model::{Field, Implementor, Newtype};

/// `Atom` for the newtype, its repr, validity and range the value's field's and its conversions
/// that field's, with the checks that stand beside the impl.
///
/// Its locals are named at `def_site`, the derive's definition site, so that no name of the user's,
/// a constant `repr` among them, resolves in their place.
pub(crate) fn newtype(implementor: &Implementor, newtype: &Newtype, def_site: Span) -> TokenStream {
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
    let to_tagged_repr = conversion("to_tagged_repr");
    let from_repr = conversion("from_repr");
    let from_repr_unchecked = conversion("from_repr_unchecked");
    let repr = Ident::new("repr", def_site);
    let field = Ident::new("field", def_site);
    let tags = Ident::new("tags", def_site);
    let from_field = built(newtype, &field.to_token_stream());
    // The call keeps its contract: the caller's repr decodes as `Self`, which `from_repr` decodes
    // it as exactly where it decodes as the value's field.
    let decoded = built(newtype, &quote!(unsafe { #from_repr_unchecked(#repr) }));
    let repr_type = quote!(<#ty as #atomiks::Atom>::Repr);
    let checks = checks(implementor, newtype);
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let where_clause =
        where_clause(implementor, bounds(implementor, newtype, &quote!([const] #atomiks::Atom)));
    // The impl keeps each promise of `Atom` as the value's field's does: the repr, range,
    // validity, tag width, alignment and conversions, its tagged repr among them, are its, and the
    // rest of `Self` is `PhantomData` markers, built alike on every decode. The value may cross
    // threads: `checks` or the where clause check or bound the type `Send` and `Sync`, or, where
    // its field is written as a pointer, whose own impl promises that it may cross, each marker.
    quote! {
        #checks
        #[automatically_derived]
        const unsafe impl #impl_generics #atomiks::Atom for #ident #ty_generics #where_clause {
            type Repr = #repr_type;
            type Validity = <#ty as #atomiks::Atom>::Validity;
            const REPRS: #atomiks::ReprRange<#repr_type> = <#ty as #atomiks::Atom>::REPRS;
            const TAG_WIDTH: ::core::primitive::u32 = <#ty as #atomiks::Atom>::TAG_WIDTH;
            const POINTEE_ALIGNMENT: #atomiks::__private::PointeeAlignment =
                <#ty as #atomiks::Atom>::POINTEE_ALIGNMENT;
            #[inline]
            fn to_repr(self) -> #repr_type {
                #to_repr(self.#member)
            }
            #[inline]
            fn to_tagged_repr(
                self, #tags: #atomiks::__private::Tags,
            ) -> (#repr_type, ::core::primitive::usize) {
                #to_tagged_repr(self.#member, #tags)
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
    let where_clause =
        where_clause(implementor, bounds(implementor, newtype, &quote!(#atomiks::#name)));
    quote! {
        #check
        #[automatically_derived]
        impl #impl_generics #atomiks::#name for #ident #ty_generics #where_clause {}
    }
}

/// `Self`, its value's field `value` and each marker `PhantomData`.
fn built(newtype: &Newtype, value: &TokenStream) -> TokenStream {
    let Newtype { markers_before, value: field, markers_after } = newtype;
    built_with_markers(markers_before, Some((field, value)), markers_after)
}

/// The checks of the newtype, each a constant beside the impl: that it may cross threads, as
/// `thread_checks` says, that its concrete value's repr is the one stated, and that a concrete
/// value written or marked as a thin pointer, whose markers alone the thread checks read, is one.
/// A wide pointer, two words, is written as one, so no check asks.
fn checks(implementor: &Implementor, newtype: &Newtype) -> TokenStream {
    let atomiks = &implementor.atomiks;
    let threads = thread_checks(implementor, newtype.fields());
    let value = &newtype.value;
    let ty = &value.ty;
    let repr = implementor.repr.as_ref().filter(|_| !value.is_generic).map(|repr| {
        quote!(const _: () = #atomiks::__private::assert_repr::<#ty, ::core::primitive::#repr>();)
    });
    let pointer = (value.is_pointer && !value.is_wide_pointer && !value.is_generic).then(|| {
        located_at(quote!(const _: () = #atomiks::__private::assert_pointer::<#ty>();), ty.span())
    });
    quote!(#threads #repr #pointer)
}

/// The newtype's bounds in an impl's where clause: `bound`, with the repr stated, on the
/// value's field where it names a parameter, `PtrAtom` too where it is written or marked as
/// a thin pointer, and those `thread_bounds` says.
fn bounds<'a>(
    implementor: &Implementor, newtype: &'a Newtype, bound: &TokenStream,
) -> impl Iterator<Item = TokenStream> + use<'a> {
    let value = &newtype.value;
    // An instance whose field has another repr is refused at the repr stated.
    let repr = implementor.repr.as_ref().map(|repr| {
        let span = Span::call_site().located_at(repr.span());
        quote_spanned!(span=> <Repr = ::core::primitive::#repr>)
    });
    let atomiks = &implementor.atomiks;
    let value_bound = value.is_generic.then(|| {
        let ty = &value.ty;
        quote!(#ty: #bound #repr)
    });
    let pointer_bound =
        (value.is_generic && value.is_pointer && !value.is_wide_pointer).then(|| {
            let ty = &value.ty;
            quote!(#ty: #atomiks::PtrAtom)
        });
    value_bound.into_iter().chain(pointer_bound).chain(thread_bounds(implementor, newtype.fields()))
}
