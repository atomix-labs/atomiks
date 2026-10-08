//! What a derive reads of a type: what an impl for it names, and its shape.

use core::iter;

use syn::{Attribute, Expr, Generics, Ident, Member, Path, Type, Visibility};

use crate::errors::DeriveError;

/// A type's definition, as read: what an impl for the type names, and its shape, or each error
/// that refuses the type.
pub(crate) struct Input {
    /// What an impl for the type names.
    pub(crate) implementor: Implementor,
    /// Its shape, or why it is refused.
    pub(crate) shape: Result<Shape, Vec<DeriveError>>,
    /// Whether a field is written as a place, an atomic, a cell or a lock, which no `Copy` type
    /// holds: a refused type's stub impl then asks `Copy` so that it raises no error of its own.
    pub(crate) holds_a_place: bool,
}

/// What an impl for the type names: the type, and what `#[atom(…)]` states of it.
pub(crate) struct Implementor {
    /// The type's visibility, which a packed struct's projection takes.
    pub(crate) vis: Visibility,
    /// Its attributes that a packed struct's projection takes too: `#[non_exhaustive]` and
    /// `#[doc(hidden)]`.
    pub(crate) projection_attributes: Vec<Attribute>,
    /// The type's name.
    pub(crate) ident: Ident,
    /// Its parameters and where clause.
    pub(crate) generics: Generics,
    /// The path to atomix: `crate = …`, else `::atomix`.
    pub(crate) atomix: Path,
    /// The integer primitive `repr = …` states, which the impl's repr must be.
    pub(crate) repr: Option<Ident>,
}

/// How the type holds its value, which decides how its impl lays it out.
pub(crate) enum Shape {
    /// A newtype: one field holds the value, beside any `PhantomData` markers.
    Newtype(Box<Newtype>),
    /// A unit struct, or one of markers alone.
    ZeroWidth(ZeroWidth),
    /// Several fields that hold a value, each in declaration order, as a struct packs them.
    Packed(Vec<Field>),
    /// Several fields, one or two of them pointers, the others their tags, packed into the low bits
    /// the first pointer's alignment leaves clear, or into an integer word beside it.
    PointerWord(PointerWord),
    /// Unit variants alone, each stored as its discriminant.
    Fieldless(Fieldless),
    /// Variants, one at least written with parentheses or braces, each stored as its discriminant
    /// beside its fields.
    EnumWithFields(EnumWithFields),
    /// Variants, one at least holding a pointer, each stored in one pointer word: its discriminant
    /// in the low bits, above its pointers' own tags, and its pointer's address or its fields
    /// above the bits its pointers' alignment leaves clear.
    PointerEnum(EnumWithFields),
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

impl EnumWithFields {
    /// Whether a variant states its discriminant; where none does, each is its variant's index.
    pub(crate) fn has_stated_discriminant(&self) -> bool {
        self.variants.iter().any(|variant| variant.discriminant.is_some())
    }
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

/// A pointer word's fields: its pointers, and its tags, every other field, and how many words it
/// takes.
pub(crate) struct PointerWord {
    /// The pointer fields, one, or two thin ones, each beside its index among the fields, in
    /// declaration order.
    pub(crate) pointers: Vec<(usize, Field)>,
    /// The tag fields, in declaration order.
    pub(crate) tag_fields: Vec<Field>,
    /// Which repr its impl stores it as: how many words it takes, and where its tags lie.
    pub(crate) repr: WordRepr,
}

impl PointerWord {
    /// Every field, in declaration order.
    pub(crate) fn fields(&self) -> impl Iterator<Item = &Field> + Clone {
        let pointers = self.pointers.iter().map(|(index, pointer)| (*index, pointer));
        pointers_among_tags(pointers, &self.tag_fields).into_iter()
    }

    /// The pointer fields, in declaration order.
    pub(crate) fn pointer_fields(&self) -> impl Iterator<Item = &Field> + Clone {
        self.pointers.iter().map(|(_, pointer)| pointer)
    }
}

/// Each part of a struct or a variant of pointer fields beside tags, in declaration order: `tags`,
/// each tag's, with each pointer's part of `pointers` put at its index, which ascend.
pub(crate) fn pointers_among_tags<T, P, I>(pointers: P, tags: I) -> Vec<T>
where
    P: IntoIterator<Item = (usize, T)>,
    I: IntoIterator<Item = T>,
{
    let mut parts: Vec<T> = tags.into_iter().collect();
    for (index, pointer) in pointers {
        parts.insert(index, pointer);
    }
    parts
}

/// Which repr a pointer word's impl stores it as: how many words it takes, and where its tags lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordRepr {
    /// One, its tags in its pointer's low bits: where `repr = u64` or `usize` states it, or the
    /// struct has parameters and states no repr.
    One,
    /// Two, of pointers, a second one or a wide one's metadata, its tags in the first pointer's
    /// low bits.
    Pointers,
    /// Two, its tags in an integer word beside its one pointer: where `repr = u128` or `i128`
    /// states it.
    IntegerWord,
    /// One where its tags and its pointer's take no more bits than an alignment leaves clear, else
    /// two, its tags in an integer word: a struct of one pointer and no parameters that states no
    /// repr.
    Selected,
}

/// A field, as an impl reads and builds it.
pub(crate) struct Field {
    /// Its name, or its index in a tuple struct.
    pub(crate) member: Member,
    /// Its visibility, which its place in a packed struct's projection takes.
    pub(crate) vis: Visibility,
    /// Its doc comments, which its place in a packed struct's projection carries.
    pub(crate) docs: Vec<Attribute>,
    /// Its type, with the user's spans.
    pub(crate) ty: Type,
    /// Whether its type names a parameter of the type, so that an impl bounds it in its where
    /// clause rather than checking it once beside the impl.
    pub(crate) is_generic: bool,
    /// Whether it is written as a raw pointer, a `NonNull` or an `Option` of one, or
    /// marked `#[atom(ptr)]`, which need not be `Send` or `Sync`: `Atom` promises such a
    /// value may cross threads.
    pub(crate) is_pointer: bool,
    /// Whether it is written as a pointer to a value of no size known at compile time, a slice, a
    /// `str` or a trait object, whose pointer holds its metadata beside its address: two words.
    pub(crate) is_wide_pointer: bool,
}
