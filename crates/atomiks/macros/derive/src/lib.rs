//! The derives of atomiks: `Atom`, and `AtomAdd`, `AtomOrd` and `AtomBitwise`, the
//! read-modify-writes a newtype takes from its field.
//!
//! atomiks re-exports each under its `derive` feature: depend on atomiks, not on this crate. The
//! logic is `atomiks-derive-impl`'s; this crate renders its errors as the compiler's.

#![feature(allow_internal_unstable, proc_macro_def_site, proc_macro_diagnostic)]
#![expect(
    internal_features,
    reason = "`allow_internal_unstable` lets a crate without `const_trait_impl` build the `const` impl \
              `Atom`'s derive writes"
)]

use atomiks_derive_impl::{Capability, DeriveError, Expansion, expand_atom, expand_capability};
use proc_macro::{Diagnostic, Level, Span, TokenStream};

/// Derives `Atom`, so that an `Atomic` holds the type.
///
/// So far it derives for a newtype, a struct of one field beside any `PhantomData` markers, whose
/// repr, range, validity and conversions are that field's. A crate that derives it for a type with
/// a parameter in that field enables `#![feature(const_trait_impl)]`; any other needs no feature.
///
/// An `Atomic` may cross threads, so the type must be `Send` and `Sync`: checked beside the impl,
/// or bounded where it has parameters, so `Wrap<*mut u8>` is refused, as is a type whose negative
/// impl says it is neither. A newtype whose field is written as a raw pointer, a `NonNull` or an
/// `Option` of one is neither, yet may cross, as `Atom` promises a pointer may: its markers alone
/// are checked.
///
/// `#[atom(crate = path)]` names atomiks where `::atomiks` does not, and `#[atom(repr = u64)]`
/// states the repr, which the field's must be.
#[proc_macro_derive(Atom, attributes(atom))]
#[allow_internal_unstable(const_trait_impl)]
pub fn derive_atom(input: TokenStream) -> TokenStream {
    emit(expand_atom(input.into(), Span::def_site().into()))
}

/// Derives `AtomAdd` for a newtype whose field has it: `add`, `sub` and their `fetch_` forms.
#[proc_macro_derive(AtomAdd, attributes(atom))]
pub fn derive_atom_add(input: TokenStream) -> TokenStream {
    emit(expand_capability(input.into(), Capability::Add))
}

/// Derives `AtomOrd` for a newtype whose field has it: `max`, `min` and their `fetch_` forms.
///
/// `AtomOrd` promises that the repr's order is the value's `Ord`, so the newtype's `Ord` must be
/// its field's: derive `Ord` too.
#[proc_macro_derive(AtomOrd, attributes(atom))]
pub fn derive_atom_ord(input: TokenStream) -> TokenStream {
    emit(expand_capability(input.into(), Capability::Ord))
}

/// Derives `AtomBitwise` for a newtype whose field has it: `and`, `or`, `xor`, `not` and their
/// `fetch_` forms.
#[proc_macro_derive(AtomBitwise, attributes(atom))]
pub fn derive_atom_bitwise(input: TokenStream) -> TokenStream {
    emit(expand_capability(input.into(), Capability::Bitwise))
}

/// The code a derive writes, each of its errors rendered first.
fn emit(expansion: Expansion) -> TokenStream {
    expansion.errors.into_iter().for_each(render);
    expansion.code.into()
}

/// Emits `error` as the compiler's, with its notes and help.
fn render(error: DeriveError) {
    let mut diagnostic = Diagnostic::spanned(error.span.unwrap(), Level::Error, error.message);
    for (span, note) in error.notes {
        diagnostic = match span {
            Some(span) => diagnostic.span_note(span.unwrap(), note),
            None => diagnostic.note(note),
        };
    }
    if let Some(help) = error.help {
        diagnostic = diagnostic.help(help);
    }
    diagnostic.emit();
}
