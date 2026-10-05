//! Reads a derive's input into the model, refusing what no impl could be written for.

use core::iter;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Error as SyntaxError, Expr, ExprPath,
    Field as FieldDefinition, Fields, GenericArgument, GenericParam, Generics, Ident, Member, Meta,
    Path, PathArguments, PathSegment, Token, Type, TypeGroup, TypeParen, parse2,
};

use crate::errors::DeriveError;
use crate::model::{
    EnumRepr, EnumWithFields, Field, Fieldless, Implementor, Input, Newtype, Shape, Variant,
    ZeroWidth,
};

/// The integer primitives `repr = …` may state.
const INTEGERS: [&str; 12] =
    ["u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize"];

/// Reads `input`, a type's definition, or gives syn's errors where it does not parse.
pub(crate) fn input(input: TokenStream) -> Result<Input, Vec<DeriveError>> {
    let DeriveInput { attrs, ident, generics, data, .. } =
        parse2::<DeriveInput>(input).map_err(|error: SyntaxError| {
            error.into_iter().map(DeriveError::from).collect::<Vec<_>>()
        })?;
    let mut errors = Vec::new();
    let (atomiks, repr) = options(&attrs, &mut errors);
    let shape = shape(&ident, &generics, &attrs, repr.as_ref(), data, &mut errors);
    let implementor = Implementor { ident, generics, atomiks, repr };
    let shape = match shape {
        Some(shape) if errors.is_empty() => Ok(shape),
        _ => Err(errors),
    };
    Ok(Input { implementor, shape })
}

/// What `#[atom(…)]` states: the path to atomiks, `::atomiks` where none is, and the repr.
///
/// Each key that does not read adds an error and leaves the rest to read, so a stub impl still
/// names atomiks by the path stated.
fn options(attrs: &[Attribute], errors: &mut Vec<DeriveError>) -> (Path, Option<Ident>) {
    let mut atomiks = None;
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
                set_once(&mut atomiks, key, crate_path(key), errors);
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
    (atomiks.unwrap_or_else(default_atomiks), repr)
}

/// `::atomiks`, the path to atomiks where `crate = …` states none.
fn default_atomiks() -> Path {
    let mut path = Path::from(Ident::new("atomiks", Span::call_site()));
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
        .help("name atomiks as the crate reaches it, such as `crate = ::atomiks`".to_owned()))
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
            let shape = struct_shape(fields(data.fields, generics, errors), is_unit);
            if matches!(shape, Shape::Packed(_)) {
                return repr_stated_where_generic(
                    shape,
                    generics,
                    repr,
                    "a struct of several fields",
                    errors,
                );
            }
            Some(shape)
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
    Shape::Newtype(Newtype { markers_before, value, markers_after: fields.collect() })
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
        let variants = data
            .variants
            .into_iter()
            .map(|variant| Variant {
                is_written_as_unit: matches!(variant.fields, Fields::Unit),
                fields: fields(variant.fields, generics, errors),
                discriminant: variant.discriminant.map(|(_, discriminant)| discriminant),
                ident: variant.ident,
            })
            .collect();
        let integer = repr_hints(attrs).into_iter().rfind(is_integer);
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
        // C's `int` is an `i32` on every target atomiks builds for.
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

/// Each of `fields`, as an impl reads it; each refusal adds an error.
fn fields(fields: Fields, generics: &Generics, errors: &mut Vec<DeriveError>) -> Vec<Field> {
    let members: Vec<Member> = fields.members().collect();
    fields
        .into_iter()
        .zip(members)
        .map(|(field, member)| {
            refuse_atom_or_default(&field, errors);
            let is_generic = names_a_parameter(field.ty.to_token_stream(), generics);
            let is_pointer = is_pointer(&field.ty);
            Field { member, ty: field.ty, is_generic, is_pointer }
        })
        .collect()
}

/// Adds an error for each `#[atom]` on `field`, and for its default value: each field is read from
/// the repr.
fn refuse_atom_or_default(field: &FieldDefinition, errors: &mut Vec<DeriveError>) {
    refuse_atom(&field.attrs, "a field", "a field's reprs are its type's", errors);
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
        EnumRepr, EnumWithFields, Field, Fieldless, Input, Newtype, Shape, ZeroWidth,
    };

    /// What `definition` reads as, which parses.
    fn read(definition: TokenStream) -> Input {
        input(definition).expect("the definition parses")
    }

    /// The newtype `definition` reads as.
    fn newtype(definition: TokenStream) -> Newtype {
        match read(definition).shape {
            Ok(Shape::Newtype(newtype)) => newtype,
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
    fn atomiks_is_named_absolutely_unless_stated() {
        let default = read(quote! { struct Seq(u64); });
        assert_eq!(text(&default.implementor.atomiks), ":: atomiks", "by default");
        let renamed = read(quote! { #[atom(crate = renamed)] struct Seq(u64); });
        assert_eq!(text(&renamed.implementor.atomiks), "renamed", "or as stated");
    }

    #[test]
    fn the_repr_stated_is_read_from_any_atom_attribute() {
        let pinned = read(quote! { #[atom(crate = renamed)] #[atom(repr = u16)] struct Seq(u16); });
        assert_eq!(pinned.implementor.repr.as_ref().map(text), Some("u16".to_owned()), "the repr");
        assert_eq!(text(&pinned.implementor.atomiks), "renamed", "beside the crate");
        assert!(read(quote! { struct Seq(u16); }).implementor.repr.is_none(), "and none unstated");
    }

    #[test]
    fn a_key_that_does_not_read_leaves_the_rest_to_read() {
        let definition = quote! { #[atom(width = 8, crate = renamed)] struct Seq(u64); };
        let atomiks = read(definition.clone()).implementor.atomiks;
        assert_eq!(text(&atomiks), "renamed", "for a stub to name");
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
            refusals(quote! { #[atom(crate = "atomiks")] struct Seq(u64); }),
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
            ["`#[atom]` goes on the type, not on a field"],
            "on a field"
        );
        assert_eq!(
            refusals(quote! { enum Side { #[atom(repr = u8)] Bid, Ask } }),
            ["`#[atom]` goes on the type, not on a variant"],
            "on a variant"
        );
        assert_eq!(
            refusals(quote! { enum Slot { Empty, Full(#[atom] u32) } }),
            ["`#[atom]` goes on the type, not on a field"],
            "or on a variant's field"
        );
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
            ["`width` is not a key of `#[atom]`", "`#[atom]` goes on the type, not on a field"],
            "the key and the field"
        );
    }

    #[test]
    fn a_definition_that_does_not_parse_gives_syns_errors() {
        let errors = input(quote! { fn seq() {} }).err().expect("a function is no type");
        assert_eq!(errors.len(), 1, "one error");
    }
}
