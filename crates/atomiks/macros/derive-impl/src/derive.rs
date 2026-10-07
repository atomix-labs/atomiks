//! The derives: each reads its input, and writes its impl or refuses it.

use proc_macro2::{Span, TokenStream};

use crate::errors::DeriveError;
use crate::model::{Implementor, Input, Shape};
use crate::{code, parse};

/// What a derive writes: its code, and the errors it reports beside it.
///
/// Where it refuses its input, the code is a stub that keeps the errors from raising others, or
/// nothing.
#[derive(Debug)]
pub struct Expansion {
    /// The impl, or its stub.
    pub code: TokenStream,
    /// Why the input is refused, if it is.
    pub errors: Vec<DeriveError>,
}

impl Expansion {
    /// The expansion to `code`, refusing nothing.
    const fn written(code: TokenStream) -> Self {
        Self { code, errors: Vec::new() }
    }

    /// The expansion refusing the input with `errors`, writing `stub` in place of its impl.
    const fn refused(errors: Vec<DeriveError>, stub: TokenStream) -> Self {
        Self { code: stub, errors }
    }
}

/// A capability a newtype takes from its field: the read-modify-writes that mean something on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    /// `AtomAdd`: add and subtract.
    Add,
    /// `AtomOrd`: maximum and minimum.
    Ord,
    /// `AtomBitwise`: and, or, xor and not.
    Bitwise,
}

impl Capability {
    /// The trait's name, `AtomAdd`.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Add => "AtomAdd",
            Self::Ord => "AtomOrd",
            Self::Bitwise => "AtomBitwise",
        }
    }

    /// The name of the function in `__private` that compiles only where a type has it,
    /// `assert_atom_add`.
    pub(crate) const fn assertion(self) -> &'static str {
        match self {
            Self::Add => "assert_atom_add",
            Self::Ord => "assert_atom_ord",
            Self::Bitwise => "assert_atom_bitwise",
        }
    }
}

/// Derives `Atom` for `input`, a type's definition, naming its locals at `def_site`: the proc
/// macro's definition site, where no name of the user's resolves in their place.
#[must_use]
pub fn expand_atom(input: TokenStream, def_site: Span) -> Expansion {
    let Input { implementor, shape } = match parse::input(input) {
        Ok(input) => input,
        Err(errors) => return Expansion::refused(errors, TokenStream::new()),
    };
    match shape {
        Ok(Shape::Newtype(newtype)) => {
            Expansion::written(code::newtype(&implementor, &newtype, def_site))
        },
        Ok(Shape::ZeroWidth(zero_width)) => {
            Expansion::written(code::zero_width(&implementor, &zero_width, def_site))
        },
        Ok(Shape::Packed(fields)) => {
            Expansion::written(code::packed(&implementor, &fields, def_site))
        },
        Ok(Shape::PointerWord(word)) => {
            Expansion::written(code::pointer_word(&implementor, &word, def_site))
        },
        Ok(Shape::Fieldless(fieldless)) => {
            Expansion::written(code::fieldless(&implementor, &fieldless, def_site))
        },
        Ok(Shape::EnumWithFields(enum_with_fields)) => {
            Expansion::written(code::enum_with_fields(&implementor, &enum_with_fields, def_site))
        },
        Ok(Shape::PointerEnum(enumeration)) => {
            Expansion::written(code::pointer_enum(&implementor, &enumeration, def_site))
        },
        Err(errors) => Expansion::refused(errors, code::stub(&implementor)),
    }
}

/// Derives `capability` for `input`, a type's definition, which must be a newtype.
#[must_use]
pub fn expand_capability(input: TokenStream, capability: Capability) -> Expansion {
    match parse::input(input) {
        Ok(Input { implementor, shape: Ok(Shape::Newtype(newtype)) }) => {
            Expansion::written(code::capability(&implementor, &newtype, capability))
        },
        Ok(Input { implementor, shape: Ok(_) }) => {
            Expansion::refused(vec![not_a_newtype(&implementor, capability)], TokenStream::new())
        },
        Ok(Input { shape: Err(errors), .. }) | Err(errors) => {
            Expansion::refused(errors, TokenStream::new())
        },
    }
}

/// The refusal of `capability` for a type that is not a newtype.
fn not_a_newtype(implementor: &Implementor, capability: Capability) -> DeriveError {
    let name = capability.name();
    DeriveError::new(implementor.ident.span(), format!("`{name}` derives only for a newtype"))
        .note(
            None,
            format!(
                "a newtype, a struct of one field beside any `PhantomData` markers, takes `{name}` from that field"
            ),
        )
        .help("to change the value in a compare-exchange loop, call `update`".to_owned())
}

#[cfg(test)]
mod tests {
    use proc_macro2::{Span, TokenStream};
    use quote::quote;

    use super::{Capability, Expansion, expand_atom, expand_capability};

    /// What `Atom`'s derive writes for `definition`, its locals named at the call site.
    fn derive_atom(definition: TokenStream) -> Expansion {
        expand_atom(definition, Span::call_site())
    }

    /// The code of `expansion`, which refuses nothing.
    fn written(expansion: &Expansion) -> String {
        assert!(expansion.errors.is_empty(), "nothing refused: {:?}", expansion.errors);
        expansion.code.to_string()
    }

    /// The message of each error of `expansion`, which refuses its input, and its code.
    fn refused(expansion: Expansion) -> (Vec<String>, String) {
        let messages = expansion.errors.into_iter().map(|error| error.message).collect();
        (messages, expansion.code.to_string())
    }

