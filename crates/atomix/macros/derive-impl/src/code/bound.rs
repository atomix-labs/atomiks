//! What an impl asks of its type and its fields: the where clause, and the checks that
//! stand beside the impl.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Path;

use crate::model::{Field, Implementor};

/// The where clause of an impl for the type: its own predicates, `Self: Copy` where it has
/// parameters, then `predicates`; none where that is nothing.
pub(crate) fn where_clause<I: IntoIterator<Item = TokenStream>>(
    implementor: &Implementor, predicates: I,
) -> Option<TokenStream> {
    // A derived `Copy` bounds each parameter by `Copy`, so a generic type is `Copy`, as `Atom`
    // asks, only where it says so.
    let copy =
        (!implementor.generics.params.is_empty()).then(|| quote!(Self: ::core::marker::Copy));
    own_where_clause(implementor, copy.into_iter().chain(predicates))
}

/// The where clause of an item beside the impl, which names no `Self`: the type's own predicates,
/// then `predicates`; none where that is nothing.
pub(crate) fn own_where_clause<I: IntoIterator<Item = TokenStream>>(
    implementor: &Implementor, predicates: I,
) -> Option<TokenStream> {
    let own = implementor.generics.where_clause.iter().flat_map(|clause| &clause.predicates);
    let predicates: Vec<TokenStream> =
        own.map(ToTokens::to_token_stream).chain(predicates).collect();
    (!predicates.is_empty()).then(|| quote!(where #(#predicates),*))
}

/// Where a where clause that bounds a field sits, which decides what it asks of the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundSite {
    /// `lay_out`'s, which reads only constants.
    LayOut,
    /// A projection's, beside the impl.
    Projection,
    /// The `Atom` impl's, `const` as the impl is.
    Impl,
}

/// The bounds at `site` on each of `fields` that names a parameter, each `[const]` in the impl's
/// where clause: `PtrAtom` on a thin pointer; `Atom` on a wide one, two words; and `Atom` on any
/// other field, whose repr a projection and the impl ask to be stored as bits.
pub(crate) fn field_bounds<'f, I: IntoIterator<Item = &'f Field>>(
    atomix: &Path, fields: I, site: BoundSite,
) -> impl Iterator<Item = TokenStream> {
    let private = quote!(#atomix::__private);
    let generic_fields = fields.into_iter().filter(|field| field.is_generic);
    generic_fields.map(move |Field { ty, is_pointer, is_wide_pointer, .. }| {
        let is_thin_pointer = *is_pointer && !*is_wide_pointer;
        match (is_thin_pointer, *is_wide_pointer, site) {
            (true, _, BoundSite::LayOut | BoundSite::Projection) => quote!(#ty: #atomix::PtrAtom),
            (true, _, BoundSite::Impl) => quote!(#ty: [const] #atomix::PtrAtom),
            (false, true, BoundSite::Impl) => quote!(#ty: [const] #atomix::Atom),
            (false, _, BoundSite::LayOut) | (false, true, BoundSite::Projection) => {
                quote!(#ty: #atomix::Atom)
            },
            (false, false, BoundSite::Projection) => {
                quote!(#ty: #atomix::Atom<Repr: #private::FieldRepr>)
            },
            (false, false, BoundSite::Impl) => {
                quote!(#ty: [const] #atomix::Atom<Repr: [const] #private::FieldRepr>)
            },
        }
    })
}

/// The bounds in an impl's where clause that each value of the implementor, of `fields`, may cross
/// threads, as an `Atomic` of it may: `Send + Sync` on the type, where it has parameters, so no
/// constant beside the impl names it.
///
/// The type itself, not each field: a negative impl makes a type of `Send` fields neither. But a
/// field written as a pointer is neither, though its `Atom` impl promises that it may cross, so
/// beside one, each other field that names a parameter is bounded instead.
pub(crate) fn thread_bounds<'a, I: IntoIterator<Item = &'a Field>>(
    implementor: &Implementor, fields: I,
) -> Vec<TokenStream> {
    let send_and_sync = quote!(::core::marker::Send + ::core::marker::Sync);
    let fields: Vec<&Field> = fields.into_iter().collect();
    if fields.iter().any(|field| field.is_pointer) {
        return fields
            .into_iter()
            .filter(|field| field.is_generic && !field.is_pointer)
            .map(|Field { ty, .. }| quote!(#ty: #send_and_sync))
            .collect();
    }
    let is_generic = !implementor.generics.params.is_empty();
    is_generic.then(|| quote!(Self: #send_and_sync)).into_iter().collect()
}

/// The constants beside an impl that compile only where each value of the implementor, of
/// `fields`, may cross threads.
///
/// One checks the type `Send` and `Sync`, where it has no parameters. Else, or beside a field
/// written as a pointer, as [`thread_bounds`] says, one checks each other field that names no
/// parameter, so a field that never may cross refuses the type where it is defined.
pub(crate) fn thread_checks<'a, I: IntoIterator<Item = &'a Field>>(
    implementor: &Implementor, fields: I,
) -> TokenStream {
    let Implementor { ident, generics, atomix, .. } = implementor;
    let fields: Vec<&Field> = fields.into_iter().collect();
    let has_pointer = fields.iter().any(|field| field.is_pointer);
    if generics.params.is_empty() && !has_pointer {
        return send_and_sync_check(atomix, ident);
    }
    let checks = fields
        .into_iter()
        .filter(|field| !field.is_generic && !field.is_pointer)
        .map(|Field { ty, .. }| send_and_sync_check(atomix, ty));
    quote!(#(#checks)*)
}

/// A constant beside the impl that compiles only where `ty` is `Send` and `Sync`.
fn send_and_sync_check<T: ToTokens>(atomix: &Path, ty: &T) -> TokenStream {
    quote!(const _: () = #atomix::__private::assert_send_and_sync::<#ty>();)
}
