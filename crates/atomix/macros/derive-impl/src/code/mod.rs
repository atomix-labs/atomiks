//! The code each derive writes.
//!
//! Each token it writes is spanned at the call site, so the user's lints read it as a macro's, but
//! the locals it names, which take the derive's definition site; each path it writes is absolute.
//! The user's own tokens, a field's type or a member, keep their spans.

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
use syn::Ident;

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
