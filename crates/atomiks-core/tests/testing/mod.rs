//! What the integration tests share, this crate's and the facade's: the checks of a value's repr,
//! validity and decodes, the `Atom` laws, and the reading of a codegen fixture's assembly.
//!
//! The facade's tests reach it by its path, not through a `testing` feature: one would publish the
//! laws in this crate, and proptest with them, and the reader must compile into the test that uses
//! it, which builds its own crate's fixture from the paths cargo gives that test.

#![allow(
    dead_code,
    unused_imports,
    unused_macros,
    reason = "each test binary uses part of this module"
)]

pub(crate) mod atom;
pub(crate) mod codegen;
pub(crate) mod law;
