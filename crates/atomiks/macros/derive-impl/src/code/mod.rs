//! The code each derive writes.
//!
//! Each token it writes is spanned at the call site, so the user's lints read it as a macro's, but
//! the locals it names, which take the derive's definition site; each path it writes is absolute.
//! The user's own tokens, a field's type or a member, keep their spans.

mod newtype;
mod stub;

pub(crate) use self::newtype::{atom, capability};
pub(crate) use self::stub::stub;
