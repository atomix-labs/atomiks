//! A packed struct's projection: each field's `HasPackedField`, which makes its path a
//! `FieldPath`; `QuoteFields<'a, P>`, a struct of one field place per field, each of the field's
//! own visibility; and `ProjectFields`, through which `fields()` lends it.

use proc_macro2::{Delimiter, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::ext::IdentExt;
use syn::{GenericParam, Ident, Lifetime, Member, Type};

use super::bound::{own_where_clause, thread_bounds, where_clause};
use super::field::PackedFields;
use super::layout::LayoutCode;
use crate::model::{Field, Implementor};

/// The code of a packed struct's projection, which names its parameters, `'a` and `P`, and its
/// locals at the derive's definition site.
pub(super) struct ProjectionCode<'a> {
    /// The struct projected.
    implementor: &'a Implementor,
    /// Its fields, in declaration order.
    fields: &'a [Field],
    /// The projection's name, `QuoteFields`, beside the struct's.
    name: Ident,
    /// The lifetime of the field places it holds: `'a`, or the first letter after it the struct
    /// does not name.
    lifetime: Lifetime,
    /// The path from an atomic to the struct the places lie in: `P`, or the first letter after it
    /// the struct does not name.
    path: Ident,
    /// The derive's definition site, where the locals it names take their names.
    def_site: Span,
}

impl<'a> ProjectionCode<'a> {
    /// The projection of `implementor`, a packed struct of `fields`, naming its parameters at
    /// `def_site`.
    ///
    /// The parameters take names the struct's own do not, though hygiene keeps them apart, so a
    /// page rustdoc writes tells them apart too.
    pub(super) fn new(implementor: &'a Implementor, fields: &'a [Field], def_site: Span) -> Self {
        let ident = &implementor.ident;
        let params = &implementor.generics.params;
        let is_taken = |name: &str| {
            params.iter().any(|param| match param {
                GenericParam::Lifetime(lifetime) => lifetime.lifetime.ident == name,
                GenericParam::Type(ty) => ty.ident == name,
                GenericParam::Const(constant) => constant.ident == name,
            })
        };
        let lifetime = first_untaken('a'..='z', is_taken).unwrap_or('a');
        let path = first_untaken('P'..='Z', is_taken).unwrap_or('P');
        Self {
            implementor,
            fields,
            name: format_ident!("{ident}Fields", span = ident.span()),
            lifetime: Lifetime::new(&format!("'{lifetime}"), def_site),
            path: Ident::new(&path.to_string(), def_site),
            def_site,
        }
    }

