//! What the integration tests share, this crate's and the facade's: the checks of a value's repr,
//! validity and decodes.
//!
//! The facade's tests reach it by its path, not through a `testing` feature, which would publish
//! the checks in this crate.

#![allow(dead_code, reason = "each test binary uses part of this module")]

pub(crate) mod atom;
