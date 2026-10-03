//! What a test checks of a value's `Atom` impl, built-in or derived: its repr and validity.

use atomiks_core::Atom;
use atomiks_core::validity::Validity;

/// Compiles only where `T`'s repr is `R` and its validity `V`.
pub(crate) const fn repr_and_validity_are<T: Atom<Repr = R, Validity = V>, R, V: Validity>() {}
