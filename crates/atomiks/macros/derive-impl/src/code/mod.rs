//! The code each derive writes.
//!
//! Each token it writes is spanned at the call site, so the user's lints read it as a macro's, but
//! the locals it names, which take the derive's definition site; each path it writes is absolute.
//! The user's own tokens, a field's type or a member, keep their spans.

mod bound;
mod field;
mod fieldless;
mod newtype;
mod repr;
mod stub;
mod zero_width;

pub(crate) use self::fieldless::fieldless;
pub(crate) use self::newtype::{capability, newtype};
pub(crate) use self::stub::stub;
pub(crate) use self::zero_width::zero_width;