    /// Each field's `HasPackedField`, where `packed` places it and `layout` lays the struct out,
    /// the struct's `ProjectFields` and the projection's `Debug`: the impls in the block beside
    /// its `Atom`.
    ///
    /// A concrete struct's placements and layout are constants, and a field that ends at the
    /// repr's top bit reaches it; a generic struct's are what each instance lays out, which no
    /// constant knows, so no field reaches the top.
    pub(super) fn implementations(
        &self, packed: &PackedFields<'_>, layout: &LayoutCode<'_>,
    ) -> TokenStream {
        let Implementor { generics, atomiks, .. } = self.implementor;
        let (impl_generics, _, _) = generics.split_for_impl();
        let instance = self.instance();
        let where_clause = self.where_clause();
        let (name, alias) = (layout.name(), layout.repr_alias());
        let is_generic = layout.instance_repr().is_some();
        let field_implementations = self.fields.iter().zip(packed.placements()).enumerate().map(
            |(index, (Field { ty, member, .. }, placement))| {
                let (placement, layout, reach) = if is_generic {
                    (
                        layout.instance_local(placement),
                        layout.instance_local(name),
                        quote!(#atomiks::__private::Reach<false>),
                    )
                } else {
                    let reaches_top =
                        quote!(#atomiks::__private::reaches_top::<#alias>(#placement));
                    (
                        quote!(#placement),
                        quote!(#name),
                        quote!(#atomiks::__private::Reach<{ #reaches_top }>),
                    )
                };
                let index = field_index(index);
                quote! {
                    #[automatically_derived]
                    unsafe impl #impl_generics #atomiks::__private::HasPackedField<#index, #ty>
                        for #instance #where_clause
                    {
                        const PLACEMENT: #atomiks::__private::PackedField = #placement;
                        const LAYOUT: #atomiks::__private::PackedLayout = #layout;
                        type Reach = #reach;
                        #[inline]
                        fn field(self) -> #ty {
                            self.#member
                        }
                    }
                }
            },
        );
        let (project_fields, debug) =
            (self.project_fields_implementation(), self.debug_implementation());
        // Each `HasPackedField` keeps its promise: its type parameter is the field's type, and its
        // placement is `PackedField::new` of that type's reprs, at the offset where the field
        // before it ends, where `to_repr` packs the field, each field in bits of its own; the
        // layout is the struct's, whose extension `to_repr` writes above the width; `from_repr`
        // decodes each field alone and checks only the bits above the width, so a repr decodes
        // wherever each field's bits decode and the bits above extend the layout; `Reach` is
        // `Reach<true>` only where `reaches_top` finds the field ending at the repr's top bit, and
        // never for a generic struct; and `field` returns the field. `ProjectFields` keeps its own:
        // `project` lends the place of each field, at its path from `place`, into the
        // projection's field of that field's visibility, and no other place.
        quote!(#(#field_implementations)* #project_fields #debug)
    }

    /// The projection, `QuoteFields<'a, P>`, of one `&'a AtomicField` per field, each with the
    /// field's visibility and docs, and the struct's `#[non_exhaustive]` and `#[doc(hidden)]`: the
    /// item beside the struct.
    pub(super) fn structure(&self) -> TokenStream {
        let Implementor { vis, ident, atomiks, projection_attributes, .. } = self.implementor;
        let (name, lifetime, path) = (&self.name, &self.lifetime, &self.path);
        let (parameters, instance) = (self.parameters(), self.instance());
        let where_clause = self.own_where_clause(&[]);
        let places = self.fields.iter().enumerate().map(|(index, field)| {
            let Field { member, vis, docs, ty, .. } = field;
            let line = format!(
                " The field `{}`, of type `{}`, in an atomic `{ident}`.",
                member_shown(member),
                shown(ty)
            );
            let gap = (!docs.is_empty()).then(|| quote!(#[doc = ""]));
            let field_path = self.field_path(index, ty);
            let place = quote!(&#lifetime #atomiks::AtomicField<#field_path>);
            match member {
                Member::Named(named) => quote!(#(#docs)* #gap #[doc = #line] #vis #named: #place),
                Member::Unnamed(_) => quote!(#(#docs)* #gap #[doc = #line] #vis #place),
            }
        });
        let body = if self.is_tuple() {
            quote!((#(#places),*) #where_clause;)
        } else {
            quote!(#where_clause { #(#places),* })
        };
        let atomiks_shown = shown(atomiks);
        let doc = format!(
            " The fields of an atomic [`{ident}`], or of a field whose value is one, each a place of \
             its own: what [`fields()`]({}::Atomic::fields) lends.",
            atomiks_shown.trim_start_matches("::")
        );
        quote! {
            #[doc = #doc]
            #(#projection_attributes)*
            #[derive(::core::clone::Clone, ::core::marker::Copy)]
            #vis struct #name<
                #lifetime,
                #(#parameters,)*
                #path: #atomiks::FieldPath<Value = #instance> + #lifetime
            >
            #body
        }
    }

    /// The projection's `Debug`, wherever each field place has one: each place's value, as its
    /// own Relaxed load reads it.
    fn debug_implementation(&self) -> TokenStream {
        let Implementor { atomiks, .. } = self.implementor;
        let (name, lifetime, path) = (&self.name, &self.lifetime, &self.path);
        let (parameters, arguments, instance) =
            (self.parameters(), self.arguments(), self.instance());
        let places: Vec<TokenStream> = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, Field { ty, .. })| {
                let field_path = self.field_path(index, ty);
                quote!(&#lifetime #atomiks::AtomicField<#field_path>: ::core::fmt::Debug)
            })
            .collect();
        let where_clause = self.own_where_clause(&places);
        let formatter = Ident::new("formatter", self.def_site);
        let shown_name = name.to_string();
        let (start, fields) = if self.is_tuple() {
            let fields =
                self.fields.iter().map(|Field { member, .. }| quote!(.field(&self.#member)));
            (quote!(debug_tuple(#shown_name)), fields.collect::<Vec<_>>())
        } else {
            let fields = self.fields.iter().map(|Field { member, .. }| {
                let shown = member_shown(member);
                quote!(.field(#shown, &self.#member))
            });
            (quote!(debug_struct(#shown_name)), fields.collect())
        };
        quote! {
            #[automatically_derived]
            impl<
                #lifetime,
                #(#parameters,)*
                #path: #atomiks::FieldPath<Value = #instance> + #lifetime
            > ::core::fmt::Debug for #name<#lifetime, #(#arguments,)* #path>
            #where_clause
            {
                fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    #formatter.#start #(#fields)*.finish()
                }
            }
        }
    }

    /// The struct's `ProjectFields`, whose `Fields` is the projection, which `project` builds of
    /// the place of each field's path.
    fn project_fields_implementation(&self) -> TokenStream {
        let Implementor { generics, atomiks, .. } = self.implementor;
        let (impl_generics, _, _) = generics.split_for_impl();
        let (name, lifetime, path) = (&self.name, &self.lifetime, &self.path);
        let (arguments, instance) = (self.arguments(), self.instance());
        let where_clause = self.where_clause();
        let place = Ident::new("place", self.def_site);
        // Each call keeps its contract: the projection's field that holds the place takes the
        // field's visibility, so the place reaches only code the field is visible to.
        let field_place = quote!(unsafe { #atomiks::__private::project_field(#place) });
        let fields = self.fields.iter().map(|Field { member, .. }| match member {
            Member::Named(named) => quote!(#named: #field_place),
            Member::Unnamed(_) => field_place.clone(),
        });
        let built = if self.is_tuple() {
            quote!(#name(#(#fields),*))
        } else {
            quote!(#name { #(#fields),* })
        };
        // Each type parameter outlives `'a` where the struct does, as the projection's places
        // need: written out, since a generic associated type's bounds are not inferred.
        let outlives = generics.params.iter().filter_map(|param| match param {
            GenericParam::Type(ty) => {
                let ident = &ty.ident;
                Some(quote!(#ident: #lifetime))
            },
            GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        });
        quote! {
            #[automatically_derived]
            const unsafe impl #impl_generics #atomiks::ProjectFields for #instance #where_clause {
                type Fields<#lifetime, #path: #atomiks::FieldPath<Value = Self> + #lifetime>
                    = #name<#lifetime, #(#arguments,)* #path>
                where
                    Self: #lifetime #(, #outlives)*;
                #[inline]
                fn project<#path: #atomiks::FieldPath<Value = Self>>(
                    #place: &#atomiks::AtomicField<#path>,
                ) -> #name<'_, #(#arguments,)* #path> {
                    #built
                }
            }
        }
    }

    /// The type the struct is, with its parameters: `Quote`, `Tagged<T>`.
    fn instance(&self) -> TokenStream {
        let Implementor { ident, generics, .. } = self.implementor;
        let (_, ty_generics, _) = generics.split_for_impl();
        quote!(#ident #ty_generics)
    }

    /// The where clause of an impl for the struct: its `Atom` impl's, as a run-time impl bounds
    /// it, so the struct is an `Atom` wherever the impl applies.
    fn where_clause(&self) -> Option<TokenStream> {
        let atomiks = &self.implementor.atomiks;
        let field_bounds = self.fields.iter().filter(|field| field.is_generic).map(
            |Field { ty, .. }| quote!(#ty: #atomiks::Atom<Repr: #atomiks::__private::FieldRepr>),
        );
        let thread_bounds = thread_bounds(self.implementor, self.fields);
        where_clause(self.implementor, field_bounds.chain(thread_bounds))
    }

    /// The where clause of the projection and of its `Debug`, which name no `Self`: the struct's
    /// own predicates, then, for a generic struct, each field's `HasPackedField`, and `Atom` of
    /// each field that names a parameter, which make its path a `FieldPath`, then `predicates`;
    /// none where that is nothing.
    fn own_where_clause(&self, predicates: &[TokenStream]) -> Option<TokenStream> {
        let Implementor { generics, atomiks, .. } = self.implementor;
        let instance = self.instance();
        let packed = (!generics.params.is_empty()).then(|| {
            let bounds = self.fields.iter().enumerate().map(|(index, Field { ty, .. })| {
                let index = field_index(index);
                quote!(#atomiks::__private::HasPackedField<#index, #ty>)
            });
            quote!(#instance: #(#bounds)+*)
        });
        let values = self
            .fields
            .iter()
            .filter(|field| field.is_generic)
            .map(|Field { ty, .. }| quote!(#ty: #atomiks::Atom));
        let predicates = packed.into_iter().chain(values).chain(predicates.iter().cloned());
        own_where_clause(self.implementor, predicates)
    }

    /// The path to the field at `index`, a `ty`, from the path `P`: `Join<P, Field<Quote, 2,
    /// bool>>`.
    fn field_path(&self, index: usize, ty: &Type) -> TokenStream {
        let atomiks = &self.implementor.atomiks;
        let (path, instance) = (&self.path, self.instance());
        let index = field_index(index);
        quote!(#atomiks::Join<#path, #atomiks::Field<#instance, #index, #ty>>)
    }

    /// The struct's parameters, without the angle brackets, with their bounds but not their
    /// defaults, which may neither come before `P` nor stand in an impl: `T: Copy`.
    fn parameters(&self) -> Vec<TokenStream> {
        let mut generics = self.implementor.generics.clone();
        for param in &mut generics.params {
            match param {
                GenericParam::Type(ty) => ty.default = None,
                GenericParam::Const(constant) => constant.default = None,
                GenericParam::Lifetime(_) => {},
            }
        }
        generics.params.iter().map(ToTokens::to_token_stream).collect()
    }

    /// The struct's arguments, without the angle brackets: `T`, `'b`, `N`.
    fn arguments(&self) -> Vec<TokenStream> {
        self.implementor
            .generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Lifetime(lifetime) => lifetime.lifetime.to_token_stream(),
                GenericParam::Type(ty) => ty.ident.to_token_stream(),
                GenericParam::Const(constant) => constant.ident.to_token_stream(),
            })
            .collect()
    }

    /// Whether the struct's fields are unnamed, so that its projection's are too.
    const fn is_tuple(&self) -> bool {
        matches!(self.fields.first(), Some(Field { member: Member::Unnamed(_), .. }))
    }
}

/// The first of `letters` whose name `is_taken` finds free.
fn first_untaken<I: IntoIterator<Item = char>>(
    letters: I, is_taken: impl Fn(&str) -> bool,
) -> Option<char> {
    letters.into_iter().find(|letter| !is_taken(&letter.to_string()))
}

/// The field at `index`, in declaration order, as `Field`'s `u32` index.
fn field_index(index: usize) -> Literal {
    Literal::u32_unsuffixed(u32::try_from(index).unwrap_or(u32::MAX))
}

/// A field's name, or its index in a tuple struct, as a doc shows it: `type` for `r#type`.
fn member_shown(member: &Member) -> String {
    match member {
        Member::Named(named) => named.unraw().to_string(),
        Member::Unnamed(unnamed) => unnamed.index.to_string(),
    }
}

/// `tokens` as source writes them, for a doc: `RangedI8<-5, 5>`, not `RangedI8 < - 5 , 5 >`.
fn shown<T: ToTokens>(tokens: &T) -> String {
    let mut text = String::new();
    // Whether the token before joins the next without a space: an opening `<`, a path's `::`, a
    // reference's `&`, a lifetime's `'`, a sign.
    let mut joins = true;
    for token in tokens.to_token_stream() {
        let (token_text, punct) = match &token {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::Brace => ("{ ", " }"),
                    Delimiter::None => ("", ""),
                };
                (format!("{open}{}{close}", shown(&group.stream())), None)
            },
            TokenTree::Punct(punct) => (punct.to_string(), Some(punct)),
            TokenTree::Ident(_) | TokenTree::Literal(_) => (token.to_string(), None),
        };
        let character = punct.map(Punct::as_char);
        if !joins && !matches!(character, Some(',' | ';' | '<' | '>' | ':')) {
            text.push(' ');
        }
        text.push_str(&token_text);
        joins = punct.is_some_and(|punct| punct.spacing() == Spacing::Joint)
            || matches!(character, Some('<' | ':' | '&' | '-'));
    }
    text
}
