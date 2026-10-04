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
        Ok(Shape::Fieldless(fieldless)) => {
            Expansion::written(code::fieldless(&implementor, &fieldless, def_site))
        },
        Ok(shape @ Shape::EnumWithFields) => Expansion::refused(
            vec![not_yet_supported(&implementor, &shape)],
            code::stub(&implementor),
        ),
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

/// The refusal of a shape the derive cannot yet write `Atom` for.
fn not_yet_supported(implementor: &Implementor, shape: &Shape) -> DeriveError {
    DeriveError::new(
        implementor.ident.span(),
        format!("deriving `Atom` for {} is not yet supported", shape.noun()),
    )
    .note(
        None,
        "so far it derives for a newtype, a zero-width struct, a struct of several fields and a fieldless enum"
            .to_owned(),
    )
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
                #[inline]
                fn to_repr(self) -> <u64 as ::atomiks::Atom>::Repr {
                    ::atomiks::__private::to_repr::<u64>(self.0)
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
                #[inline]
                fn to_repr(self) -> <T as ::atomiks::Atom>::Repr {
                    <T as ::atomiks::Atom>::to_repr(self.0)
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
    fn a_packed_struct_places_each_field_where_the_one_before_it_ends() {
        let expected = quote! {
            const _: () = ::atomiks::__private::assert_send_and_sync::<Step>();
            const _: () = {
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
        };
        let step = derive_atom(quote! { struct Step { length: u8, sign: Sign } });
        assert_eq!(written(&step), expected.to_string(), "the impl");
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
            derive_atom(quote! { #[atom(repr = u32)] struct Quote { qty: u8, live: bool } });
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
                [::atomiks::__private::PackedField; 3], ::atomiks::__private::PackedLayout
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
                ([placement_0, placement_1, placement_2], layout)
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
            let ([placement_0, placement_1, placement_2], layout) =
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
    fn a_shape_not_yet_supported_is_refused_beside_a_stub() {
        let stub = quote! {
            #[automatically_derived]
            const unsafe impl ::atomiks::Atom for Slot
            where
                Self: ::core::marker::Copy,
            {
                type Repr = ::core::primitive::u8;
                const REPRS: ::atomiks::ReprRange<::core::primitive::u8> = ::atomiks::ReprRange::new(0, 0);
                #[inline]
                fn to_repr(self) -> ::core::primitive::u8 {
                    0
                }
                #[inline]
                fn from_repr(_: ::core::primitive::u8) -> ::core::option::Option<Self> {
                    ::core::panic!("`#[derive(Atom)]` refused this type")
                }
            }
        };
        let payload = derive_atom(quote! { enum Slot { Empty, Full(u32) } });
        let (messages, code) = refused(payload);
        assert_eq!(
            messages,
            ["deriving `Atom` for an enum with fields is not yet supported"],
            "why"
        );
        assert_eq!(code, stub.to_string(), "and the stub");
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
