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
    match parse::input(input) {
        Ok(Input { implementor, shape: Ok(Shape::Newtype(newtype)) }) => {
            Expansion::written(code::atom(&implementor, &newtype, def_site))
        },
        Ok(Input { implementor, shape: Ok(shape) }) => Expansion::refused(
            vec![not_yet_supported(&implementor, &shape)],
            code::stub(&implementor),
        ),
        Ok(Input { implementor, shape: Err(errors) }) => {
            Expansion::refused(errors, code::stub(&implementor))
        },
        Err(errors) => Expansion::refused(errors, TokenStream::new()),
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
        "only a newtype derives it so far: a struct of one field, beside any `PhantomData` markers"
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
    fn a_shape_not_yet_supported_is_refused_beside_a_stub() {
        let stub = quote! {
            #[automatically_derived]
            const unsafe impl ::atomiks::Atom for Side
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
        let (messages, code) = refused(derive_atom(quote! { enum Side { Bid, Ask } }));
        assert_eq!(messages, ["deriving `Atom` for a fieldless enum is not yet supported"], "why");
        assert_eq!(code, stub.to_string(), "and the stub");
    }

    #[test]
    fn each_shape_names_atomiks_by_the_path_stated_alone() {
        let shapes = [quote! { struct Seq(u64); }, quote! { struct Wrap<T>(T); }];
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
