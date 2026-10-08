//! Reads a derive's input into the model, refusing what no impl could be written for.

use core::iter;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Error as SyntaxError, Expr, ExprPath,
    Field as FieldDefinition, Fields, GenericArgument, GenericParam, Generics, Ident, Member, Meta,
    Path, PathArguments, PathSegment, Token, Type, TypeGroup, TypeParen, parse2,
};

use crate::code::{member_shown, shown};
use crate::errors::DeriveError;
use crate::model::{
    EnumRepr, EnumWithFields, Field, Fieldless, Implementor, Input, Newtype, PointerWord, Shape,
    Variant, WordRepr, ZeroWidth,
};

/// The integer primitives `repr = …` may state.
const INTEGERS: [&str; 12] =
    ["u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize"];

/// Reads `input`, a type's definition, or gives syn's errors where it does not parse.
pub(crate) fn input(input: TokenStream) -> Result<Input, Vec<DeriveError>> {
    let DeriveInput { attrs, vis, mut ident, generics, data } = parse2::<DeriveInput>(input)
        .map_err(|error: SyntaxError| {
            error.into_iter().map(DeriveError::from).collect::<Vec<_>>()
        })?;
    let mut errors = Vec::new();
    let (atomix, repr) = options(&attrs, &mut errors);
    let holds_a_place = holds_a_place(&data);
    let shape = shape(&ident, &generics, &attrs, repr.as_ref(), data, &mut errors);
    let projection_attributes =
        attrs.iter().filter(|attr| is_projection_attribute(attr)).cloned().collect();
    let name_span = ident.span();
    ident.set_span(Span::call_site().located_at(name_span));
    let implementor =
        Implementor { vis, projection_attributes, ident, name_span, generics, atomix, repr };
    let shape = match shape {
        Some(shape) if errors.is_empty() => Ok(shape),
        _ => Err(errors),
    };
    Ok(Input { implementor, shape, holds_a_place })
}

/// Whether a packed struct's projection takes `attr` as the struct does: `#[non_exhaustive]`,
/// which keeps a pattern of the projection open as one of the struct, and `#[doc(hidden)]`.
fn is_projection_attribute(attr: &Attribute) -> bool {
    match &attr.meta {
        Meta::Path(path) => path.is_ident("non_exhaustive"),
        Meta::List(list) => list.path.is_ident("doc") && list.tokens.to_string() == "hidden",
        Meta::NameValue(_) => false,
    }
}

/// What `#[atom(…)]` states: the path to atomix, `::atomix` where none is, and the repr.
///
/// Each key that does not read adds an error and leaves the rest to read, so a stub impl still
/// names atomix by the path stated.
fn options(attrs: &[Attribute], errors: &mut Vec<DeriveError>) -> (Path, Option<Ident>) {
    let mut atomix = None;
    let mut repr = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("atom")) {
        let keys = match attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
            Ok(keys) => keys,
            Err(error) => {
                errors.extend(error.into_iter().map(DeriveError::from));
                continue;
            },
        };
        for key in &keys {
            if key.path().is_ident("crate") {
                set_once(&mut atomix, key, crate_path(key), errors);
            } else if key.path().is_ident("repr") {
                set_once(&mut repr, key, integer(key), errors);
            } else {
                errors.push(
                    DeriveError::new(
                        key.path().span(),
                        format!("`{}` is not a key of `#[atom]`", key.path().to_token_stream()),
                    )
                    .help(
                        "`#[atom]` takes `crate = <path>` and `repr = <integer primitive>`"
                            .to_owned(),
                    ),
                );
            }
        }
    }
    (atomix.unwrap_or_else(default_atomix), repr)
}

/// `::atomix`, the path to atomix where `crate = …` states none.
fn default_atomix() -> Path {
    let mut path = Path::from(Ident::new("atomix", Span::call_site()));
    path.leading_colon = Some(<Token![::]>::default());
    path
}

/// Stores `value`, what `key` states, in `slot`, unless an earlier key stated it; adds an error
/// where the key is stated twice, or its value does not read.
fn set_once<T: Spanned>(
    slot: &mut Option<T>, key: &Meta, value: Result<T, DeriveError>, errors: &mut Vec<DeriveError>,
) {
    match (&*slot, value) {
        (Some(first), _) => errors.push(
            DeriveError::new(
                key.span(),
                format!("`{}` is stated twice", key.path().to_token_stream()),
            )
            .note(Some(first.span()), "stated first here".to_owned()),
        ),
        (None, Ok(value)) => *slot = Some(value),
        (None, Err(error)) => errors.push(error),
    }
}

/// The value `key = value` states, which a key alone or a list does not.
fn value(key: &Meta) -> Result<&Expr, DeriveError> {
    if let Meta::NameValue(pair) = key {
        return Ok(&pair.value);
    }
    let name = key.path().to_token_stream();
    Err(DeriveError::new(key.span(), format!("`{name}` takes a value"))
        .help(format!("write `{name} = …`")))
}

/// The path `crate = path` states.
fn crate_path(key: &Meta) -> Result<Path, DeriveError> {
    let value = value(key)?;
    if let Expr::Path(ExprPath { qself: None, path, .. }) = value {
        return Ok(path.clone());
    }
    Err(DeriveError::new(value.span(), "`crate` takes a path".to_owned())
        .help("name atomix as the crate reaches it, such as `crate = ::atomix`".to_owned()))
}

/// The integer primitive `repr = …` states.
fn integer(key: &Meta) -> Result<Ident, DeriveError> {
    let value = value(key)?;
    if let Expr::Path(ExprPath { qself: None, path, .. }) = value
        && let Some(ident) = path.get_ident()
        && is_integer(ident)
    {
        return Ok(ident.clone());
    }
    Err(DeriveError::new(
        value.span(),
        format!("`{}` is not an integer primitive", value.to_token_stream()),
    )
    .help("state one, such as `repr = u64`".to_owned()))
}

/// Whether `ident` names an integer primitive.
fn is_integer(ident: &Ident) -> bool {
    INTEGERS.iter().any(|integer| ident == integer)
}

/// The shape of `data`, that of the type `ident` of `generics` and `attrs`, which states `repr` in
/// `#[atom]`; `None` for a union or an enum of no variants, which have none an impl could encode,
/// or another shape refused. Each refusal adds an error.
fn shape(
    ident: &Ident, generics: &Generics, attrs: &[Attribute], repr: Option<&Ident>, data: Data,
    errors: &mut Vec<DeriveError>,
) -> Option<Shape> {
    match data {
        Data::Struct(data) => {
            let is_unit = matches!(data.fields, Fields::Unit);
            let fields = fields(data.fields, generics, Owner::Struct(ident), errors);
            let shape = struct_shape(fields, is_unit);
            let Shape::Packed(fields) = shape else {
                return Some(shape);
            };
            if fields.iter().any(|field| field.is_pointer) {
                Some(pointer_word_shape(fields, generics, repr, errors))
            } else {
                repr_stated_where_generic(
                    Shape::Packed(fields),
                    generics,
                    repr,
                    "a struct of several fields",
                    errors,
                )
            }
        },
        Data::Enum(data) => enum_shape(ident, generics, attrs, repr, data, errors),
        Data::Union(data) => {
            errors.push(
                DeriveError::new(
                    data.union_token.span,
                    format!("`{ident}` is a union, so no repr could say which field it holds"),
                )
                .help("use an enum, whose variant says which".to_owned()),
            );
            None
        },
    }
}

/// `shape`, that of a type of `generics` whose instances lay out their fields by their reprs, and
/// which `#[atom]` states `repr` of; `None` where it has parameters but states no repr, which adds
/// an error naming the shape as `noun`.
fn repr_stated_where_generic(
    shape: Shape, generics: &Generics, repr: Option<&Ident>, noun: &str,
    errors: &mut Vec<DeriveError>,
) -> Option<Shape> {
    if generics.params.is_empty() || repr.is_some() {
        return Some(shape);
    }
    errors.push(
        DeriveError::new(
            generics.span(),
            format!("{noun} derives `Atom` with parameters only where it states its repr"),
        )
        .note(
            None,
            "each instance lays its fields out by their reprs, so how many bits it needs is known only once they are"
                .to_owned(),
        )
        .help(
            "state a repr every instance fits, such as `#[atom(repr = u64)]`, which each one is checked against as it is built"
                .to_owned(),
        ),
    );
    None
}

