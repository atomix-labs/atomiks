//! The repr an impl names, through `__private::SelectRepr`, which selects one wherever the type is
//! built; and the checks, located at what states the repr, that refuse it once where it
//! does not hold the value.

use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{Ident, Path};

use crate::model::Implementor;

/// The repr selected from the integer primitive `integer`, which a `#[repr]` or
/// `#[atom(repr = …)]` names: the integer, wherever an atomic cell on the target holds it.
pub(crate) fn selected_from(atomiks: &Path, integer: &Ident) -> TokenStream {
    quote!(<::core::primitive::#integer as #atomiks::__private::SelectRepr>::Repr)
}

/// A constant that refuses `implementor`'s type where no atomic cell on the target holds
/// `integer`, which a `#[repr]` or `#[atom(repr = …)]` names: a 128-bit one without 128-bit
/// atomics.
///
/// It names the type, or the integer where the type has parameters, which no constant beside the
/// impl can name.
pub(crate) fn integer_check(implementor: &Implementor, integer: &Ident) -> TokenStream {
    let Implementor { ident, generics, atomiks, .. } = implementor;
    let integer_type = quote!(::core::primitive::#integer);
    let named =
        if generics.params.is_empty() { ident.to_token_stream() } else { integer_type.clone() };
    let check = quote! {
        const _: () = #atomiks::__private::assert_width::<#named>(#integer_type::BITS);
    };
    located_at(check, integer.span())
}

/// The narrowest unsigned integer that holds `width` bits, a constant expression.
pub(crate) fn narrowest(atomiks: &Path, width: &TokenStream) -> TokenStream {
    let private = quote!(#atomiks::__private);
    quote!(<#private::Width<{ #private::narrowest_width(#width) }> as #private::SelectRepr>::Repr)
}

/// A constant that refuses `ty`, a value `width` bits wide, a constant expression, where the
/// integer `integer`, which `stated` names in an attribute, is narrower.
///
/// It names the integer stated, not the repr selected from it, so where no atomic cell on the
/// target holds the integer, [`integer_check`] alone refuses it.
pub(crate) fn stated_width_check(
    atomiks: &Path, ty: &Ident, integer: &TokenStream, stated: &Ident, width: &TokenStream,
) -> TokenStream {
    let check = quote! {
        const _: () = #atomiks::__private::assert_stated_width::<#ty, #integer>(#width);
    };
    located_at(check, stated.span())
}

/// `tokens`, each located at `span`, where an error they raise points, but resolved where it was.
fn located_at(tokens: TokenStream, span: Span) -> TokenStream {
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
