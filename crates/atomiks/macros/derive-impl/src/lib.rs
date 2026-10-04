//! The logic of atomiks' derives: reads a type's definition, and writes its impls or the errors
//! that refuse it.
//!
//! It is a library of its own so that its tests run without a compiler's proc-macro context.
//!
//! [`expand_atom`] and [`expand_capability`] each take three steps: `parse` reads the definition
//! into `model`'s terms, the type an impl names and its shape, refusing what no impl could be
//! written for; `code` writes the shape's impl, beside the constants that check it as the user's
//! crate builds; and the [`Expansion`] holds that code, or each [`DeriveError`], beside a stub
//! where one keeps others from following.

mod code;
mod derive;
mod errors;
mod model;
mod parse;

pub use crate::derive::{Capability, Expansion, expand_atom, expand_capability};
pub use crate::errors::DeriveError;