/// A struct's shape, by how many of its fields are not `PhantomData` markers: packed where several
/// are, a newtype where one is, zero-width where none is. A unit struct is zero-width.
fn struct_shape(fields: Vec<Field>, is_unit: bool) -> Shape {
    if fields.iter().filter(|field| !is_marker(&field.ty)).count() > 1 {
        return Shape::Packed(fields);
    }
    let mut fields = fields.into_iter().peekable();
    let markers_before: Vec<Field> =
        iter::from_fn(|| fields.next_if(|field| is_marker(&field.ty))).collect();
    let Some(value) = fields.next() else {
        return Shape::ZeroWidth(if is_unit {
            ZeroWidth::Unit
        } else {
            ZeroWidth::Markers(markers_before)
        });
    };
    Shape::Newtype(Box::new(Newtype { markers_before, value, markers_after: fields.collect() }))
}

/// An enum's shape: fieldless where each variant is a unit, else with fields; `None` where it has
/// no variants, or is refused. Each refusal adds an error.
fn enum_shape(
    ident: &Ident, generics: &Generics, attrs: &[Attribute], repr: Option<&Ident>, data: DataEnum,
    errors: &mut Vec<DeriveError>,
) -> Option<Shape> {
    for variant in &data.variants {
        refuse_atom(&variant.attrs, "a variant", "a variant's tag is its discriminant", errors);
    }
    if data.variants.is_empty() {
        errors.push(DeriveError::new(
            ident.span(),
            format!("`{ident}` has no variants, so no value to store"),
        ));
        return None;
    }
    // A variant written with parentheses or braces, even of no fields, cannot be cast to its
    // discriminant where another states one, so its enum is read as one with fields.
    if data.variants.iter().any(|variant| !matches!(variant.fields, Fields::Unit)) {
        let variants: Vec<Variant> = data
            .variants
            .into_iter()
            .map(|variant| Variant {
                is_written_as_unit: matches!(variant.fields, Fields::Unit),
                fields: fields(
                    variant.fields,
                    generics,
                    Owner::Variant(ident, &variant.ident),
                    errors,
                ),
                discriminant: variant.discriminant.map(|(_, discriminant)| discriminant),
                ident: variant.ident,
            })
            .collect();
        let integer = repr_hints(attrs).into_iter().rfind(is_integer);
        if variants.iter().flat_map(|variant| &variant.fields).any(|field| field.is_pointer) {
            return Some(pointer_enum_shape(EnumWithFields { integer, variants }, repr, errors));
        }
        let shape = Shape::EnumWithFields(EnumWithFields { integer, variants });
        return repr_stated_where_generic(shape, generics, repr, "an enum with fields", errors);
    }
    if !generics.params.is_empty() {
        errors.push(
            DeriveError::new(
                generics.span(),
                "a fieldless enum derives `Atom` without parameters".to_owned(),
            )
            .note(
                None,
                "no variant holds a field, so no parameter changes what is stored".to_owned(),
            )
            .help(
                "remove them, or move them to a newtype of the enum and derive `Atom` for it"
                    .to_owned(),
            ),
        );
        return None;
    }
    let repr = enum_repr(attrs, repr, errors);
    let variants = data.variants.into_iter().map(|variant| variant.ident).collect();
    Some(Shape::Fieldless(Fieldless { repr, variants }))
}

/// The names each `#[repr]` among `attrs` gives as a hint alone, in order: `C` and `u8` of
/// `#[repr(C, u8)]`.
fn repr_hints(attrs: &[Attribute]) -> Vec<Ident> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("repr"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).ok()
        })
        .flatten()
        .filter_map(
            |hint| if let Meta::Path(path) = hint { path.get_ident().cloned() } else { None },
        )
        .collect()
}

/// The integer a fieldless enum's impl stores its discriminants as: the one the enum's `#[repr]`
/// among `attrs` names, or `stated` by `#[atom]`, which must then be the same;
/// adds an error where it is not.
fn enum_repr(
    attrs: &[Attribute], stated: Option<&Ident>, errors: &mut Vec<DeriveError>,
) -> EnumRepr {
    let named = repr_hints(attrs);
    // The last integer decides, over `C` too, as it does for rustc: `#[repr(C, u8)]` stores `u8`s.
    let integer = named.iter().rfind(|ident| is_integer(ident));
    let c = named.iter().find(|ident| *ident == "C");
    match (integer, c, stated) {
        (Some(integer), _, Some(stated)) if stated != integer => {
            let agreeing = integer.to_string();
            errors.push(disagreement(stated, integer, &format!("a `{agreeing}`"), &agreeing));
            EnumRepr::Integer(integer.clone())
        },
        (Some(integer), ..) => EnumRepr::Integer(integer.clone()),
        // C's `int` is an `i32` on every target atomix builds for.
        (None, Some(c), Some(stated)) if stated != "i32" => {
            errors.push(disagreement(stated, c, "C's `int`, an `i32`", "i32"));
            EnumRepr::C(c.clone())
        },
        (None, Some(c), _) => EnumRepr::C(c.clone()),
        (None, None, Some(stated)) => EnumRepr::Stated(stated.clone()),
        (None, None, None) => EnumRepr::Selected,
    }
}

/// The refusal of `stated`, the repr `#[atom]` states, beside `named`, the `#[repr]` hint that
/// stores each discriminant as `stored_as`, which `repr = agreeing` would agree with.
fn disagreement(stated: &Ident, named: &Ident, stored_as: &str, agreeing: &str) -> DeriveError {
    DeriveError::new(
        stated.span(),
        format!("`#[atom(repr = {stated})]` disagrees with the enum's `#[repr({named})]`"),
    )
    .note(Some(named.span()), format!("`#[repr({named})]` stores each discriminant as {stored_as}"))
    .help(format!("state `repr = {agreeing}`, or leave it out"))
}

/// Each of `fields`, those of `owner`, a struct or a variant, as an impl reads it; each refusal
/// adds an error.
///
/// A field marked `#[atom(ptr)]` is a pointer its type does not show.
fn fields(
    fields: Fields, generics: &Generics, owner: Owner<'_>, errors: &mut Vec<DeriveError>,
) -> Vec<Field> {
    let members: Vec<Member> = fields.members().collect();
    let projects = matches!(owner, Owner::Struct(_))
        && fields.iter().filter(|field| !is_marker(&field.ty)).count() > 1;
    fields
        .into_iter()
        .zip(members)
        .map(|(field, member)| {
            refuse_atom_other_than_ptr(&field, errors);
            refuse_default(&field, errors);
            refuse_place(&field.ty, &member, owner, projects, errors);
            let is_generic = names_a_parameter(field.ty.to_token_stream(), generics);
            let is_marked = field.attrs.iter().any(is_pointer_mark);
            let is_pointer = is_marked || is_pointer(&field.ty);
            let is_wide_pointer = pointee(&field.ty).is_some_and(is_unsized);
            let docs =
                field.attrs.iter().filter(|attr| attr.path().is_ident("doc")).cloned().collect();
            let deprecation = field
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("deprecated"))
                .cloned()
                .collect();
            Field {
                member,
                vis: field.vis,
                docs,
                deprecation,
                ty: field.ty,
                is_generic,
                is_pointer,
                is_wide_pointer,
            }
        })
        .collect()
}

/// What holds a field: a struct, or a variant of an enum, each named.
#[derive(Clone, Copy, Debug)]
enum Owner<'a> {
    /// The struct of this name.
    Struct(&'a Ident),
    /// The variant named second, of the enum named first.
    Variant(&'a Ident, &'a Ident),
}

