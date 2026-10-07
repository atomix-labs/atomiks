//! What the integration tests share, this crate's and the facade's: the checks of a value's repr,
//! validity and decodes, the `Atom` laws, the reading of a codegen fixture's assembly, the checks
//! of a field operation, and a packed struct and a pointer word written out as the derive writes
//! them.
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
// Loom's atomics have no `as_ptr`, and the field tests run off loom.
#[cfg(not(loom))]
pub(crate) mod field;
pub(crate) mod law;
pub(crate) mod packed;
pub(crate) mod pointer_word;
