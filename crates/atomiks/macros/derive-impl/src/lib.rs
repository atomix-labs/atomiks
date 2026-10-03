//! The logic of atomiks' derives: reads a type's definition, and writes its impls or the errors
//! that refuse it.
//!
//! It is a library of its own so that its tests run without a compiler's proc-macro context.

mod code;
mod derive;
mod errors;
mod model;
mod parse;

pub use crate::derive::{Capability, Expansion, expand_atom, expand_capability};
pub use crate::errors::DeriveError;