/// The atomics a field may be written as with no type argument, each beside the value it holds:
/// atomix's aliases, and core's, loom's and portable-atomic's types of those names.
const CONCRETE_ATOMICS: [(&str, &str); 17] = [
    ("AtomicBool", "bool"),
    ("AtomicU8", "u8"),
    ("AtomicU16", "u16"),
    ("AtomicU32", "u32"),
    ("AtomicU64", "u64"),
    ("AtomicU128", "u128"),
    ("AtomicUsize", "usize"),
    ("AtomicI8", "i8"),
    ("AtomicI16", "i16"),
    ("AtomicI32", "i32"),
    ("AtomicI64", "i64"),
    ("AtomicI128", "i128"),
    ("AtomicIsize", "isize"),
    ("AtomicF16", "f16"),
    ("AtomicF32", "f32"),
    ("AtomicF64", "f64"),
    ("AtomicF128", "f128"),
];

/// The atomics a field may be written as with a type argument: atomix's and core's `Atomic<T>`
/// and `AtomicPtr<T>`, atomix's `AtomicField<P>`, and crossbeam's `AtomicCell<T>`.
const GENERIC_ATOMICS: [&str; 4] = ["Atomic", "AtomicField", "AtomicCell", "AtomicPtr"];

/// The cells a field may be written as, each with a type argument: core's, whose names the other
/// crates' take.
const CELLS: [&str; 6] =
    ["Cell", "RefCell", "OnceCell", "LazyCell", "UnsafeCell", "SyncUnsafeCell"];

/// The locks a field may be written as, each with a type argument: std's, whose names the other
/// crates' take.
const LOCKS: [&str; 5] = ["Mutex", "RwLock", "OnceLock", "LazyLock", "ReentrantLock"];

/// A field's type written as a place: an atomic, a cell or a lock, which shared code changes in
/// place, and which no `Copy` value holds.
#[derive(Debug)]
struct Place {
    /// The name it is written as: `AtomicU64`, `Cell`.
    name: String,
    /// Whether it is an atomic, rather than a cell or a lock.
    is_atomic: bool,
    /// The value it holds, where its name tells: `u64` of `AtomicU64`, `T` of `Cell<T>`.
    value: Option<TokenStream>,
}