    #[test]
    fn a_concrete_newtype_converts_through_the_codecs() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Seq>();
            #[automatically_derived]
            const unsafe impl ::atomiks::Atom for Seq {
                type Repr = <u64 as ::atomiks::Atom>::Repr;
                type Validity = <u64 as ::atomiks::Atom>::Validity;
                const REPRS: ::atomiks::ReprRange< <u64 as ::atomiks::Atom>::Repr > =
                    <u64 as ::atomiks::Atom>::REPRS;
                const TAG_WIDTH: ::core::primitive::u32 = <u64 as ::atomiks::Atom>::TAG_WIDTH;
                const POINTEE_ALIGNMENT: ::atomiks::__private::PointeeAlignment =
                    <u64 as ::atomiks::Atom>::POINTEE_ALIGNMENT;
                #[inline]
                fn to_repr(self) -> <u64 as ::atomiks::Atom>::Repr {
                    ::atomiks::__private::to_repr::<u64>(self.0)
                }
                #[inline]
                fn to_tagged_repr(
                    self, tags: ::atomiks::__private::Tags,
                ) -> (<u64 as ::atomiks::Atom>::Repr, ::core::primitive::usize) {
                    ::atomiks::__private::to_tagged_repr::<u64>(self.0, tags)
                }
                #[inline]
                fn from_repr(repr: <u64 as ::atomiks::Atom>::Repr) -> ::core::option::Option<Self> {
                    match ::atomiks::__private::from_repr::<u64>(repr) {
                        ::core::option::Option::Some(field) => ::core::option::Option::Some(Self(field)),
                        ::core::option::Option::None => ::core::option::Option::None,
                    }
                }
                #[inline]
                unsafe fn from_repr_unchecked(repr: <u64 as ::atomiks::Atom>::Repr) -> Self {
                    Self(unsafe { ::atomiks::__private::from_repr_unchecked::<u64>(repr) })
                }
            }
        };
        let seq = derive_atom(quote! { struct Seq(u64); });
        assert_eq!(written(&seq), expected.to_string(), "the impl");
    }

    #[test]
    fn a_generic_newtype_bounds_its_field_and_converts_through_the_trait() {
        let expected = quote! {
            #[automatically_derived]
            const unsafe impl<T> ::atomiks::Atom for Wrap<T>
            where
                T: Copy,
                Self: ::core::marker::Copy,
                T: [const] ::atomiks::Atom<Repr = ::core::primitive::u32>,
                Self: ::core::marker::Send + ::core::marker::Sync
            {
                type Repr = <T as ::atomiks::Atom>::Repr;
                type Validity = <T as ::atomiks::Atom>::Validity;
                const REPRS: ::atomiks::ReprRange< <T as ::atomiks::Atom>::Repr > =
                    <T as ::atomiks::Atom>::REPRS;
                const TAG_WIDTH: ::core::primitive::u32 = <T as ::atomiks::Atom>::TAG_WIDTH;
                const POINTEE_ALIGNMENT: ::atomiks::__private::PointeeAlignment =
                    <T as ::atomiks::Atom>::POINTEE_ALIGNMENT;
                #[inline]
                fn to_repr(self) -> <T as ::atomiks::Atom>::Repr {
                    <T as ::atomiks::Atom>::to_repr(self.0)
                }
                #[inline]
                fn to_tagged_repr(
                    self, tags: ::atomiks::__private::Tags,
                ) -> (<T as ::atomiks::Atom>::Repr, ::core::primitive::usize) {
                    <T as ::atomiks::Atom>::to_tagged_repr(self.0, tags)
                }
                #[inline]
                fn from_repr(repr: <T as ::atomiks::Atom>::Repr) -> ::core::option::Option<Self> {
                    match <T as ::atomiks::Atom>::from_repr(repr) {
                        ::core::option::Option::Some(field) => ::core::option::Option::Some(Self(field)),
                        ::core::option::Option::None => ::core::option::Option::None,
                    }
                }
                #[inline]
                unsafe fn from_repr_unchecked(repr: <T as ::atomiks::Atom>::Repr) -> Self {
                    Self(unsafe { <T as ::atomiks::Atom>::from_repr_unchecked(repr) })
                }
            }
        };
        let wrap = derive_atom(quote! { #[atom(repr = u32)] struct Wrap<T>(T) where T: Copy; });
        assert_eq!(written(&wrap), expected.to_string(), "the impl");
    }

    #[test]
    fn markers_are_built_in_their_places_and_the_type_checked_beside_the_impl() {
        let id = derive_atom(quote! {
            #[atom(crate = renamed, repr = u32)]
            struct Id { kind: PhantomData<Venue>, value: u32, tag: PhantomData<fn() -> Venue> }
        });
        let code = written(&id);
        let checks = quote! {
            const _: () = renamed::__private::assert_send_and_sync::<Id>();
            const _: () = renamed::__private::assert_repr::<u32, ::core::primitive::u32>();
        };
        assert!(code.starts_with(&checks.to_string()), "the type's check, then the repr's: {code}");
        let built = quote! {
            Self { kind: ::core::marker::PhantomData, value: field, tag: ::core::marker::PhantomData }
        };
        assert!(code.contains(&built.to_string()), "each field in its place: {code}");
    }

    #[test]
    fn a_pointer_field_is_not_bounded_send_and_sync() {
        let code = written(&derive_atom(quote! { struct Head<T>(*mut T, PhantomData<T>); }));
        assert!(!code.contains("* mut T : :: core :: marker :: Send"), "the pointer: {code}");
        assert!(code.contains("PhantomData < T > : :: core :: marker :: Send"), "but the marker");
        assert!(!code.contains("Self : :: core :: marker :: Send"), "nor the type: {code}");
    }

    #[test]
    fn a_capability_is_its_fields() {
        let concrete = quote! {
            const _: () = ::atomiks::__private::assert_atom_add::<u64>();
            #[automatically_derived]
            impl ::atomiks::AtomAdd for Seq {}
        };
        let seq = expand_capability(quote! { struct Seq(u64); }, Capability::Add);
        assert_eq!(written(&seq), concrete.to_string(), "checked beside it for a concrete field");
        let generic = quote! {
            #[automatically_derived]
            impl<T> ::atomiks::AtomOrd for Wrap<T>
            where
                Self: ::core::marker::Copy,
                T: ::atomiks::AtomOrd,
                Self: ::core::marker::Send + ::core::marker::Sync
            {}
        };
        let wrap = expand_capability(quote! { struct Wrap<T>(T); }, Capability::Ord);
        assert_eq!(
            written(&wrap),
            generic.to_string(),
            "bounded as `Atom`'s impl for a generic one"
        );
    }

    #[test]
    fn a_fieldless_enum_reads_each_discriminant_by_casting_its_variant() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Sign>();
            const _: () = {
                type Repr = <::core::primitive::i8 as ::atomiks::__private::SelectRepr>::Repr;
                const _: () = ::atomiks::__private::assert_width::<Sign>(::core::primitive::i8::BITS);
                const discriminant_0: Repr = Sign::Minus as Repr;
                const discriminant_1: Repr = Sign::Flat as Repr;
                const discriminant_2: Repr = Sign::Plus as Repr;
                #[automatically_derived]
                const unsafe impl ::atomiks::Atom for Sign {
                    type Repr = Repr;
                    type Validity = <
                        ::atomiks::__private::ValidityCode<{
                            ::atomiks::__private::discriminant_validity_code([
                                discriminant_0, discriminant_1, discriminant_2
                            ])
                        }>
                        as ::atomiks::__private::SelectValidity
                    >::Validity;
                    const REPRS: ::atomiks::ReprRange<Repr> = ::atomiks::__private::discriminant_range([
                        discriminant_0, discriminant_1, discriminant_2
                    ]);
                    #[inline]
                    fn to_repr(self) -> Repr {
                        match self {
                            Self::Minus => discriminant_0,
                            Self::Flat => discriminant_1,
                            Self::Plus => discriminant_2,
                        }
                    }
                    #[inline]
                    fn from_repr(repr: Repr) -> ::core::option::Option<Self> {
                        match repr {
                            discriminant_0 => ::core::option::Option::Some(Self::Minus),
                            discriminant_1 => ::core::option::Option::Some(Self::Flat),
                            discriminant_2 => ::core::option::Option::Some(Self::Plus),
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    unsafe fn from_repr_unchecked(repr: Repr) -> Self {
                        match repr {
                            discriminant_0 => Self::Minus,
                            discriminant_1 => Self::Flat,
                            discriminant_2 => Self::Plus,
                            _ => unsafe { ::core::hint::unreachable_unchecked() },
                        }
                    }
                }
            };
        };
        let sign = derive_atom(quote! { #[repr(i8)] enum Sign { Minus = -1, Flat, Plus } });
        assert_eq!(written(&sign), expected.to_string(), "the impl");
    }

    #[test]
    fn a_fieldless_enum_without_a_repr_selects_the_narrowest_that_holds_its_discriminants() {
        let selected = quote! {
            type Repr = <
                ::atomiks::__private::Width<{
                    ::atomiks::__private::narrowest_width(::atomiks::__private::discriminant_width([
                        Side::Bid as ::core::primitive::i128,
                        Side::Ask as ::core::primitive::i128
                    ]))
                }>
                as ::atomiks::__private::SelectRepr
            >::Repr;
            const discriminant_0: Repr = Side::Bid as Repr;
        };
        let code = written(&derive_atom(quote! { enum Side { Bid, Ask } }));
        assert!(code.contains(&selected.to_string()), "selected: {code}");
    }

    #[test]
    fn a_stated_repr_is_checked_to_be_held_and_to_hold_each_discriminant() {
        let stated = quote! {
            type Repr = <::core::primitive::u32 as ::atomiks::__private::SelectRepr>::Repr;
            const _: () = ::atomiks::__private::assert_width::<Phase>(::core::primitive::u32::BITS);
            const _: () = ::atomiks::__private::assert_stated_width::<Phase, ::core::primitive::u32>(
                ::atomiks::__private::discriminant_width([
                    Phase::Pre as ::core::primitive::i128,
                    Phase::Open as ::core::primitive::i128
                ])
            );
        };
        let code =
            written(&derive_atom(quote! { #[atom(repr = u32)] enum Phase { Pre = 1, Open } }));
        assert!(code.contains(&stated.to_string()), "checked: {code}");
    }

    #[test]
    fn a_c_enum_is_stored_as_cs_int_checked_to_hold_each_discriminant() {
        let c_int = quote! {
            type Repr = <::core::ffi::c_int as ::atomiks::__private::SelectRepr>::Repr;
            const _: () = ::atomiks::__private::assert_stated_width::<Mode, ::core::ffi::c_int>(
                ::atomiks::__private::discriminant_width([
                    Mode::Off as ::core::primitive::i128,
                    Mode::On as ::core::primitive::i128
                ])
            );
        };
        let code = written(&derive_atom(quote! { #[repr(C)] enum Mode { Off, On = 4 } }));
        assert!(code.contains(&c_int.to_string()), "C's int: {code}");
    }

    #[test]
    fn a_unit_struct_is_zero_alone() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Marker>();
            #[automatically_derived]
            const unsafe impl ::atomiks::Atom for Marker {
                type Repr = ::core::primitive::u8;
                type Validity = ::atomiks::validity::ZeroValid;
                const REPRS: ::atomiks::ReprRange<::core::primitive::u8> = ::atomiks::ReprRange::new(0, 0);
                #[inline]
                fn to_repr(self) -> ::core::primitive::u8 {
                    0
                }
                #[inline]
                fn from_repr(repr: ::core::primitive::u8) -> ::core::option::Option<Self> {
                    match repr {
                        0 => ::core::option::Option::Some(Self),
                        _ => ::core::option::Option::None,
                    }
                }
                #[inline]
                unsafe fn from_repr_unchecked(_: ::core::primitive::u8) -> Self {
                    Self
                }
            }
        };
        let marker = derive_atom(quote! { struct Marker; });
        assert_eq!(written(&marker), expected.to_string(), "the impl");
    }

    #[test]
    fn a_struct_of_markers_builds_each_and_bounds_it_by_its_thread_safety() {
        let tag = written(&derive_atom(quote! { struct Tag<K>(PhantomData<K>); }));
        let bounds = quote! {
            where
                Self: ::core::marker::Copy,
                Self: ::core::marker::Send + ::core::marker::Sync
        };
        assert!(tag.contains(&bounds.to_string()), "a generic one is bounded: {tag}");
        let built = quote!(Self(::core::marker::PhantomData)).to_string();
        assert!(tag.contains(&built), "and built by position: {tag}");
        let kind = written(&derive_atom(quote! {
            #[atom(repr = u64)]
            struct Kind { kind: PhantomData<Venue> }
        }));
        let check = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Kind>();
        };
        assert!(kind.starts_with(&check.to_string()), "a concrete one checked beside it: {kind}");
        let built = quote!(Self { kind: ::core::marker::PhantomData }).to_string();
        assert!(kind.contains(&built), "and built by name: {kind}");
        let pinned = quote!(
            type Repr = <::core::primitive::u64 as ::atomiks::__private::SelectRepr>::Repr;
        );
        assert!(kind.contains(&pinned.to_string()), "in the repr stated: {kind}");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "the whole expansion it pins, written out, is that long"
    )]
    fn a_packed_struct_places_each_field_where_the_one_before_it_ends() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Step>();
            const _: () = {
                #[automatically_derived]
                unsafe impl ::atomiks::__private::HasPackedField<0, u8> for Step {
                    const PLACEMENT: ::atomiks::__private::PackedField = placement_0;
                    const LAYOUT: ::atomiks::__private::PackedLayout = layout;
                    type Reach = ::atomiks::__private::Reach<{
                        ::atomiks::__private::reaches_top::<Repr>(placement_0)
                    }>;
                    #[inline]
                    fn field(self) -> u8 {
                        self.length
                    }
                }
                #[automatically_derived]
                unsafe impl ::atomiks::__private::HasPackedField<1, Sign> for Step {
                    const PLACEMENT: ::atomiks::__private::PackedField = placement_1;
                    const LAYOUT: ::atomiks::__private::PackedLayout = layout;
                    type Reach = ::atomiks::__private::Reach<{
                        ::atomiks::__private::reaches_top::<Repr>(placement_1)
                    }>;
                    #[inline]
                    fn field(self) -> Sign {
                        self.sign
                    }
                }
                #[automatically_derived]
                const unsafe impl ::atomiks::ProjectFields for Step {
                    type Fields<'a, P: ::atomiks::FieldPath<Value = Self> + 'a> = StepFields<'a, P>
                    where
                        Self: 'a;
                    #[inline]
                    fn project<P: ::atomiks::FieldPath<Value = Self>>(
                        place: &::atomiks::AtomicField<P>,
                    ) -> StepFields<'_, P> {
                        StepFields {
                            length: unsafe { ::atomiks::__private::project_field(place) },
                            sign: unsafe { ::atomiks::__private::project_field(place) }
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, P: ::atomiks::FieldPath<Value = Step> + 'a> ::core::fmt::Debug for StepFields<'a, P>
                where
                    &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Step, 0, u8>>>: ::core::fmt::Debug,
                    &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Step, 1, Sign>>>: ::core::fmt::Debug
                {
                    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        formatter
                            .debug_struct("StepFields")
                            .field("length", &self.length)
                            .field("sign", &self.sign)
                            .finish()
                    }
                }
                const placement_0: ::atomiks::__private::PackedField =
                    ::atomiks::__private::PackedField::new(<u8 as ::atomiks::Atom>::REPRS, 0);
                const placement_1: ::atomiks::__private::PackedField =
                    ::atomiks::__private::PackedField::new(<Sign as ::atomiks::Atom>::REPRS, placement_0.next_offset());
                const layout: ::atomiks::__private::PackedLayout =
                    ::atomiks::__private::PackedLayout::new(&[placement_0, placement_1]);
                type Repr = <
                    ::atomiks::__private::Width<{ ::atomiks::__private::narrowest_width(layout.width()) }>
                    as ::atomiks::__private::SelectRepr
                >::Repr;
                const _: () = ::atomiks::__private::assert_width::<Step>(layout.width());
                #[automatically_derived]
                const unsafe impl ::atomiks::Atom for Step {
                    type Repr = Repr;
                    type Validity = <
                        ::atomiks::__private::ValidityCode<{
                            ::atomiks::__private::PackedValidity::EMPTY
                                .with_field::<<u8 as ::atomiks::Atom>::Validity>(placement_0.layout())
                                .with_field::<<Sign as ::atomiks::Atom>::Validity>(placement_1.layout())
                                .code(layout.width(), <Repr as ::atomiks::Primitive>::BITS)
                        }>
                        as ::atomiks::__private::SelectValidity
                    >::Validity;
                    const REPRS: ::atomiks::ReprRange<Repr> = layout.range();
                    #[inline]
                    fn to_repr(self) -> Repr {
                        layout.repr(
                            placement_0.pack(::atomiks::__private::to_bits::<u8>(self.length))
                                | placement_1.pack(::atomiks::__private::to_bits::<Sign>(self.sign))
                        )
                    }
                    #[inline]
                    fn from_repr(repr: Repr) -> ::core::option::Option<Self> {
                        match layout.canonical_bits(repr) {
                            ::core::option::Option::Some(bits) => match (
                                ::atomiks::__private::from_bits::<u8>(placement_0.unpack(bits)),
                                ::atomiks::__private::from_bits::<Sign>(placement_1.unpack(bits)),
                            ) {
                                (
                                    ::core::option::Option::Some(value_0),
                                    ::core::option::Option::Some(value_1),
                                ) => ::core::option::Option::Some(Self { length: value_0, sign: value_1 }),
                                _ => ::core::option::Option::None,
                            },
                            ::core::option::Option::None => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    unsafe fn from_repr_unchecked(repr: Repr) -> Self {
                        let bits = ::atomiks::__private::to_bits(repr);
                        Self {
                            length: unsafe {
                                ::atomiks::__private::from_bits_unchecked::<u8>(placement_0.unpack(bits))
                            },
                            sign: unsafe {
                                ::atomiks::__private::from_bits_unchecked::<Sign>(placement_1.unpack(bits))
                            }
                        }
                    }
                }
            };
            #[doc = " The fields of an atomic [`Step`], or of a field whose value is one, each a place of its own: what [`fields()`](atomiks::Atomic::fields) lends."]
            #[derive(::core::clone::Clone, ::core::marker::Copy)]
            struct StepFields<'a, P: ::atomiks::FieldPath<Value = Step> + 'a> {
                #[doc = " The field `length`, of type `u8`, in an atomic `Step`."]
                length: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Step, 0, u8>>>,
                #[doc = " The field `sign`, of type `Sign`, in an atomic `Step`."]
                sign: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Step, 1, Sign>>>
            }
        };
        let step = derive_atom(quote! { struct Step { length: u8, sign: Sign } });
        assert_eq!(written(&step), expected.to_string(), "the impl and the projection");
    }

    #[test]
    fn each_place_of_the_projection_takes_its_fields_visibility_and_docs() {
        let quote = written(&derive_atom(quote! {
            pub struct Quote {
                /// How many.
                pub quantity: u32,
                side: Side,
                pub(crate) live: bool,
            }
        }));
        let fields = quote! {
            pub struct QuoteFields<'a, P: ::atomiks::FieldPath<Value = Quote> + 'a> {
                #[doc = r" How many."]
                #[doc = ""]
                #[doc = " The field `quantity`, of type `u32`, in an atomic `Quote`."]
                pub quantity: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Quote, 0, u32>>>,
                #[doc = " The field `side`, of type `Side`, in an atomic `Quote`."]
                side: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Quote, 1, Side>>>,
                #[doc = " The field `live`, of type `bool`, in an atomic `Quote`."]
                pub(crate) live: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Quote, 2, bool>>>
            }
        };
        assert!(quote.contains(&fields.to_string()), "each place as its field is: {quote}");
    }

    #[test]
    fn the_projection_is_hidden_and_open_as_its_struct_is() {
        let quote = written(&derive_atom(quote! {
            #[doc(hidden)]
            #[non_exhaustive]
            #[derive(Debug)]
            pub struct Quote { pub quantity: u32, pub live: bool }
        }));
        let attributes = quote! {
            #[doc(hidden)]
            #[non_exhaustive]
            #[derive(::core::clone::Clone, ::core::marker::Copy)]
            pub struct QuoteFields
        };
        assert!(quote.contains(&attributes.to_string()), "those two carried, alone: {quote}");
    }

    #[test]
    fn a_tuple_structs_projection_is_a_tuple_struct() {
        let code = written(&derive_atom(quote! { struct Pair(u16, bool); }));
        let fields = quote! {
            struct PairFields<'a, P: ::atomiks::FieldPath<Value = Pair> + 'a>(
                #[doc = " The field `0`, of type `u16`, in an atomic `Pair`."]
                &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Pair, 0, u16>>>,
                #[doc = " The field `1`, of type `bool`, in an atomic `Pair`."]
                &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Pair, 1, bool>>>
            );
        };
        assert!(code.contains(&fields.to_string()), "a tuple struct: {code}");
        let built =
            quote!(PairFields(unsafe { ::atomiks::__private::project_field(place) }, unsafe {
                ::atomiks::__private::project_field(place)
            }));
        assert!(code.contains(&built.to_string()), "built by position: {code}");
        let shown =
            quote!(formatter.debug_tuple("PairFields").field(&self.0).field(&self.1).finish());
        assert!(code.contains(&shown.to_string()), "and printed so: {code}");
    }

    #[test]
    fn a_generic_structs_fields_lie_where_each_instance_puts_them() {
        let code = written(&derive_atom(quote! {
            #[atom(repr = u64)]
            struct Tagged<T> { marked: bool, value: T }
        }));
        let placed = quote! {
            const PLACEMENT: ::atomiks::__private::PackedField =
                lay_out(::core::marker::PhantomData::<Self>).1;
            const LAYOUT: ::atomiks::__private::PackedLayout =
                lay_out(::core::marker::PhantomData::<Self>).2;
            type Reach = ::atomiks::__private::Reach<false>;
        };
        assert!(code.contains(&placed.to_string()), "each instance's placement: {code}");
        let bounded = quote! {
            struct TaggedFields<'a, T, P: ::atomiks::FieldPath<Value = Tagged<T> > + 'a>
            where
                Tagged<T>: ::atomiks::__private::HasPackedField<0, bool> + ::atomiks::__private::HasPackedField<1, T>,
                T: ::atomiks::Atom
        };
        assert!(code.contains(&bounded.to_string()), "and each field's path bounded: {code}");
        let outlives = quote!(where Self: 'a, T: 'a;);
        assert!(code.contains(&outlives.to_string()), "with its parameter outliving `'a`: {code}");
    }

    #[test]
    fn the_projection_takes_the_structs_parameters_without_their_defaults() {
        let code = written(&derive_atom(quote! {
            #[atom(repr = u64)]
            struct Tagged<T: Copy = u32, const N: usize = 4> { marked: bool, value: T }
        }));
        let header = quote! {
            struct TaggedFields<'a, T: Copy, const N: usize, P: ::atomiks::FieldPath<Value = Tagged<T, N> > + 'a>
        };
        assert!(code.contains(&header.to_string()), "its parameters, not their defaults: {code}");
    }

    #[test]
    fn the_projections_parameters_take_names_the_structs_own_do_not() {
        let code = written(&derive_atom(quote! {
            #[atom(repr = u64)]
            struct Named<'a, P, Q> { marked: bool, value: P, other: Q, origin: PhantomData<&'a ()> }
        }));
        let header = quote! {
            struct NamedFields<'b, 'a, P, Q, R: ::atomiks::FieldPath<Value = Named<'a, P, Q> > + 'b>
        };
        assert!(code.contains(&header.to_string()), "`'b` and `R`: {code}");
    }

    #[test]
    fn a_type_reads_in_a_doc_as_its_source_does() {
        let code = written(&derive_atom(quote! {
            #[atom(repr = u128)]
            struct Spread<F> {
                low: RangedI8<-5, 5>,
                owner: OwnerId,
                raw: core::num::NonZero<u8>,
                price: f32,
                flags: F,
                r#type: Side,
            }
        }));
        for line in [
            "The field `low`, of type `RangedI8<-5, 5>`,",
            "The field `owner`, of type `OwnerId`,",
            "The field `raw`, of type `core::num::NonZero<u8>`,",
            "The field `price`, of type `f32`,",
            "The field `flags`, of type `F`,",
            "The field `type`, of type `Side`,",
        ] {
            assert!(code.contains(line), "{line}: {code}");
        }
    }

    #[test]
    fn a_packed_struct_is_checked_against_the_repr_it_states() {
        let stated = quote! {
            type Repr = <::core::primitive::u32 as ::atomiks::__private::SelectRepr>::Repr;
            const _: () = ::atomiks::__private::assert_width::<Quote>(::core::primitive::u32::BITS);
            const _: () =
                ::atomiks::__private::assert_stated_width::<Quote, ::core::primitive::u32>(layout.width());
        };
        let quote =
            derive_atom(quote! { #[atom(repr = u32)] struct Quote { quantity: u8, live: bool } });
        let code = written(&quote);
        assert!(code.contains(&stated.to_string()), "checked: {code}");
    }

    #[test]
    fn a_packed_tuple_struct_is_built_by_position() {
        let code = written(&derive_atom(quote! { struct Pair(u8, bool); }));
        let built = quote!(Self(value_0, value_1)).to_string();
        assert!(code.contains(&built), "built by position: {code}");
    }

    #[test]
    fn a_generic_packed_struct_lays_out_each_instance_in_the_repr_it_states() {
        let layout = quote! {
            const fn lay_out<A, B>(_: ::core::marker::PhantomData<Pair<A, B> >) -> (
                ::atomiks::__private::PackedField,
                ::atomiks::__private::PackedField,
                ::atomiks::__private::PackedField,
                ::atomiks::__private::PackedLayout,
            )
            where
                A: ::atomiks::Atom,
                B: ::atomiks::Atom
            {
                let placement_0 = ::atomiks::__private::PackedField::new(<A as ::atomiks::Atom>::REPRS, 0);
                let placement_1 = ::atomiks::__private::PackedField::new(<u8 as ::atomiks::Atom>::REPRS, placement_0.next_offset());
                let placement_2 = ::atomiks::__private::PackedField::new(<B as ::atomiks::Atom>::REPRS, placement_1.next_offset());
                let layout = ::atomiks::__private::PackedLayout::new(&[placement_0, placement_1, placement_2]);
                ::atomiks::__private::assert_stated_width::<Pair<A, B>, ::core::primitive::u64>(
                    layout.width()
                );
                (placement_0, placement_1, placement_2, layout,)
            }
        };
        let bounds = quote! {
            where
                Self: ::core::marker::Copy,
                A: [const] ::atomiks::Atom<Repr: [const] ::atomiks::__private::FieldRepr>,
                B: [const] ::atomiks::Atom<Repr: [const] ::atomiks::__private::FieldRepr>,
                Self: ::core::marker::Send + ::core::marker::Sync
        };
        let validity = quote! {
            type Validity =
                <<A as ::atomiks::Atom>::Validity as ::atomiks::validity::Validity>::ZeroValidityWith<
                    <<u8 as ::atomiks::Atom>::Validity as ::atomiks::validity::Validity>::ZeroValidityWith<
                        <<B as ::atomiks::Atom>::Validity as ::atomiks::validity::Validity>::ZeroValidityWith<
                            ::atomiks::validity::ZeroValid
                        >
                    >
                >;
        };
        let bound = quote! {
            let (placement_0, placement_1, placement_2, layout,) =
                const { lay_out(::core::marker::PhantomData::<Self>) };
        };
        let pair = derive_atom(quote! {
            #[atom(repr = u64)]
            struct Pair<A, B> { first: A, tag: u8, second: B }
        });
        let code = written(&pair);
        let check = quote!(
            const _: () = ::atomiks::__private::assert_send_and_sync::<u8>();
        );
        assert!(code.starts_with(&check.to_string()), "the concrete field checked: {code}");
        assert!(code.contains(&layout.to_string()), "laid out for each instance: {code}");
        assert!(code.contains(&bounds.to_string()), "each generic field bounded: {code}");
        assert!(code.contains(&validity.to_string()), "zero valid where each field is: {code}");
        assert!(code.contains(&bound.to_string()), "and each conversion reaches it: {code}");
    }

    #[test]
    #[expect(clippy::too_many_lines, reason = "the impl is checked whole, as a crate gets it")]
    fn a_tagged_enum_puts_its_tag_above_the_widest_variant() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Slot>();
            const _: () = {
                type Discriminant = ::core::primitive::isize;
                const discriminant_0: Discriminant = 0;
                const discriminant_1: Discriminant = 1;
                const discriminant_2: Discriminant = 2;
                const placement_1_0: ::atomiks::__private::PackedField =
                    ::atomiks::__private::PackedField::new(<u32 as ::atomiks::Atom>::REPRS, 0);
                const variant_1: ::atomiks::__private::PackedLayout =
                    ::atomiks::__private::PackedLayout::new(&[placement_1_0]);
                const placement_2_0: ::atomiks::__private::PackedField =
                    ::atomiks::__private::PackedField::new(<Sign as ::atomiks::Atom>::REPRS, 0);
                const variant_2: ::atomiks::__private::PackedLayout =
                    ::atomiks::__private::PackedLayout::new(&[placement_2_0]);
                const layout: ::atomiks::__private::EnumLayout =
                    ::atomiks::__private::EnumLayout::tagged(
                        ::atomiks::__private::discriminant_range([
                            discriminant_0, discriminant_1, discriminant_2
                        ]),
                        &[variant_1, variant_2]
                    );
                type Repr = <
                    ::atomiks::__private::Width<{ ::atomiks::__private::narrowest_width(layout.width()) }>
                    as ::atomiks::__private::SelectRepr
                >::Repr;
                const _: () = ::atomiks::__private::assert_width::<Slot>(layout.width());
                #[automatically_derived]
                const unsafe impl ::atomiks::Atom for Slot {
                    type Repr = Repr;
                    type Validity = <
                        ::atomiks::__private::ValidityCode<{
                            ::atomiks::__private::EnumValidity::new(layout)
                                .with_unit(discriminant_0)
                                .with_variant(
                                    discriminant_1,
                                    variant_1,
                                    ::atomiks::__private::PackedValidity::EMPTY
                                        .with_field::<<u32 as ::atomiks::Atom>::Validity>(placement_1_0.layout())
                                )
                                .with_variant(
                                    discriminant_2,
                                    variant_2,
                                    ::atomiks::__private::PackedValidity::EMPTY
                                        .with_field::<<Sign as ::atomiks::Atom>::Validity>(placement_2_0.layout())
                                )
                                .code(<Repr as ::atomiks::Primitive>::BITS)
                        }>
                        as ::atomiks::__private::SelectValidity
                    >::Validity;
                    const REPRS: ::atomiks::ReprRange<Repr> = layout.range();
                    #[inline]
                    fn to_repr(self) -> Repr {
                        match self {
                            Self::Empty => layout.repr(discriminant_0, 0),
                            Self::Writing { lap: value_0 } => layout.repr(
                                discriminant_1,
                                placement_1_0.pack(::atomiks::__private::to_bits::<u32>(value_0))
                            ),
                            Self::Turned(value_0) => layout.repr(
                                discriminant_2,
                                placement_2_0.pack(::atomiks::__private::to_bits::<Sign>(value_0))
                            ),
                        }
                    }
                    #[inline]
                    fn from_repr(repr: Repr) -> ::core::option::Option<Self> {
                        match layout.canonical_bits(repr) {
                            ::core::option::Option::Some(bits) => match layout.discriminant::<Discriminant>(bits) {
                                discriminant_0 if layout.is_clear_below_selector(bits) =>
                                    ::core::option::Option::Some(Self::Empty),
                                discriminant_1 if layout.is_clear_above_fields(bits, variant_1) => match
                                    ::atomiks::__private::from_bits::<u32>(placement_1_0.unpack(bits))
                                {
                                    ::core::option::Option::Some(value_0) =>
                                        ::core::option::Option::Some(Self::Writing { lap: value_0 }),
                                    ::core::option::Option::None => ::core::option::Option::None,
                                },
                                discriminant_2 if layout.is_clear_above_fields(bits, variant_2) => match
                                    ::atomiks::__private::from_bits::<Sign>(placement_2_0.unpack(bits))
                                {
                                    ::core::option::Option::Some(value_0) =>
                                        ::core::option::Option::Some(Self::Turned(value_0)),
                                    ::core::option::Option::None => ::core::option::Option::None,
                                },
                                _ => ::core::option::Option::None,
                            },
                            ::core::option::Option::None => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    unsafe fn from_repr_unchecked(repr: Repr) -> Self {
                        let bits = ::atomiks::__private::to_bits(repr);
                        match layout.discriminant::<Discriminant>(bits) {
                            discriminant_0 => Self::Empty,
                            discriminant_1 => Self::Writing {
                                lap: unsafe {
                                    ::atomiks::__private::from_bits_unchecked::<u32>(placement_1_0.unpack(bits))
                                }
                            },
                            discriminant_2 => Self::Turned(unsafe {
                                ::atomiks::__private::from_bits_unchecked::<Sign>(placement_2_0.unpack(bits))
                            }),
                            _ => unsafe { ::core::hint::unreachable_unchecked() },
                        }
                    }
                }
            };
        };
        let slot = derive_atom(quote! { enum Slot { Empty, Writing { lap: u32 }, Turned(Sign) } });
        assert_eq!(written(&slot), expected.to_string(), "the impl");
    }

    #[test]
    fn one_variant_with_fields_among_units_fills_a_niche_where_it_is_no_wider() {
        let gate = written(&derive_atom(quote! { enum Gate { Closed, Open(OwnerId), Held } }));
        let layout = quote! {
            const layout: ::atomiks::__private::EnumLayout =
                ::atomiks::__private::EnumLayout::niche_or_tagged(2, variant_1, discriminant_1);
        };
        assert!(gate.contains(&layout.to_string()), "two units beside the payload: {gate}");
        let explicit = written(&derive_atom(quote! {
            #[repr(u8)]
            enum Gate { Closed = 1, Open(OwnerId) = 2 }
        }));
        let tagged = quote! {
            ::atomiks::__private::EnumLayout::tagged(
                ::atomiks::__private::discriminant_range([discriminant_0, discriminant_1]), &[variant_1]
            )
        };
        assert!(explicit.contains(&tagged.to_string()), "but tagged by those stated: {explicit}");
    }

    #[test]
    fn each_discriminant_is_a_constant_of_the_enums_integer() {
        let level = written(&derive_atom(quote! {
            #[repr(i8)]
            enum Level { Low(u8) = -1 - OFFSET, Mid, High = 4, Top }
        }));
        let discriminants = quote! {
            type Discriminant = ::core::primitive::i8;
            const discriminant_0: Discriminant = -1 - OFFSET;
            const discriminant_1: Discriminant = discriminant_0.wrapping_add(1);
            const discriminant_2: Discriminant = 4;
            const discriminant_3: Discriminant = discriminant_2.wrapping_add(1);
        };
        assert!(level.contains(&discriminants.to_string()), "as stated, or one past: {level}");
        let based = written(&derive_atom(quote! {
            #[repr(u8)]
            enum Based { Low = Self::BASE + 1, High(u8) }
        }));
        let low = quote!(
            const discriminant_0: Discriminant = Based::BASE + 1;
        );
        assert!(based.contains(&low.to_string()), "`Self` naming the enum: {based}");
    }

    #[test]
    fn a_unit_variant_written_with_parentheses_or_braces_is_built_so() {
        let mode = written(&derive_atom(quote! { enum Mode { Off(), On {}, Idle, Busy(u8) } }));
        for built in [
            quote!(Self::Off {} => layout.repr(discriminant_0, 0)),
            quote!(Self::Idle => layout.repr(discriminant_2, 0)),
        ] {
            assert!(mode.contains(&built.to_string()), "`{built}`: {mode}");
        }
    }

    #[test]
    fn a_generic_enum_lays_out_each_instance_and_promises_zero_where_a_unit_takes_it() {
        let lock = written(&derive_atom(quote! {
            #[atom(repr = u64)]
            enum Lock<O> { Uninit, Free, Owned(O) }
        }));
        let layout = quote! {
            const fn lay_out<O>(_: ::core::marker::PhantomData<Lock<O> >) -> (
                ::atomiks::__private::PackedField,
                ::atomiks::__private::PackedLayout,
                ::atomiks::__private::EnumLayout,
            )
            where
                O: ::atomiks::Atom
            {
                let placement_2_0 = ::atomiks::__private::PackedField::new(<O as ::atomiks::Atom>::REPRS, 0);
                let variant_2 = ::atomiks::__private::PackedLayout::new(&[placement_2_0]);
                let layout = ::atomiks::__private::EnumLayout::niche_or_tagged(2, variant_2, discriminant_2);
                ::atomiks::__private::assert_stated_width::<Lock<O>, ::core::primitive::u64>(
                    layout.width()
                );
                (placement_2_0, variant_2, layout,)
            }
        };
        assert!(lock.contains(&layout.to_string()), "laid out for each instance: {lock}");
        let partial = quote!(
            type Validity = ::atomiks::validity::Partial;
        );
        assert!(lock.contains(&partial.to_string()), "promising nothing of zero: {lock}");
        let word = written(&derive_atom(quote! {
            #[atom(repr = u64)]
            #[repr(u8)]
            enum Word<O> { Unbuilt = 0, Free = 1, Owned(O) = 2, Poisoned = 3 }
        }));
        let zero_valid = quote! {
            if discriminant_0 == 0 || discriminant_1 == 0 || discriminant_3 == 0 {
                ::atomiks::__private::ZERO_VALID
            } else {
                ::atomiks::__private::PARTIAL
            }
        };
        assert!(word.contains(&zero_valid.to_string()), "but where a unit is zero: {word}");
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "the whole expansion it pins, written out, is that long"
    )]
    fn a_pointer_word_packs_its_tags_where_their_width_puts_them_and_projects_each_field() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<bool>();
            const _: () = {
                #[automatically_derived]
                unsafe impl ::atomiks::__private::HasPackedField<0, NonNull<Node> > for Link {
                    const PLACEMENT: ::atomiks::__private::PackedField = layout.pointer_placement();
                    const LAYOUT: ::atomiks::__private::PackedLayout = layout.packed_layout();
                    type Reach = ::atomiks::__private::Reach<false>;
                    #[inline]
                    fn field(self) -> NonNull<Node> {
                        self.next
                    }
                }
                #[automatically_derived]
                unsafe impl ::atomiks::__private::HasPackedField<1, bool> for Link {
                    const PLACEMENT: ::atomiks::__private::PackedField = placement_0;
                    const LAYOUT: ::atomiks::__private::PackedLayout = layout.packed_layout();
                    type Reach = ::atomiks::__private::Reach<{
                        ::atomiks::__private::reaches_top::<Repr>(placement_0)
                    }>;
                    #[inline]
                    fn field(self) -> bool {
                        self.deleted
                    }
                }
                #[automatically_derived]
                const unsafe impl ::atomiks::ProjectFields for Link {
                    type Fields<'a, P: ::atomiks::FieldPath<Value = Self> + 'a>
                        = LinkFields<'a, P>
                    where
                        Self: 'a;
                    #[inline]
                    fn project<P: ::atomiks::FieldPath<Value = Self>>(
                        place: &::atomiks::AtomicField<P>,
                    ) -> LinkFields<'_, P> {
                        LinkFields {
                            next: unsafe { ::atomiks::__private::project_field(place) },
                            deleted: unsafe { ::atomiks::__private::project_field(place) }
                        }
                    }
                }
                #[automatically_derived]
                impl<'a, P: ::atomiks::FieldPath<Value = Link> + 'a> ::core::fmt::Debug for LinkFields<'a, P>
                where
                    &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Link, 0, NonNull<Node> >> >:
                        ::core::fmt::Debug,
                    &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Link, 1, bool>>>:
                        ::core::fmt::Debug
                {
                    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        formatter
                            .debug_struct("LinkFields")
                            .field("next", &self.next)
                            .field("deleted", &self.deleted)
                            .finish()
                    }
                }
                const word_count: ::core::primitive::u32 = ::atomiks::__private::PointerWordLayout::word_count(
                    <NonNull<Node> as ::atomiks::Atom>::TAG_WIDTH,
                    &[::atomiks::__private::PackedField::new(<bool as ::atomiks::Atom>::REPRS, 0)]
                );
                const placement_0: ::atomiks::__private::PackedField = ::atomiks::__private::PackedField::new(
                    <bool as ::atomiks::Atom>::REPRS,
                    ::atomiks::__private::PointerWordLayout::tag_fields_offset(
                        word_count,
                        <NonNull<Node> as ::atomiks::Atom>::TAG_WIDTH
                    )
                );
                const layout: ::atomiks::__private::PointerWordLayout =
                    ::atomiks::__private::PointerWordLayout::in_words(
                        word_count,
                        <NonNull<Node> as ::atomiks::Atom>::TAG_WIDTH,
                        ::atomiks::__private::PackedLayout::new(&[placement_0])
                    );
                const alignment: ::atomiks::__private::PointeeAlignment =
                    ::atomiks::__private::PointeeAlignment::least(&[
                        <NonNull<Node> as ::atomiks::Atom>::POINTEE_ALIGNMENT
                    ]);
                type Repr = <::atomiks::__private::Words<{ word_count }> as ::atomiks::__private::SelectPointerWordRepr<
                    <NonNull<Node> as ::atomiks::Atom>::Repr
                >>::Repr;
                const _: () = ::atomiks::__private::assert_pointer::<NonNull<Node> >();
                const _: () = layout.assert_tags_fit::<Link, NonNull<Node> >("tag field `deleted` needs");
                #[automatically_derived]
                const unsafe impl ::atomiks::Atom for Link {
                    type Repr = Repr;
                    type Validity = <::atomiks::__private::ValidityCode<
                        {
                            ::atomiks::__private::PackedValidity::EMPTY
                                .with_field::<<bool as ::atomiks::Atom>::Validity>(placement_0.layout())
                                .with_field::<<NonNull<Node> as ::atomiks::Atom>::Validity>(
                                    layout.pointer_layout()
                                )
                                .code(
                                    layout.width::<Repr>(),
                                    <Repr as ::atomiks::Primitive>::BITS
                                )
                        }
                    > as ::atomiks::__private::SelectValidity>::Validity;
                    const REPRS: ::atomiks::ReprRange<Repr> =
                        layout.range(<NonNull<Node> as ::atomiks::Atom>::REPRS);
                    const TAG_WIDTH: ::core::primitive::u32 = layout.tag_width();
                    const POINTEE_ALIGNMENT: ::atomiks::__private::PointeeAlignment = alignment;
                    #[inline]
                    fn to_repr(self) -> Repr {
                        let (repr, misaligned) = ::atomiks::__private::to_tagged_repr::<Self>(
                            self,
                            ::atomiks::__private::Tags::EMPTY
                        );
                        ::atomiks::__private::assert_aligned::<Self>(misaligned);
                        repr
                    }
                    #[inline]
                    fn to_tagged_repr(
                        self, tags: ::atomiks::__private::Tags
                    ) -> (Repr, ::core::primitive::usize) {
                        let bits = placement_0.pack(::atomiks::__private::to_bits::<bool>(self.deleted));
                        let (pointer, misaligned) = ::atomiks::__private::to_tagged_repr::<NonNull<Node> >(
                            self.next,
                            layout.pointer_tags(bits, tags)
                        );
                        (layout.join_words(pointer, bits), misaligned)
                    }
                    #[inline]
                    fn from_repr(repr: Repr) -> ::core::option::Option<Self> {
                        let ::core::option::Option::Some((pointer, bits)) = layout.split_words(repr)
                        else {
                            return ::core::option::Option::None;
                        };
                        match (
                            ::atomiks::__private::from_repr::<NonNull<Node> >(pointer),
                            ::atomiks::__private::from_bits::<bool>(placement_0.unpack(bits)),
                        ) {
                            (
                                ::core::option::Option::Some(value_0),
                                ::core::option::Option::Some(value_1),
                            ) => ::core::option::Option::Some(Self { next: value_0, deleted: value_1 }),
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    unsafe fn from_repr_unchecked(repr: Repr) -> Self {
                        let (pointer, bits) = unsafe { layout.split_words(repr).unwrap_unchecked() };
                        Self {
                            next: unsafe {
                                ::atomiks::__private::from_repr_unchecked::<NonNull<Node> >(pointer)
                            },
                            deleted: unsafe {
                                ::atomiks::__private::from_bits_unchecked::<bool>(placement_0.unpack(bits))
                            }
                        }
                    }
                }
            };
            #[doc = " The fields of an atomic [`Link`], or of a field whose value is one, each a place of its own: what [`fields()`](atomiks::Atomic::fields) lends."]
            #[derive(:: core :: clone :: Clone, :: core :: marker :: Copy)]
            struct LinkFields<'a, P: ::atomiks::FieldPath<Value = Link> + 'a> {
                #[doc = " The field `next`, of type `NonNull<Node>`, in an atomic `Link`."]
                next: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Link, 0, NonNull<Node> >> >,
                #[doc = " The field `deleted`, of type `bool`, in an atomic `Link`."]
                deleted: &'a ::atomiks::AtomicField<::atomiks::Join<P, ::atomiks::Field<Link, 1, bool>>>
            }
        };
        let code = written(&derive_atom(quote! {
            struct Link { next: NonNull<Node>, deleted: bool }
        }));
        assert_eq!(code, expected.to_string(), "the impl, its layout and its projection");
    }

    #[test]
    fn a_pointer_enum_of_a_unit_and_a_pointer_never_null_fills_the_pointers_niche() {
        let expected = quote! {
            const _: () = {
                type Discriminant = ::core::primitive::isize;
                const discriminant_0: Discriminant = 0;
                const discriminant_1: Discriminant = 1;
                const variant_1: ::atomiks::__private::PackedLayout =
                    ::atomiks::__private::PackedLayout::new(&[]);
                const layout: ::atomiks::__private::PointerEnumLayout =
                    ::atomiks::__private::PointerEnumLayout::niche_or_tagged(&[
                        ::atomiks::__private::PointerEnumVariant::unit(discriminant_0),
                        ::atomiks::__private::PointerEnumVariant::pointer::<_, NonNull<Node> >(
                            discriminant_1, variant_1
                        )
                    ]);
                const promises: ::atomiks::__private::PointerEnumValidity =
                    ::atomiks::__private::PointerEnumValidity::new(layout)
                        .with_variant(discriminant_0, ::atomiks::__private::PackedValidity::EMPTY)
                        .with_variant(
                            discriminant_1,
                            ::atomiks::__private::PackedValidity::EMPTY
                                .with_field::<<NonNull<Node> as ::atomiks::Atom>::Validity>(
                                    layout.pointer_layout()
                                )
                        );
                const alignment: ::atomiks::__private::PointeeAlignment =
                    ::atomiks::__private::PointeeAlignment::least(&[
                        <NonNull<Node> as ::atomiks::Atom>::POINTEE_ALIGNMENT
                    ]);
                type Repr = *mut ();
                const _: () = layout.assert_tags_fit::<Next, NonNull<Node> >("`Next::Node`");
                #[automatically_derived]
                const unsafe impl ::atomiks::Atom for Next {
                    type Repr = Repr;
                    type Validity = <::atomiks::__private::ValidityCode<{ promises.code() }>
                        as ::atomiks::__private::SelectValidity>::Validity;
                    const REPRS: ::atomiks::ReprRange<Repr> = promises.range();
                    const TAG_WIDTH: ::core::primitive::u32 = layout.tag_width();
                    const POINTEE_ALIGNMENT: ::atomiks::__private::PointeeAlignment = alignment;
                    #[inline]
                    fn to_repr(self) -> *mut () {
                        let (repr, misaligned) = ::atomiks::__private::to_tagged_repr::<Self>(
                            self,
                            ::atomiks::__private::Tags::EMPTY
                        );
                        ::atomiks::__private::assert_aligned::<Self>(misaligned);
                        repr
                    }
                    #[inline]
                    fn to_tagged_repr(
                        self, tags: ::atomiks::__private::Tags
                    ) -> (*mut (), ::core::primitive::usize) {
                        match self {
                            Self::End => tags.set_in(layout.unit_repr(discriminant_0)),
                            Self::Node(pointer) => ::atomiks::__private::to_tagged_pointer::<NonNull<Node> >(
                                pointer,
                                layout.pointer_tags(discriminant_1, 0, tags)
                            ),
                        }
                    }
                    #[inline]
                    fn from_repr(repr: *mut ()) -> ::core::option::Option<Self> {
                        match layout.discriminant::<Discriminant>(repr) {
                            discriminant_0 if layout.is_unit(repr, discriminant_0) => ::core::option::Option::Some(Self::End),
                            discriminant_1 => {
                                let (pointer, bits) = layout.split(repr);
                                if !layout.is_clear_above_fields(bits, variant_1) {
                                    return ::core::option::Option::None;
                                }
                                match (::atomiks::__private::from_pointer::<NonNull<Node> >(pointer),) {
                                    (::core::option::Option::Some(value_0),) => ::core::option::Option::Some(Self::Node(value_0)),
                                    _ => ::core::option::Option::None,
                                }
                            },
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    unsafe fn from_repr_unchecked(repr: *mut ()) -> Self {
                        match layout.discriminant::<Discriminant>(repr) {
                            discriminant_0 => Self::End,
                            discriminant_1 => {
                                let (pointer, _) = layout.split(repr);
                                Self::Node(unsafe {
                                    ::atomiks::__private::from_pointer_unchecked::<NonNull<Node> >(pointer)
                                })
                            },
                            _ => unsafe { ::core::hint::unreachable_unchecked() },
                        }
                    }
                }
            };
        };
        let code = written(&derive_atom(quote! { enum Next { End, Node(NonNull<Node>) } }));
        assert_eq!(code, expected.to_string(), "the impl, its discriminants and its layout");
    }

    #[test]
    fn each_shape_names_atomiks_by_the_path_stated_alone() {
        let shapes = [
            quote! { struct Seq(u64); },
            quote! { struct Wrap<T>(T); },
            quote! { struct Marker; },
            quote! { struct Tag<K>(PhantomData<K>); },
            quote! { enum Side { Bid, Ask } },
            quote! { #[atom(repr = u16)] struct Step { length: u8, sign: Sign } },
            quote! { #[atom(repr = u64)] struct Pair<A, B> { first: A, second: B } },
            quote! { enum Slot { Empty, Writing { lap: u32 }, Ready(u32) } },
            quote! { #[atom(repr = u64)] #[repr(u8)] enum Lock<O> { Free = 0, Owned(O) = 1 } },
            quote! { struct Link { next: NonNull<Node>, deleted: bool } },
            quote! { struct Tagged<P> { #[atom(ptr)] pointer: P, flag: bool } },
            quote! { enum Slot { Empty, Inline(u32), Node(NonNull<Node>) } },
            quote! { enum Either<A, B> { Left(#[atom(ptr)] A), Right(#[atom(ptr)] B) } },
        ];
        for shape in shapes {
            let code = written(&derive_atom(quote! { #[atom(crate = renamed)] #shape }));
            assert!(code.contains("renamed :: Atom for"), "the impl names it: {code}");
            assert!(!code.contains(":: atomiks"), "and nothing else does: {code}");
        }
        for capability in [Capability::Add, Capability::Ord, Capability::Bitwise] {
            for shape in [quote! { struct Seq(u64); }, quote! { struct Wrap<T>(T); }] {
                let definition = quote! { #[atom(crate = renamed)] #shape };
                let code = written(&expand_capability(definition, capability));
                assert!(!code.contains(":: atomiks"), "a capability's impl alike: {code}");
            }
        }
    }

    #[test]
    fn a_refused_type_gets_a_stub_of_atom_and_nothing_else() {
        let unknown_key = quote! { #[atom(width = 8)] struct Seq(u64); };
        let (_, code) = refused(derive_atom(unknown_key.clone()));
        assert!(code.contains("const unsafe impl :: atomiks :: Atom for Seq"), "a stub: {code}");
        let (_, code) = refused(expand_capability(unknown_key, Capability::Add));
        assert_eq!(code, "", "no capability");
        let (_, code) = refused(derive_atom(quote! { fn seq() {} }));
        assert_eq!(code, "", "nor a stub where nothing names a type");
    }

    #[test]
    fn a_capability_is_refused_for_a_type_that_is_not_a_newtype() {
        let pair = expand_capability(quote! { struct Pair(u32, u32); }, Capability::Bitwise);
        let (messages, code) = refused(pair);
        assert_eq!(messages, ["`AtomBitwise` derives only for a newtype"], "why");
        assert_eq!(code, "", "with no impl");
    }
}
