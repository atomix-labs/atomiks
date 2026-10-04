//! A value's fields: how it is built of them.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Member;

use crate::model::Field;

/// `Self`, of a struct of `PhantomData` markers beside at most one field that holds its value:
/// `markers_before`, then that field, of `value`, where it has one, then `markers_after`, each
/// marker built as `PhantomData`.
pub(super) fn built_with_markers(
    markers_before: &[Field], value: Option<(&Field, &TokenStream)>, markers_after: &[Field],
) -> TokenStream {
    let marker = quote!(::core::marker::PhantomData);
    let (field, value) = value.unzip();
    let fields = markers_before.iter().chain(field).chain(markers_after);
    let values = (markers_before.iter().map(|_| &marker))
        .chain(value)
        .chain(markers_after.iter().map(|_| &marker));
    build(&quote!(Self), fields, values)
}

/// What `constructor` builds of `values`, one for each of `fields` in declaration order: by name,
/// or by position where the fields have no names.
pub(super) fn build<'a, F, V>(constructor: &TokenStream, fields: F, values: V) -> TokenStream
where
    F: IntoIterator<Item = &'a Field>,
    V: IntoIterator<Item: ToTokens>,
{
    let mut fields = fields.into_iter().peekable();
    let values = values.into_iter();
    if let Some(Field { member: Member::Unnamed(_), .. }) = fields.peek() {
        quote!(#constructor(#(#values),*))
    } else {
        let members = fields.map(|field| &field.member);
        quote!(#constructor { #(#members: #values),* })
    }
}