/// `ty` as a place, where its path's last segment names one: an atomic alias with no type
/// argument, or a generic atomic, a cell or a lock with one.
///
/// It reads the name as written, as `is_pointer` does: an alias of one, or one renamed by
/// `use … as`, reads as no place, and a value of the user's that takes one of these names reads as
/// one, so a field that holds such a value names it otherwise, by an alias or `use … as`.
fn place(ty: &Type) -> Option<Place> {
    let segment = last_segment(ty)?;
    let name = segment.ident.to_string();
    let argument = type_argument(segment);
    if let Some((_, value)) = CONCRETE_ATOMICS.iter().find(|(atomic, _)| segment.ident == atomic) {
        let value = Ident::new(value, segment.ident.span());
        return segment.arguments.is_none().then(|| Place {
            name,
            is_atomic: true,
            value: Some(value.into_token_stream()),
        });
    }
    let argument = argument?;
    if GENERIC_ATOMICS.iter().any(|atomic| segment.ident == atomic) {
        let value = match name.as_str() {
            "AtomicPtr" => Some(quote!(*mut #argument)),
            "AtomicField" => None,
            _ => Some(argument.to_token_stream()),
        };
        return Some(Place { name, is_atomic: true, value });
    }
    let is_cell = CELLS.iter().any(|cell| segment.ident == cell);
    let is_lock = LOCKS.iter().any(|lock| segment.ident == lock);
    (is_cell || is_lock).then(|| Place {
        name,
        is_atomic: false,
        value: Some(argument.to_token_stream()),
    })
}

/// Whether a field of `data` is written as a place, which no `Copy` type holds, so that a stub
/// impl asks `Copy` only where the type has it.
fn holds_a_place(data: &Data) -> bool {
    let is_place = |field: &FieldDefinition| place(&field.ty).is_some();
    match data {
        Data::Struct(data) => data.fields.iter().any(is_place),
        Data::Enum(data) => data.variants.iter().flat_map(|variant| &variant.fields).any(is_place),
        Data::Union(_) => false,
    }
}

/// Adds an error where `ty`, the type of `owner`'s field `member`, is written as a place: a
/// derived atom is a value one atomic holds, and its fields are values. `projects` says whether
/// `owner` is a struct whose atomic lends each field's place.
fn refuse_place(
    ty: &Type, member: &Member, owner: Owner<'_>, projects: bool, errors: &mut Vec<DeriveError>,
) {
    let Some(Place { name: written, is_atomic, value }) = place(ty) else {
        return;
    };
    let name = member_shown(member);
    let (field, atom) = match owner {
        Owner::Struct(ident) => (format!("the field `{name}`"), ident),
        Owner::Variant(ident, variant) => (format!("`{variant}`'s field `{name}`"), ident),
    };
    let kind = if is_atomic { "an atomic".to_owned() } else { format!("a `{written}`") };
    let field_advice = match (value.as_ref().map(shown), member) {
        (Some(value), Member::Named(_)) => format!("make the field its value, `{name}: {value}`"),
        (Some(value), Member::Unnamed(_)) => format!("make the field its value, `{value}`"),
        (None, _) => "make the field its value".to_owned(),
    };
    let sharing_advice = if projects {
        format!(", and share `Atomic<{atom}>`, whose `fields().{name}` is the field's place")
    } else {
        format!(", and share `Atomic<{atom}>`")
    };
    let places = if is_atomic { "atomics" } else { "places" };
    let refusal = DeriveError::new(
        ty.span(),
        format!("{field} is {kind}, a place, but `{atom}` derives a value that one atomic holds"),
    )
    .note(None, "a value is `Copy`, and copying a place would split it in two".to_owned());
    // A value of the user's may take a cell's or a lock's name, as a grid's `Cell<T>`.
    let refusal = if is_atomic {
        refusal
    } else {
        refusal.note(
            None,
            format!(
                "a value of your own named `{written}` derives once the field names it otherwise, \
                 by an alias or `use … as`"
            ),
        )
    };
    errors.push(refusal.help(format!(
        "{field_advice}{sharing_advice}; or keep the {places} side by side in a struct that \
         derives no `Atom`"
    )));
}

/// Whether `attr` is `#[atom(ptr)]`, the mark of a pointer its field's type does not show.
fn is_pointer_mark(attr: &Attribute) -> bool {
    attr.path().is_ident("atom") && attr.parse_args::<Ident>().is_ok_and(|key| key == "ptr")
}

/// Adds an error for each `#[atom]` on `field` but `#[atom(ptr)]`: a field's reprs are its type's.
fn refuse_atom_other_than_ptr(field: &FieldDefinition, errors: &mut Vec<DeriveError>) {
    let refused =
        field.attrs.iter().filter(|attr| attr.path().is_ident("atom") && !is_pointer_mark(attr));
    errors.extend(refused.map(|attr| {
        DeriveError::new(attr.span(), "a field takes `#[atom(ptr)]` alone".to_owned())
            .note(
                None,
                "a field's reprs are its type's: `#[atom]`'s other keys go on the type".to_owned(),
            )
            .help(
                "mark `#[atom(ptr)]` a pointer whose type shows none: an alias, a newtype, a parameter, a pointer word or a pointer enum"
                    .to_owned(),
            )
    }));
}

/// The shape of a struct of `fields`, at least one a pointer, and `generics`, which `#[atom]`
/// states `repr` of: a pointer word of one word or two, every field but its pointers a tag.
///
/// Its pointers take two words at most, a word for each thin one and both for a wide one. Its
/// width is read from its own fields: two words for two pointers or a wide one, which
/// `repr = u128` or `i128` may state; and for one pointer, two where either states them, one where
/// `u64` or `usize` does or the struct has parameters, and otherwise as many as its tags need,
/// which a constant counts. Each refusal adds an error: a pointer past two words, tags beside a
/// trait object's pointer, one word stated of two words' pointers, and any other repr.
fn pointer_word_shape(
    fields: Vec<Field>, generics: &Generics, repr: Option<&Ident>, errors: &mut Vec<DeriveError>,
) -> Shape {
    let (pointers, tag_fields) =
        fields.into_iter().enumerate().partition::<Vec<_>, _>(|(_, field)| field.is_pointer);
    let tag_fields: Vec<Field> = tag_fields.into_iter().map(|(_, field)| field).collect();
    let mut word_count = 0_usize;
    for (_, pointer) in &pointers {
        word_count = word_count.saturating_add(if pointer.is_wide_pointer { 2 } else { 1 });
        if word_count > 2 {
            refuse_third_word(&pointers, pointer, errors);
        }
    }
    let trait_object = pointers.iter().find_map(|(_, pointer)| {
        pointee(&pointer.ty).filter(|pointee| matches!(ungrouped(pointee), Type::TraitObject(_)))
    });
    if let (Some(pointee), Some(tag)) = (trait_object, tag_fields.first()) {
        refuse_tags_beside_a_trait_object(pointee, tag, errors);
    }
    let word_repr = match (repr, word_count) {
        (Some(repr), _) if !is_one_word_repr(repr) && !is_two_words_repr(repr) => {
            refuse_repr_of_neither_width(repr, errors);
            WordRepr::One
        },
        (Some(repr), 2) if is_one_word_repr(repr) => {
            refuse_one_word_repr_of_two_words(repr, &pointers, errors);
            WordRepr::One
        },
        (Some(repr), 1) if is_two_words_repr(repr) => WordRepr::IntegerWord,
        (Some(repr), 1) if is_one_word_repr(repr) => WordRepr::One,
        (_, 1) if !generics.params.is_empty() => WordRepr::One,
        (_, 1) => WordRepr::Selected,
        _ => WordRepr::Pointers,
    };
    Shape::PointerWord(PointerWord { pointers, tag_fields, repr: word_repr })
}

/// Whether `repr`, as `#[atom]` states it, is one word, `u64` or `usize`, as a pointer word may
/// be.
fn is_one_word_repr(repr: &Ident) -> bool {
    repr == "u64" || repr == "usize"
}

/// Whether `repr`, as `#[atom]` states it, is two words, `u128` or `i128`.
fn is_two_words_repr(repr: &Ident) -> bool {
    repr == "u128" || repr == "i128"
}

/// Adds an error for `pointer`, one of `pointers`, which takes a word past the two a pointer word
/// holds.
fn refuse_third_word(pointers: &[(usize, Field)], pointer: &Field, errors: &mut Vec<DeriveError>) {
    let member = pointer.member.to_token_stream();
    let mut error = DeriveError::new(
        pointer.ty.span(),
        format!("a pointer field, `{member}`, past the two words a pointer word holds"),
    );
    for (_, before) in pointers.iter().take_while(|(_, before)| before.member != pointer.member) {
        let words = if before.is_wide_pointer { "two words" } else { "a word" };
        error = error.note(Some(before.ty.span()), format!("this pointer takes {words}"));
    }
    errors.push(
        error
            .note(
                None,
                "two pointers, or one to a slice, a `str` or a trait object, fill two words, the widest atomic".to_owned(),
            )
            .help("store the rest in an atomic of its own".to_owned()),
    );
}

/// Adds an error for `tag`, a tag field beside a pointer to `pointee`, a trait object, whose
/// alignment no constant knows.
fn refuse_tags_beside_a_trait_object(pointee: &Type, tag: &Field, errors: &mut Vec<DeriveError>) {
    let member = tag.member.to_token_stream();
    errors.push(
        DeriveError::new(
            tag.ty.span(),
            format!(
                "a tag field, `{member}`, beside a pointer to `{}`, whose low bits it cannot share",
                pointee.to_token_stream()
            ),
        )
        .note(Some(pointee.span()), "a trait object's alignment is known only at run time".to_owned())
        .help("point to a sized value that holds it, such as a `Box` of it, or store the tags in an atomic of their own".to_owned()),
    );
}

/// Adds an error for `repr`, as `#[atom]` states it, one word, beside `pointers`, which take two.
fn refuse_one_word_repr_of_two_words(
    repr: &Ident, pointers: &[(usize, Field)], errors: &mut Vec<DeriveError>,
) {
    let error = DeriveError::new(
        repr.span(),
        format!("`repr = {repr}` states one word, and its pointers take two"),
    );
    let error = pointers.iter().fold(error, |error, (_, pointer)| {
        let words = if pointer.is_wide_pointer { "two words" } else { "a word" };
        error.note(Some(pointer.ty.span()), format!("this pointer takes {words}"))
    });
    errors.push(error.help("state `repr = u128`, the two words it is, or leave it out".to_owned()));
}

/// Adds an error for `repr`, as `#[atom]` states it, neither one word nor two, of a pointer word.
fn refuse_repr_of_neither_width(repr: &Ident, errors: &mut Vec<DeriveError>) {
    errors.push(
        DeriveError::new(
            repr.span(),
            format!("`repr = {repr}` names no repr of a pointer word, which is one word or two"),
        )
        .help(
            "state `repr = u64` or `usize` for one word, `u128` for two, or leave it out"
                .to_owned(),
        ),
    );
}

/// The shape of `enum_with_fields`, an enum one of whose variants or more holds a pointer, which
/// `#[atom]` states `repr` of: a pointer enum, one word. A variant's second pointer, a pointer two
/// words wide and a repr other than a word each add an error.
fn pointer_enum_shape(
    enum_with_fields: EnumWithFields, repr: Option<&Ident>, errors: &mut Vec<DeriveError>,
) -> Shape {
    let mut pointers = enum_with_fields
        .variants
        .iter()
        .flat_map(|variant| &variant.fields)
        .filter(|field| field.is_pointer);
    if let Some(pointer) = pointers.next() {
        refuse_enum_repr_other_than_a_word(repr, &pointer.ty, errors);
    }
    for variant in &enum_with_fields.variants {
        let mut pointers = variant.fields.iter().filter(|field| field.is_pointer);
        let Some(first) = pointers.next() else { continue };
        for second in pointers {
            refuse_second_pointer_in_a_variant(&variant.ident, first, second, errors);
        }
        refuse_wide_pointer_in_a_variant(&first.ty, errors);
    }
    Shape::PointerEnum(enum_with_fields)
}

/// Adds an error for `second`, a pointer field beside `first` in the variant `variant`.
fn refuse_second_pointer_in_a_variant(
    variant: &Ident, first: &Field, second: &Field, errors: &mut Vec<DeriveError>,
) {
    errors.push(
        DeriveError::new(
            second.ty.span(),
            format!("a second pointer field in `{variant}`: a variant holds one pointer"),
        )
        .note(Some(first.ty.span()), "the first is here".to_owned())
        .note(
            None,
            "a pointer enum is one word, each variant's pointer with its tag in the low bits"
                .to_owned(),
        )
        .help(
            "store the second in an atomic of its own, or point to a struct that holds both"
                .to_owned(),
        ),
    );
}

/// Adds an error where `ty`, a variant's pointer field's type, points to a value of no size known
/// at compile time, whose pointer is two words.
fn refuse_wide_pointer_in_a_variant(ty: &Type, errors: &mut Vec<DeriveError>) {
    let Some(pointee) = pointee(ty).filter(|pointee| is_unsized(pointee)) else {
        return;
    };
    errors.push(
        DeriveError::new(
            pointee.span(),
            format!(
                "a pointer to `{}` is two words, and a pointer enum is one",
                pointee.to_token_stream()
            ),
        )
        .note(
            None,
            "a pointer to a trait object, a slice or a `str` holds its metadata beside its address"
                .to_owned(),
        )
        .help("point to a sized value that holds it, such as a `Box` of it".to_owned()),
    );
}

/// Adds an error where `repr`, the repr `#[atom]` states of a pointer enum stored as the pointer
/// `ty`, is other than `u64` or `usize`, which state the one word it is.
fn refuse_enum_repr_other_than_a_word(
    repr: Option<&Ident>, ty: &Type, errors: &mut Vec<DeriveError>,
) {
    let Some(repr) = repr.filter(|repr| !is_one_word_repr(repr)) else {
        return;
    };
    let message = if is_two_words_repr(repr) {
        format!("`repr = {repr}` states two words, and a pointer enum is one")
    } else {
        format!("`repr = {repr}` names no repr of a pointer enum, which is its pointer's, one word")
    };
    errors.push(
        DeriveError::new(repr.span(), message)
            .note(Some(ty.span()), "its repr is this pointer's".to_owned())
            .help("state `repr = u64` or `usize`, the one word it is, or leave it out".to_owned()),
    );
}

/// Adds an error for `field`'s default value: each field is read from the repr.
fn refuse_default(field: &FieldDefinition, errors: &mut Vec<DeriveError>) {
    if let Some((_, default)) = &field.default {
        errors.push(
            DeriveError::new(
                default.span(),
                "a field of a derived atom takes no default".to_owned(),
            )
            .help("remove it: each field is read from the repr".to_owned()),
        );
    }
}

/// Adds an error for each `#[atom]` among `attrs`, those of `place`, with `note` on what decides
/// its layout instead: `#[atom]` goes on the type.
fn refuse_atom(attrs: &[Attribute], place: &str, note: &str, errors: &mut Vec<DeriveError>) {
    errors.extend(attrs.iter().filter(|attr| attr.path().is_ident("atom")).map(|attr| {
        DeriveError::new(attr.span(), format!("`#[atom]` goes on the type, not on {place}"))
            .note(None, note.to_owned())
    }));
}

/// Whether `tokens`, a type, name one of `generics`' parameters: a lifetime, a type or a const.
fn names_a_parameter(tokens: TokenStream, generics: &Generics) -> bool {
    let mut after_apostrophe = false;
    tokens.into_iter().any(|token| {
        let named = match &token {
            TokenTree::Group(group) => names_a_parameter(group.stream(), generics),
            TokenTree::Ident(ident) => {
                generics.params.iter().any(|param| is_named(param, ident, after_apostrophe))
            },
            TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        };
        after_apostrophe = matches!(&token, TokenTree::Punct(punct) if punct.as_char() == '\'');
        named
    })
}

/// Whether `ident`, a lifetime's name where `is_lifetime`, names `param`.
fn is_named(param: &GenericParam, ident: &Ident, is_lifetime: bool) -> bool {
    match param {
        GenericParam::Lifetime(param) => is_lifetime && param.lifetime.ident == *ident,
        GenericParam::Type(param) => !is_lifetime && param.ident == *ident,
        GenericParam::Const(param) => !is_lifetime && param.ident == *ident,
    }
}

/// Whether `ty` is written as a `PhantomData`.
fn is_marker(ty: &Type) -> bool {
    last_segment(ty).is_some_and(|segment| segment.ident == "PhantomData")
}

/// Whether `ty` is written as a raw pointer, a `NonNull` or an `Option` of one.
fn is_pointer(ty: &Type) -> bool {
    is_raw_or_non_null(ty)
        || last_segment(ty).is_some_and(|segment| {
            segment.ident == "Option" && type_argument(segment).is_some_and(is_raw_or_non_null)
        })
}

/// Whether `ty` is written as a raw pointer or a `NonNull`.
fn is_raw_or_non_null(ty: &Type) -> bool {
    matches!(ungrouped(ty), Type::Ptr(_))
        || last_segment(ty).is_some_and(|segment| segment.ident == "NonNull")
}

/// The type `ty` is written to point to: `T` of `*mut T`, `NonNull<T>` or `Option<NonNull<T>>`;
/// `None` where it is written as no pointer, as a field marked `#[atom(ptr)]` may be.
fn pointee(ty: &Type) -> Option<&Type> {
    if let Type::Ptr(pointer) = ungrouped(ty) {
        return Some(&pointer.elem);
    }
    let segment = last_segment(ty)?;
    if segment.ident == "NonNull" {
        return type_argument(segment);
    }
    if segment.ident == "Option" {
        return type_argument(segment).and_then(pointee);
    }
    None
}

/// Whether `ty` is written as a type of no size known at compile time, whose pointer is two words:
/// a trait object, a slice or `str`.
fn is_unsized(ty: &Type) -> bool {
    let ty = ungrouped(ty);
    matches!(ty, Type::TraitObject(_) | Type::Slice(_))
        || matches!(ty, Type::Path(path) if path.qself.is_none() && path.path.is_ident("str"))
}

/// The last segment of `ty`'s path, where it is written as one: `NonNull<T>` of
/// `core::ptr::NonNull<T>`.
fn last_segment(ty: &Type) -> Option<&PathSegment> {
    if let Type::Path(path) = ungrouped(ty)
        && path.qself.is_none()
    {
        return path.path.segments.last();
    }
    None
}

/// The first type argument of `segment`: `T` of `Option<T>`.
fn type_argument(segment: &PathSegment) -> Option<&Type> {
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    arguments
        .args
        .iter()
        .find_map(|argument| if let GenericArgument::Type(ty) = argument { Some(ty) } else { None })
}

/// `ty` without the parentheses, or the invisible group a macro's `$ty` adds, around it.
fn ungrouped(ty: &Type) -> &Type {
    if let Type::Group(TypeGroup { elem, .. }) | Type::Paren(TypeParen { elem, .. }) = ty {
        ungrouped(elem)
    } else {
        ty
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Delimiter, Group, Span, TokenStream};
    use quote::{ToTokens, quote};
    use syn::Ident;

    use super::input;
    use crate::model::{
        EnumRepr, EnumWithFields, Field, Fieldless, Input, Newtype, PointerWord, Shape, WordRepr,
        ZeroWidth,
    };

    /// What `definition` reads as, which parses.
    fn read(definition: TokenStream) -> Input {
        input(definition).expect("the definition parses")
    }

    /// The newtype `definition` reads as.
    fn newtype(definition: TokenStream) -> Newtype {
        match read(definition).shape {
            Ok(Shape::Newtype(newtype)) => *newtype,
            Ok(shape) => panic!("a newtype, not {}", noun(&shape)),
            Err(errors) => panic!("a newtype, not refused: {errors:?}"),
        }
    }

    /// The members of the markers of the zero-width struct `definition` reads as, or `None` for a
    /// unit struct.
    fn markers(definition: TokenStream) -> Option<Vec<String>> {
        match read(definition).shape {
            Ok(Shape::ZeroWidth(ZeroWidth::Unit)) => None,
            Ok(Shape::ZeroWidth(ZeroWidth::Markers(markers))) => {
                Some(markers.iter().map(|field| text(&field.member)).collect())
            },
            Ok(shape) => panic!("a zero-width struct, not {}", noun(&shape)),
            Err(errors) => panic!("a zero-width struct, not refused: {errors:?}"),
        }
    }

    /// The fieldless enum `definition` reads as.
    fn fieldless(definition: TokenStream) -> Fieldless {
        match read(definition).shape {
            Ok(Shape::Fieldless(fieldless)) => fieldless,
            Ok(shape) => panic!("a fieldless enum, not {}", noun(&shape)),
            Err(errors) => panic!("a fieldless enum, not refused: {errors:?}"),
        }
    }

    /// The repr the fieldless enum `definition` stores its discriminants as.
    fn enum_repr(definition: TokenStream) -> EnumRepr {
        fieldless(definition).repr
    }

    /// The integer primitive `name`.
    fn integer(name: &str) -> Ident {
        Ident::new(name, Span::call_site())
    }

    /// The field that holds the value of `struct Value<'a, T, const N: usize>(FIELD)`.
    fn value(field: &TokenStream) -> Field {
        newtype(quote! { struct Value<'a, T, const N: usize>(#field); }).value
    }

    /// What `shape` is called where a test fails.
    const fn noun(shape: &Shape) -> &'static str {
        match shape {
            Shape::Newtype(_) => "a newtype",
            Shape::ZeroWidth(_) => "a zero-width struct",
            Shape::Packed(_) => "a struct of several fields",
            Shape::Fieldless(_) => "a fieldless enum",
            Shape::EnumWithFields(_) => "an enum with fields",
            Shape::PointerWord(_) => "a pointer word",
            Shape::PointerEnum(_) => "a pointer enum",
        }
    }

    /// What the shape `definition` reads as is called.
    fn noun_of(definition: TokenStream) -> &'static str {
        noun(&read(definition).shape.expect("the definition is read"))
    }

    /// The enum with fields `definition` reads as.
    fn enum_with_fields(definition: TokenStream) -> EnumWithFields {
        match read(definition).shape {
            Ok(Shape::EnumWithFields(enum_with_fields)) => enum_with_fields,
            Ok(shape) => panic!("an enum with fields, not {}", noun(&shape)),
            Err(errors) => panic!("an enum with fields, not refused: {errors:?}"),
        }
    }

    /// The message of each error that refuses `definition`.
    fn refusals(definition: TokenStream) -> Vec<String> {
        match read(definition).shape {
            Ok(shape) => panic!("refused, not read as {}", noun(&shape)),
            Err(errors) => errors.into_iter().map(|error| error.message).collect(),
        }
    }

    /// The text of `tokens`, as `quote!` prints them.
    fn text<T: ToTokens>(tokens: &T) -> String {
        tokens.to_token_stream().to_string()
    }

    #[test]
    fn a_struct_of_one_field_beside_markers_is_a_newtype() {
        let seq = newtype(quote! { struct Seq(u64); });
        assert_eq!(text(&seq.value.member), "0", "a tuple struct's field, by its index");
        let id = newtype(quote! {
            struct Id<K> { kind: PhantomData<K>, value: u32, tag: core::marker::PhantomData<fn() -> K> }
        });
        assert_eq!(text(&id.value.member), "value", "a named field, between its markers");
        assert_eq!(id.markers_before.len(), 1, "one marker before it");
        assert_eq!(id.markers_after.len(), 1, "and one after");
        assert_eq!(id.fields().count(), 3, "three fields in all");
    }

    #[test]
    fn a_marker_a_macro_wrote_is_a_marker() {
        let marker = Group::new(Delimiter::None, quote! { PhantomData<u8> });
        let wrapped = newtype(quote! { struct Wrapped(u64, #marker); });
        assert_eq!(wrapped.markers_after.len(), 1, "a `$ty` of `PhantomData` is a marker");
    }

    #[test]
    fn each_other_shape_is_read_from_its_fields() {
        let zero_width = "a zero-width struct";
        assert_eq!(noun_of(quote! { struct Marker; }), zero_width, "a unit struct");
        assert_eq!(noun_of(quote! { struct Kind<K>(PhantomData<K>); }), zero_width, "a marker");
        assert_eq!(noun_of(quote! { struct Pair(u32, ()); }), "a struct of several fields", "two");
        let side = quote! { enum Side { Bid, Ask = 3 } };
        assert_eq!(noun_of(side), "a fieldless enum", "unit variants alone");
        let slot = quote! { enum Slot { Empty, Full(u32) } };
        assert_eq!(noun_of(slot), "an enum with fields", "and a variant with fields");
        let mode = quote! { enum Mode { Off(), On } };
        assert_eq!(noun_of(mode), "an enum with fields", "or written with parentheses");
    }

    #[test]
    fn an_enum_with_fields_reads_each_variant_and_its_discriminant() {
        let slot = enum_with_fields(quote! {
            #[repr(C, u8)]
            enum Slot { Empty, Writing { lap: u32 } = OFFSET + 1, Done(), Moved {} }
        });
        assert_eq!(slot.integer.as_ref().map(text), Some("u8".to_owned()), "a `u8` each");
        let read: Vec<(String, bool, Vec<String>, Option<String>)> = slot
            .variants
            .iter()
            .map(|variant| {
                let members = variant.fields.iter().map(|field| text(&field.member)).collect();
                let discriminant = variant.discriminant.as_ref().map(text);
                (variant.ident.to_string(), variant.is_written_as_unit, members, discriminant)
            })
            .collect();
        let expected = [
            ("Empty".to_owned(), true, vec![], None),
            ("Writing".to_owned(), false, vec!["lap".to_owned()], Some("OFFSET + 1".to_owned())),
            ("Done".to_owned(), false, vec![], None),
            ("Moved".to_owned(), false, vec![], None),
        ];
        assert_eq!(read, expected, "in declaration order");
        let isize = enum_with_fields(quote! { #[repr(C)] enum Slot { Empty, Full(u32) } });
        assert!(isize.integer.is_none(), "`isize`, where no integer is named");
        let parentheses = enum_with_fields(quote! { enum Mode { Off(), On } });
        assert!(parentheses.variants.iter().all(|variant| variant.fields.is_empty()), "no fields");
    }

    #[test]
    fn an_enum_with_fields_and_parameters_states_its_repr() {
        assert_eq!(
            refusals(quote! { enum Lock<O> { Free, Owned(O) } }),
            ["an enum with fields derives `Atom` with parameters only where it states its repr"],
            "with none stated"
        );
        let lock = enum_with_fields(quote! { #[atom(repr = u64)] enum Lock<O> { Free, Owned(O) } });
        assert!(
            lock.variants.iter().any(|variant| variant.fields.iter().any(|field| field.is_generic)),
            "a generic field"
        );
    }

    #[test]
    fn a_zero_width_struct_is_a_unit_struct_or_its_markers() {
        assert_eq!(markers(quote! { struct Marker; }), None, "a unit struct");
        assert_eq!(markers(quote! { struct Marker {} }), Some(vec![]), "is not one with braces");
        assert_eq!(
            markers(quote! { struct Kind<K>(PhantomData<K>, PhantomData<fn() -> K>); }),
            Some(vec!["0".to_owned(), "1".to_owned()]),
            "and markers alone are each read, in order"
        );
    }

    /// The members of the fields of the packed struct `definition` reads as.
    fn packed(definition: TokenStream) -> Vec<String> {
        match read(definition).shape {
            Ok(Shape::Packed(fields)) => fields.iter().map(|field| text(&field.member)).collect(),
            Ok(shape) => panic!("a struct of several fields, not {}", noun(&shape)),
            Err(errors) => panic!("a struct of several fields, not refused: {errors:?}"),
        }
    }

    #[test]
    fn a_struct_of_several_fields_reads_its_markers_too_in_order() {
        assert_eq!(
            packed(quote! { struct Quote { quantity: u32, kind: PhantomData<Venue>, live: bool } }),
            ["quantity", "kind", "live"],
            "by name"
        );
        assert_eq!(packed(quote! { struct Pair(u32, ()); }), ["0", "1"], "or by position");
    }

    #[test]
    fn a_struct_of_several_fields_with_parameters_states_its_repr() {
        assert_eq!(
            refusals(quote! { struct Pair<A, B> { first: A, second: B } }),
            [
                "a struct of several fields derives `Atom` with parameters only where it states its repr"
            ],
            "with none stated"
        );
        assert_eq!(
            packed(quote! { #[atom(repr = u64)] struct Pair<A, B> { first: A, second: B } }),
            ["first", "second"],
            "but read where it does"
        );
    }

    #[test]
    fn a_fieldless_enum_reads_each_variant_whatever_its_discriminant() {
        let level = fieldless(quote! { enum Level { Low = 1 << 3, High = OFFSET + 2, Next } });
        let names: Vec<String> = level.variants.iter().map(ToString::to_string).collect();
        assert_eq!(names, ["Low", "High", "Next"], "in declaration order");
    }

    #[test]
    fn a_fieldless_enum_stores_the_integer_its_repr_names() {
        assert_eq!(
            enum_repr(quote! { #[repr(u8)] enum Side { Bid } }),
            EnumRepr::Integer(integer("u8")),
            "an integer"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(C)] enum Side { Bid } }),
            EnumRepr::C(integer("C")),
            "C's `int`"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(C, u16)] enum Side { Bid } }),
            EnumRepr::Integer(integer("u16")),
            "an integer over C's"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(u8, u16)] enum Side { Bid } }),
            EnumRepr::Integer(integer("u16")),
            "the last of two, as rustc reads them"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(align(8))] #[repr(i32)] enum Side { Bid } }),
            EnumRepr::Integer(integer("i32")),
            "beside other hints, in any attribute"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(align(8))] enum Side { Bid } }),
            EnumRepr::Selected,
            "and none, selected, where no hint names one"
        );
        assert_eq!(enum_repr(quote! { enum Side { Bid } }), EnumRepr::Selected, "or no `#[repr]`");
    }

    #[test]
    fn a_repr_stated_beside_an_enums_must_name_its_integer() {
        assert_eq!(
            enum_repr(quote! { #[atom(repr = u32)] enum Side { Bid } }),
            EnumRepr::Stated(integer("u32")),
            "stated alone"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(u8)] #[atom(repr = u8)] enum Side { Bid } }),
            EnumRepr::Integer(integer("u8")),
            "agreeing"
        );
        assert_eq!(
            enum_repr(quote! { #[repr(C)] #[atom(repr = i32)] enum Side { Bid } }),
            EnumRepr::C(integer("C")),
            "with C's `int`, an `i32`"
        );
        assert_eq!(
            refusals(quote! { #[repr(u8)] #[atom(repr = u16)] enum Side { Bid } }),
            ["`#[atom(repr = u16)]` disagrees with the enum's `#[repr(u8)]`"],
            "but not disagreeing"
        );
        assert_eq!(
            refusals(quote! { #[repr(C)] #[atom(repr = u32)] enum Side { Bid } }),
            ["`#[atom(repr = u32)]` disagrees with the enum's `#[repr(C)]`"],
            "with C's either"
        );
    }

    #[test]
    fn a_fieldless_enum_with_parameters_is_refused() {
        assert_eq!(
            refusals(quote! { enum Level<const N: usize> { Low } }),
            ["a fieldless enum derives `Atom` without parameters"],
            "a constant no variant holds"
        );
    }

    #[test]
    fn a_field_naming_a_parameter_is_generic() {
        assert!(value(&quote! { T }).is_generic, "a type parameter");
        assert!(value(&quote! { Option<T> }).is_generic, "inside a type");
        assert!(value(&quote! { Ranged<N> }).is_generic, "a const parameter");
        assert!(value(&quote! { Borrowed<'a> }).is_generic, "a lifetime");
        assert!(!value(&quote! { a::Seq }).is_generic, "but not a path named as a lifetime is");
        assert!(!value(&quote! { u64 }).is_generic, "nor a concrete type");
    }

    #[test]
    fn a_raw_pointer_a_non_null_and_an_option_of_one_are_pointers() {
        for pointer in [
            quote! { *mut u8 },
            quote! { *const T },
            quote! { NonNull<u8> },
            quote! { core::ptr::NonNull<u8> },
            quote! { Option<NonNull<u8>> },
            quote! { Option<*mut u8> },
        ] {
            assert!(value(&pointer).is_pointer, "`{pointer}` is a pointer");
        }
        for other in [quote! { u64 }, quote! { Option<u64> }, quote! { Option<Option<*mut u8>> }] {
            assert!(!value(&other).is_pointer, "`{other}` is not");
        }
    }

    #[test]
    fn atomix_is_named_absolutely_unless_stated() {
        let default = read(quote! { struct Seq(u64); });
        assert_eq!(text(&default.implementor.atomix), ":: atomix", "by default");
        let renamed = read(quote! { #[atom(crate = renamed)] struct Seq(u64); });
        assert_eq!(text(&renamed.implementor.atomix), "renamed", "or as stated");
    }

    #[test]
    fn the_repr_stated_is_read_from_any_atom_attribute() {
        let pinned = read(quote! { #[atom(crate = renamed)] #[atom(repr = u16)] struct Seq(u16); });
        assert_eq!(pinned.implementor.repr.as_ref().map(text), Some("u16".to_owned()), "the repr");
        assert_eq!(text(&pinned.implementor.atomix), "renamed", "beside the crate");
        assert!(read(quote! { struct Seq(u16); }).implementor.repr.is_none(), "and none unstated");
    }

    #[test]
    fn a_key_that_does_not_read_leaves_the_rest_to_read() {
        let definition = quote! { #[atom(width = 8, crate = renamed)] struct Seq(u64); };
        let atomix = read(definition.clone()).implementor.atomix;
        assert_eq!(text(&atomix), "renamed", "for a stub to name");
        assert_eq!(
            refusals(definition),
            ["`width` is not a key of `#[atom]`"],
            "while the type is refused for the key alone"
        );
    }

    #[test]
    fn a_key_of_atom_is_refused_where_it_does_not_read() {
        assert_eq!(
            refusals(quote! { #[atom(width = 8)] struct Seq(u64); }),
            ["`width` is not a key of `#[atom]`"],
            "an unknown key"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr = u64, repr = u32)] struct Seq(u64); }),
            ["`repr` is stated twice"],
            "a key stated twice"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr)] struct Seq(u64); }),
            ["`repr` takes a value"],
            "a key without a value"
        );
        assert_eq!(
            refusals(quote! { #[atom(crate = "atomix")] struct Seq(u64); }),
            ["`crate` takes a path"],
            "a crate named by a string"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr = f64)] struct Price(f64); }),
            ["`f64` is not an integer primitive"],
            "a repr that is no integer"
        );
    }

    #[test]
    fn atom_on_a_field_or_a_variant_is_refused() {
        assert_eq!(
            refusals(quote! { struct Seq(#[atom(repr = u8)] u64); }),
            ["a field takes `#[atom(ptr)]` alone"],
            "on a field, but to mark a pointer"
        );
        assert_eq!(
            refusals(quote! { enum Side { #[atom(repr = u8)] Bid, Ask } }),
            ["`#[atom]` goes on the type, not on a variant"],
            "on a variant"
        );
        assert_eq!(
            refusals(quote! { enum Slot { Empty, Full(#[atom] u32) } }),
            ["a field takes `#[atom(ptr)]` alone"],
            "or on a variant's field"
        );
    }

    #[test]
    fn a_field_written_as_a_place_is_refused() {
        let message = |field: &str, kind: &str, atom: &str| {
            format!(
                "{field} is {kind}, a place, but `{atom}` derives a value that one atomic holds"
            )
        };
        assert_eq!(
            refusals(quote! { struct Pair { seq: core::sync::atomic::AtomicU64, live: bool } }),
            [message("the field `seq`", "an atomic", "Pair")],
            "core's alias, by its path's last segment"
        );
        assert_eq!(
            refusals(quote! { struct Seq(Atomic<u32>); }),
            [message("the field `0`", "an atomic", "Seq")],
            "the generic atomic, in a newtype"
        );
        assert_eq!(
            refusals(quote! { enum Slot { Empty, Full { lap: Cell<u32> } } }),
            [message("`Full`'s field `lap`", "a `Cell`", "Slot")],
            "a cell, in a variant"
        );
        for value in [
            quote! { struct Tile { number: AtomicNumber, cell: Cell } },
            quote! { struct Tile { cell: grid::Cell, count: AtomicU64<u8> } },
        ] {
            assert!(
                read(value).shape.is_ok(),
                "a value named near a place, or with arguments no place takes"
            );
        }
    }

    #[test]
    fn a_field_default_is_refused_in_a_struct_and_in_a_variant() {
        let refusal = ["a field of a derived atom takes no default"];
        assert_eq!(refusals(quote! { struct Seq { value: u64 = 1 } }), refusal, "in a struct");
        assert_eq!(refusals(quote! { enum Slot { Full { lap: u32 = 0 } } }), refusal, "a variant");
    }

    #[test]
    fn a_union_and_an_enum_of_no_variants_are_refused() {
        assert_eq!(
            refusals(quote! { union Bits { word: u32, bytes: [u8; 4] } }),
            ["`Bits` is a union, so no repr could say which field it holds"],
            "a union"
        );
        assert_eq!(
            refusals(quote! { enum Never {} }),
            ["`Never` has no variants, so no value to store"],
            "an empty enum"
        );
    }

    #[test]
    fn every_refusal_of_a_type_is_reported() {
        assert_eq!(
            refusals(quote! { #[atom(width = 8)] struct Seq(#[atom] u64); }),
            ["`width` is not a key of `#[atom]`", "a field takes `#[atom(ptr)]` alone"],
            "the key and the field"
        );
    }

    /// The pointer word `definition` reads as.
    fn pointer_word(definition: TokenStream) -> PointerWord {
        match read(definition).shape {
            Ok(Shape::PointerWord(word)) => word,
            Ok(shape) => panic!("a pointer word, not {}", noun(&shape)),
            Err(errors) => panic!("a pointer word, not refused: {errors:?}"),
        }
    }

    /// The members of `word`'s pointers, each beside its index, and of its tags.
    fn members(word: &PointerWord) -> (Vec<(usize, String)>, Vec<String>) {
        let pointers =
            word.pointers.iter().map(|(index, pointer)| (*index, text(&pointer.member))).collect();
        (pointers, word.tag_fields.iter().map(|tag| text(&tag.member)).collect())
    }

    #[test]
    fn a_struct_of_one_pointer_beside_tags_is_a_pointer_word() {
        let head = pointer_word(
            quote! { struct Head { version: u2, top: Option<NonNull<Node>>, marked: bool } },
        );
        let expected =
            (vec![(1, "top".to_owned())], vec!["version".to_owned(), "marked".to_owned()]);
        assert_eq!(members(&head), expected, "the pointer, beside its tags, in order");
        let fields: Vec<String> = head.fields().map(|field| text(&field.member)).collect();
        assert_eq!(fields, ["version", "top", "marked"], "every field, in declaration order");
        assert_eq!(head.repr, WordRepr::Selected, "as wide as its tags need");
        let link =
            pointer_word(quote! { struct Link { #[atom(ptr)] next: NodePointer, deleted: bool } });
        assert_eq!(members(&link).0, [(0, "next".to_owned())], "or a field marked one");
        let tagged = pointer_word(quote! { struct Tagged<T> { pointer: NonNull<T>, tag: u2 } });
        let (_, pointer) = &tagged.pointers[0];
        assert!(pointer.is_generic, "and one with parameters, with no repr stated");
        assert_eq!(tagged.repr, WordRepr::One, "one word, which no constant of `T` could widen");
        for repr in [quote!(u64), quote!(usize)] {
            let pinned = quote! { #[atom(repr = #repr)] struct Link { next: *mut u8, bit: bool } };
            assert_eq!(pointer_word(pinned).repr, WordRepr::One, "`repr = {repr}` pins one word");
        }
        for repr in [quote!(u128), quote!(i128)] {
            let pinned =
                quote! { #[atom(repr = #repr)] struct Link<T> { next: *mut T, bit: bool } };
            let word_repr = pointer_word(pinned).repr;
            assert_eq!(
                word_repr,
                WordRepr::IntegerWord,
                "`repr = {repr}` states two, its tags in an integer word"
            );
        }
    }

    #[test]
    fn two_pointers_or_one_two_words_wide_take_two_words() {
        let pair = pointer_word(quote! { struct Pair { a: *mut u8, b: NonNull<u8>, c: bool } });
        let expected = (vec![(0, "a".to_owned()), (1, "b".to_owned())], vec!["c".to_owned()]);
        assert_eq!(members(&pair), expected, "both pointers, beside the tag");
        assert_eq!(pair.repr, WordRepr::Pointers, "in two words");
        let stated = quote! { #[atom(repr = u128)] struct Pair { a: *mut u8, b: NonNull<u8> } };
        assert_eq!(pointer_word(stated).repr, WordRepr::Pointers, "as `repr = u128` states");
        for pointer in
            [quote! { NonNull<dyn Fn()> }, quote! { *const [u8] }, quote! { Option<NonNull<str>> }]
        {
            assert!(value(&pointer).is_wide_pointer, "`{pointer}` is two words wide");
        }
        assert!(
            !value(&quote! { NonNull<[u8; 4]> }).is_wide_pointer,
            "and an array's pointer, one"
        );
        let chunk = pointer_word(quote! { struct Chunk { words: NonNull<[u64]>, sealed: bool } });
        assert_eq!(chunk.repr, WordRepr::Pointers, "a slice's, with a tag beside");
    }

    #[test]
    fn a_third_word_tags_beside_a_trait_object_and_a_repr_of_neither_width_are_refused() {
        assert_eq!(
            refusals(quote! { struct Three { a: *mut u8, b: *mut u8, c: NonNull<u8> } }),
            ["a pointer field, `c`, past the two words a pointer word holds"],
            "a third pointer"
        );
        assert_eq!(
            refusals(quote! { struct Wide { a: NonNull<[u8]>, b: *mut u8 } }),
            ["a pointer field, `b`, past the two words a pointer word holds"],
            "or one beside a wide one"
        );
        assert_eq!(
            refusals(quote! { struct Callback { call: NonNull<dyn Fn()>, armed: bool } }),
            [
                "a tag field, `armed`, beside a pointer to `dyn Fn ()`, whose low bits it cannot share"
            ],
            "tags beside a trait object"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr = u64)] struct Pair { a: *mut u8, b: *mut u8 } }),
            ["`repr = u64` states one word, and its pointers take two"],
            "one word of two words' pointers"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr = u32)] struct Link { next: *mut u8, c: bool } }),
            ["`repr = u32` names no repr of a pointer word, which is one word or two"],
            "less than a word"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr = isize)] struct Link { next: *mut u8, c: bool } }),
            ["`repr = isize` names no repr of a pointer word, which is one word or two"],
            "or a signed one"
        );
    }

    #[test]
    fn a_pointer_in_a_variant_makes_a_pointer_enum() {
        let noun = "a pointer enum";
        assert_eq!(noun_of(quote! { enum Next { End, Node(NonNull<u8>) } }), noun, "written one");
        let marked = quote! { enum Next { End, Node(#[atom(ptr)] Inner) } };
        assert_eq!(noun_of(marked), noun, "or marked one");
        let pinned = quote! { #[atom(repr = usize)] enum Next { End, Node(NonNull<u8>) } };
        assert_eq!(noun_of(pinned), noun, "and pinned to one word");
        assert_eq!(
            refusals(quote! { enum Pair { Both(NonNull<u8>, *mut u8) } }),
            ["a second pointer field in `Both`: a variant holds one pointer"],
            "two in a variant"
        );
        assert_eq!(
            refusals(quote! { #[atom(repr = i128)] enum Next { End, Node(NonNull<u8>) } }),
            ["`repr = i128` states two words, and a pointer enum is one"],
            "two words"
        );
        assert_eq!(
            refusals(quote! { enum Shape { Empty, Points(NonNull<[u8]>) } }),
            ["a pointer to `[u8]` is two words, and a pointer enum is one"],
            "and a slice"
        );
    }

    #[test]
    fn a_definition_that_does_not_parse_gives_syns_errors() {
        let errors = input(quote! { fn seq() {} }).err().expect("a function is no type");
        assert_eq!(errors.len(), 1, "one error");
    }
}
