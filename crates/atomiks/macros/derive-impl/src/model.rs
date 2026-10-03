//! What a derive reads of a type: what an impl for it names, and its shape.

use core::iter;

use syn::{Generics, Ident, Member, Path, Type};

use crate::errors::DeriveError;

/// A type's definition, as read: what an impl for the type names, and its shape, or each error
/// that refuses the type.
pub(crate) struct Input {
    /// What an impl for the type names.
    pub(crate) implementor: Implementor,
    /// Its shape, or why it is refused.
    pub(crate) shape: Result<Shape, Vec<DeriveError>>,
}

/// What an impl for the type names: the type, and what `#[atom(…)]` states of it.
pub(crate) struct Implementor {
    /// The type's name.
    pub(crate) ident: Ident,
    /// Its parameters and where clause.
    pub(crate) generics: Generics,
    /// The path to atomiks: `crate = …`, else `::atomiks`.
    pub(crate) atomiks: Path,
    /// The integer primitive `repr = …` states, which the impl's repr must be.
    pub(crate) repr: Option<Ident>,
}

/// How the type holds its value, which decides how its impl lays it out.
#[expect(
    clippy::large_enum_variant,
    reason = "a derive reads one shape, so its size costs nothing"
)]
pub(crate) enum Shape {
    /// A newtype: one field holds the value, beside any `PhantomData` markers.
    Newtype(Newtype),
    /// A unit struct, or one of markers alone.
    ZeroWidth,
    /// Several fields that hold a value, as a struct packs them.
    Packed,
    /// Variants that hold no fields.
    Fieldless,
    /// Variants, one of them at least holding fields.
    EnumWithFields,
}

impl Shape {
    /// What the shape is called in a message: "a fieldless enum".
    pub(crate) const fn noun(&self) -> &'static str {
        match self {
            Self::Newtype(_) => "a newtype",
            Self::ZeroWidth => "a struct of no field but markers",
            Self::Packed => "a struct of several fields",
            Self::Fieldless => "a fieldless enum",
            Self::EnumWithFields => "an enum with fields",
        }
    }
}

/// A newtype's fields: the one that holds the value, and the `PhantomData` markers split around it
/// so that a value is built with each field in its place.
pub(crate) struct Newtype {
    /// The markers declared before the value's field.
    pub(crate) markers_before: Vec<Field>,
    /// The field that holds the value.
    pub(crate) value: Field,
    /// The markers declared after it.
    pub(crate) markers_after: Vec<Field>,
}

impl Newtype {
    /// Every field, in declaration order.
    pub(crate) fn fields(&self) -> impl Iterator<Item = &Field> {
        self.markers_before.iter().chain(iter::once(&self.value)).chain(&self.markers_after)
    }
}

/// A field, as an impl reads and builds it.
pub(crate) struct Field {
    /// Its name, or its index in a tuple struct.
    pub(crate) member: Member,
    /// Its type, with the user's spans.
    pub(crate) ty: Type,
    /// Whether its type names a parameter of the type, so that an impl bounds it in its where
    /// clause rather than checking it once beside the impl.
    pub(crate) is_generic: bool,
    /// Whether it is written as a raw pointer, a `NonNull` or an `Option` of one, which need not
    /// be `Send` or `Sync`: `Atom` promises such a value may cross threads.
    pub(crate) is_pointer: bool,
}
