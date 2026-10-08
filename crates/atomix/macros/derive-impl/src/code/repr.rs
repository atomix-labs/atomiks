//! The repr an impl names, through `__private::SelectRepr`, which selects one wherever the type is
//! built; and the checks, located at what states the repr, that refuse it once where it
//! does not hold the value.

use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::ext::IdentExt;
use syn::{Ident, Path};

use crate::model::Implementor;

/// The repr selected from the integer primitive `integer`, which a `#[repr]` or
/// `#[atom(repr = …)]` names: the integer, wherever an atomic cell on the target holds it.
pub(crate) fn selected_from(atomix: &Path, integer: &Ident) -> TokenStream {
    quote!(<::core::primitive::#integer as #atomix::__private::SelectRepr>::Repr)
}

/// A constant that refuses `implementor`'s type where no atomic cell on the target holds `integer`,
/// which a `#[repr]` or `#[atom(repr = …)]` names: a 128-bit one without 128-bit atomics.
pub(crate) fn integer_check(implementor: &Implementor, integer: &Ident) -> TokenStream {
    let Implementor { ident, atomix, .. } = implementor;
    let (name, integer_type) = (name_shown(ident), quote!(::core::primitive::#integer));
    let check = quote! {
        const _: () = #atomix::__private::assert_width(#name, #integer_type::BITS);
    };
    located_at(check, integer.span())
}

/// The narrowest unsigned integer that holds `width` bits, a constant expression.
pub(crate) fn narrowest(atomix: &Path, width: &TokenStream) -> TokenStream {
    let private = quote!(#atomix::__private);
    quote!(<#private::Width<{ #private::narrowest_width(#width) }> as #private::SelectRepr>::Repr)
}

/// A constant that refuses the type `ident` names, a value `width` bits wide, a constant
/// expression, where no atomic word on the target holds it: located at `ident`, the type's name.
pub(crate) fn width_check(atomix: &Path, ident: &Ident, width: &TokenStream) -> TokenStream {
    let name = name_shown(ident);
    let check = quote!(const _: () = #atomix::__private::assert_width(#name, #width););
    located_at(check, ident.span())
}

/// A constant that refuses the type `ident` names, a value `width` bits wide, a constant
/// expression, where the integer `integer`, which `stated` names in an attribute, is narrower:
/// located at `stated`.
///
/// It names the integer stated, not the repr selected from it, so where no atomic cell on the
/// target holds the integer, [`integer_check`] alone refuses it.
pub(crate) fn stated_width_check(
    atomix: &Path, ident: &Ident, integer: &TokenStream, stated: &Ident, width: &TokenStream,
) -> TokenStream {
    let name = name_shown(ident);
    let assertion = quote!(#atomix::__private::assert_stated_width::<#integer>(#name, #width));
    let assertion = located_at(assertion, stated.span());
    quote!(const _: () = #assertion;)
}

/// A call, located at `stated`, that refuses `instance`, an instance of a type with parameters, as
/// [`stated_width_check`]'s constant refuses a type without: each instance's layout makes the call,
/// since only the instance knows its width.
pub(crate) fn instance_stated_width_assertion(
    atomix: &Path, instance: &TokenStream, integer: &TokenStream, stated: &Ident,
    width: &TokenStream,
) -> TokenStream {
    let assertion =
        quote!(#atomix::__private::assert_instance_stated_width::<#instance, #integer>(#width));
    located_at(assertion, stated.span())
}

/// The name a refusal shows of the type `ident` names: `Quote`, with no path, a raw name unraw.
fn name_shown(ident: &Ident) -> String {
    ident.unraw().to_string()
}

/// `tokens`, each located at `span`, where an error they raise points, but resolved where it was.
pub(super) fn located_at(tokens: TokenStream, span: Span) -> TokenStream {
    tokens
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(group) = &token {
                let mut located = Group::new(group.delimiter(), located_at(group.stream(), span));
                located.set_span(group.span().located_at(span));
                return TokenTree::Group(located);
            }
            token.set_span(token.span().located_at(span));
            token
        })
        .collect()
}
