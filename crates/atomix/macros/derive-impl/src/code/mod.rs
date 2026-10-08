//! The code each derive writes.
//!
//! Each token it writes is spanned at the call site, so the user's lints read it as a macro's, but
//! the locals it names, which take the derive's definition site; each path it writes is absolute.
//! The user's own tokens keep their spans, a field's type among them, but the type's name and each
//! member the derive uses, which take the call site at the user's place, so that the `deprecated`
//! lint reads none of them: under a macro's opaque hygiene, as a `macro` 2.0's, they then resolve
//! at its definition site, not at the user's.

mod bound;
mod enum_with_fields;
mod field;
mod fieldless;
mod layout;
mod newtype;
mod packed;
mod pointer_enum;
mod pointer_word;
mod projection;
mod repr;
mod stub;
mod zero_width;

use proc_macro2::Span;
use syn::{Ident, Member};

pub(crate) use self::enum_with_fields::enum_with_fields;
pub(crate) use self::fieldless::fieldless;
pub(crate) use self::newtype::{capability, newtype};
pub(crate) use self::packed::packed;
pub(crate) use self::pointer_enum::pointer_enum;
pub(crate) use self::pointer_word::pointer_word;
pub(crate) use self::projection::{member_shown, shown};
pub(crate) use self::stub::{stub, stub_of_a_type_holding_a_place};
pub(crate) use self::zero_width::zero_width;

/// `ident`, spanned at `span`.
fn respan(ident: &Ident, span: Span) -> Ident {
    let mut respanned = ident.clone();
    respanned.set_span(span);
    respanned
}

/// `member`, a field's name or index, as the derive writes a use of it: at the user's place, but in
/// the derive's expansion, so that the `deprecated` lint, which reads no use a derive writes, reads
/// none of a deprecated type's or field's, wherever the type is declared, a macro's expansion too.
fn member_in_expansion(member: &Member) -> Member {
    let mut member = member.clone();
    match &mut member {
        Member::Named(named) => named.set_span(Span::call_site().located_at(named.span())),
        Member::Unnamed(index) => index.span = Span::call_site().located_at(index.span),
    }
    member
}
