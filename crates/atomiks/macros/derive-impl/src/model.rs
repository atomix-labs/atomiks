//! What a derive reads of a type: what an impl for it names, and its shape.

use core::iter;

use syn::{Expr, Generics, Ident, Member, Path, Type};

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
    ZeroWidth(ZeroWidth),
    /// Several fields that hold a value, each in declaration order, as a struct packs them.
    Packed(Vec<Field>),
    /// Unit variants alone, each stored as its discriminant.
    Fieldless(Fieldless),
    /// Variants, one at least written with parentheses or braces, each stored as its discriminant
    /// beside its fields.
    EnumWithFields(EnumWithFields),
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

/// A zero-width struct: a unit struct, or one of `PhantomData` markers alone, so that its impl
/// builds its one value alike on every decode.
pub(crate) enum ZeroWidth {
    /// `struct Marker;`, whose value is `Self`.
    Unit,
    /// The markers in braces or parentheses, in declaration order, each built as `PhantomData`;
    /// none, for `struct Marker {}`.
    Markers(Vec<Field>),
}

impl ZeroWidth {
    /// Its markers, in declaration order: none for a unit struct.
    pub(crate) fn markers(&self) -> &[Field] {
        match self {
            Self::Unit => &[],
            Self::Markers(markers) => markers,
        }
    }
}

/// A fieldless enum's variants, each a unit stored as its discriminant, and the integer that stores
/// them.
pub(crate) struct Fieldless {
    /// Which integer its impl stores the discriminants as.
    pub(crate) repr: EnumRepr,
    /// Each variant's name, in declaration order.
    pub(crate) variants: Vec<Ident>,
}

/// What an enum with fields stores: each variant's discriminant, of the integer named, beside its
/// fields.
pub(crate) struct EnumWithFields {
    /// The integer its discriminants are, the one its `#[repr]` names; `None` for `isize`, where
    /// none does.
    pub(crate) integer: Option<Ident>,
    /// Each variant, in declaration order.
    pub(crate) variants: Vec<Variant>,
}

/// A variant of an enum with fields.
pub(crate) struct Variant {
    /// Its name.
    pub(crate) ident: Ident,
    /// Whether it is written as a unit, with neither parentheses nor braces.
    pub(crate) is_written_as_unit: bool,
    /// Its fields, in declaration order: none for a unit variant, which holds no value but its
    /// discriminant, however it is written.
    pub(crate) fields: Vec<Field>,
    /// Its discriminant, where it states one, with the user's spans.
    pub(crate) discriminant: Option<Expr>,
}

/// Which integer a fieldless enum's impl stores its discriminants as.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EnumRepr {
    /// The one its `#[repr]` names, `u8` of `#[repr(u8)]`: the discriminants' own type.
    Integer(Ident),
    /// C's `int`, which `#[repr(C)]`, the hint given, names, and which must hold each
    /// discriminant: rustc widens the enum past it with a warning alone.
    C(Ident),
    /// The one `#[atom(repr = …)]` states where no `#[repr]` names one,
    /// which each discriminant must fit.
    Stated(Ident),
    /// Where neither names one, the narrowest unsigned integer that holds each discriminant.
    Selected,
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
