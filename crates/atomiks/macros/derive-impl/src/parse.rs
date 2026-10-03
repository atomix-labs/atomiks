//! Reads a derive's input into the model, refusing what no impl could be written for.

use core::iter;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error as SyntaxError, Expr, ExprPath, Field as FieldDefinition,
    Fields, GenericArgument, GenericParam, Generics, Ident, Member, Meta, Path, PathArguments,
    PathSegment, Token, Type, TypeGroup, TypeParen, parse2,
};

use crate::errors::DeriveError;
use crate::model::{Field, Implementor, Input, Newtype, Shape};

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
    let shape = shape(&ident, &generics, data, &mut errors);
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
        && INTEGERS.iter().any(|integer| ident == integer)
    {
        return Ok(ident.clone());
    }
    Err(DeriveError::new(
        value.span(),
        format!("`{}` is not an integer primitive", value.to_token_stream()),
    )
    .help("state one, such as `repr = u64`".to_owned()))
}

/// The shape of `data`, that of the type `ident` of `generics`, or `None` for a union or an enum of
/// no variants, which have none an impl could encode. Each refusal adds an error.
fn shape(
    ident: &Ident, generics: &Generics, data: Data, errors: &mut Vec<DeriveError>,
) -> Option<Shape> {
    match data {
        Data::Struct(data) => Some(struct_shape(fields(data.fields, generics, errors))),
        Data::Enum(data) => {
            for variant in &data.variants {
                refuse_atom(
                    &variant.attrs,
                    "a variant",
                    "a variant's tag is its discriminant",
                    errors,
                );
                variant.fields.iter().for_each(|field| refuse_atom_or_default(field, errors));
            }
            if data.variants.is_empty() {
                errors.push(DeriveError::new(
                    ident.span(),
                    format!("`{ident}` has no variants, so no value to store"),
                ));
                return None;
            }
            Some(if data.variants.iter().all(|variant| variant.fields.is_empty()) {
                Shape::Fieldless
            } else {
                Shape::EnumWithFields
            })
        },
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

/// A struct's shape, by how many of its fields are not `PhantomData` markers: a newtype where one
/// is, zero-width where none is, else packed.
fn struct_shape(fields: Vec<Field>) -> Shape {
    let mut fields = fields.into_iter().peekable();
    let markers_before = iter::from_fn(|| fields.next_if(|field| is_marker(&field.ty))).collect();
    let Some(value) = fields.next() else {
        return Shape::ZeroWidth;
    };
    let markers_after: Vec<Field> = fields.collect();
    if markers_after.iter().all(|field| is_marker(&field.ty)) {
        Shape::Newtype(Newtype { markers_before, value, markers_after })
    } else {
        Shape::Packed
    }
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
    use proc_macro2::{Delimiter, Group, TokenStream};
    use quote::{ToTokens, quote};

    use super::input;
    use crate::model::{Field, Input, Newtype, Shape};

    /// What `definition` reads as, which parses.
    fn read(definition: TokenStream) -> Input {
        input(definition).expect("the definition parses")
    }

    /// The newtype `definition` reads as.
    fn newtype(definition: TokenStream) -> Newtype {
        match read(definition).shape {
            Ok(Shape::Newtype(newtype)) => newtype,
            Ok(shape) => panic!("a newtype, not {}", shape.noun()),
            Err(errors) => panic!("a newtype, not refused: {errors:?}"),
        }
    }

    /// The field that holds the value of `struct Value<'a, T, const N: usize>(FIELD)`.
    fn value(field: &TokenStream) -> Field {
        newtype(quote! { struct Value<'a, T, const N: usize>(#field); }).value
    }

    /// The noun of the shape `definition` reads as.
    fn noun(definition: TokenStream) -> &'static str {
        read(definition).shape.expect("the definition is read").noun()
    }

    /// The message of each error that refuses `definition`.
    fn refusals(definition: TokenStream) -> Vec<String> {
        match read(definition).shape {
            Ok(shape) => panic!("refused, not read as {}", shape.noun()),
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
    fn each_other_shape_is_named() {
        let zero_width = "a struct of no field but markers";
        assert_eq!(noun(quote! { struct Marker; }), zero_width, "a unit struct");
        assert_eq!(noun(quote! { struct Kind<K>(PhantomData<K>); }), zero_width, "a marker alone");
        assert_eq!(noun(quote! { struct Pair(u32, ()); }), "a struct of several fields", "two");
        assert_eq!(
            noun(quote! { enum Side { Bid, Ask = 3 } }),
            "a fieldless enum",
            "unit variants"
        );
        assert_eq!(
            noun(quote! { enum Slot { Empty, Full(u32) } }),
            "an enum with fields",
            "payload"
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
