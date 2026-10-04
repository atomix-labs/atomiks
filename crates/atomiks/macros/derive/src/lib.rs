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
/// It derives for five shapes:
///
/// - **A newtype**, a struct of one field beside any `PhantomData` markers: its repr, range,
///   validity and conversions are that field's.
/// - **A zero-width struct**, a unit struct or one of markers alone: its one value is zero, in a
///   `u8`.
/// - **A struct of several fields**: each field takes bits of its own, from bit 0 in declaration
///   order, as few as its reprs need, two's complement where that is fewer; the bits above extend
///   the last field of any bits, so a signed one's sign fills them. Its repr is the narrowest
///   unsigned integer that holds them, and its validity what its fields promise of their bits. With
///   parameters, it states its repr, which each instance is checked against as it is built, and
///   promises only that zero decodes, where each field's does. A field stored as a pointer is
///   refused.
/// - **A fieldless enum**, of unit variants alone, without parameters: each variant is its
///   discriminant, exactly as rustc evaluates it, in the integer its `#[repr]` names, C's `int` for
///   `#[repr(C)]`, which must hold each, or else the narrowest unsigned integer that holds each,
///   sign-extended where one is negative. Its validity says which reprs the discriminants take:
///   `Total` where they are every one, `TotalZeroNiche` every one but zero, else `ZeroValid` or
///   `ZeroNiche` as zero is one or not.
/// - **An enum with fields**: each variant's fields take bits as a struct's do, from bit 0, below a
///   tag of its discriminant, exactly as rustc evaluates it, above the widest variant's fields.
///   Where one variant alone has fields and none states its discriminant, the unit variants instead
///   take the reprs beside the range of that variant's last field of any bits, as `Option`'s `None`
///   takes one, wherever that is no wider than a tag. Its repr is the narrowest unsigned integer
///   that holds the tag or that field, and its validity what its variants promise of zero and of
///   every repr. With parameters, it states its repr, as a struct of several fields does, and
///   promises that zero decodes only where it states its discriminants and a unit variant's is 0,
///   stated or implied. A field stored as a pointer is refused.
///
/// A crate enables `#![feature(const_trait_impl)]` where the impl converts a field that names one
/// of the type's parameters, a lifetime too: a newtype's field that holds the value, or any field
/// of a struct of several fields or an enum with fields, a `PhantomData` among them. Every other
/// field converts through
/// functions bounded `const`, so a newtype whose markers alone name its parameters, a zero-width
/// struct, or a type whose `const` parameter no field names needs no gate.
///
/// An `Atomic` may cross threads, so the type must be `Send` and `Sync`: checked beside the impl,
/// or bounded where it has parameters, so `Wrap<*mut u8>` is refused, as is a type whose negative
/// impl says it is neither. A newtype whose field is written as a raw pointer, a `NonNull` or an
/// `Option` of one is neither, yet may cross, as `Atom` promises a pointer may: its markers alone
/// are checked.
///
/// `#[atom(crate = path)]` names atomiks where `::atomiks` does not, and `#[atom(repr = u64)]`
/// states the repr: a newtype's field must have it, a zero-width struct, a struct of several fields
/// or an enum with fields that fits it, or a fieldless enum without a `#[repr]`, takes it, and a
/// fieldless enum's `#[repr]` must name it. A 128-bit repr is refused on a target without 128-bit
/// atomics.
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
