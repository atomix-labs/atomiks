//! The code of a value laid out over packed fields: the locals that make its layout, constants
//! beside its impl where the type has no parameters, else what a function evaluates for each
//! instance, checked against the repr the type states; and the impl that reaches them.

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{Ident, Path};

use super::bound::{own_where_clause, thread_bounds, thread_checks, where_clause};
use super::repr;
use crate::model::{Field, Implementor};

/// The code of a value's layout: each local that makes it, the layout itself, `layout`, last, and
/// the names of the items beside them, each named at the derive's definition site.
pub(super) struct LayoutCode<'a> {
    /// The type laid out.
    implementor: &'a Implementor,
    /// Each local, as its name, its type and its value, in the order each is computed.
    locals: Vec<(Ident, TokenStream, TokenStream)>,
    /// The layout's name, `layout`.
    name: Ident,
    /// The repr's alias, `Repr`.
    repr_alias: Ident,
    /// The name of the function that lays an instance out, `lay_out`.
    lay_out: Ident,
}

impl<'a> LayoutCode<'a> {
    /// The code of `implementor`'s layout, of no locals yet, naming its items at `def_site`.
    pub(super) fn new(implementor: &'a Implementor, def_site: Span) -> Self {
        let name = |name| Ident::new(name, def_site);
        Self {
            implementor,
            locals: Vec::new(),
            name: name("layout"),
            repr_alias: name("Repr"),
            lay_out: name("lay_out"),
        }
    }

    /// The path to atomiks.
    pub(super) const fn atomiks(&self) -> &'a Path {
        &self.implementor.atomiks
    }

    /// The layout's name, the last local's.
    pub(super) const fn name(&self) -> &Ident {
        &self.name
    }

    /// The repr's alias.
    pub(super) const fn repr_alias(&self) -> &Ident {
        &self.repr_alias
    }

    /// Adds the local `name`, of type `ty`, whose value is `value`, after the others.
    pub(super) fn push(&mut self, name: &Ident, ty: TokenStream, value: TokenStream) {
        self.locals.push((name.clone(), ty, value));
    }

    /// The repr the type states where it has parameters, so that each instance lays itself out in
    /// it; `None` where the type has none, so that its locals are constants.
    ///
    /// Parse refuses a type with parameters that states no repr: no constant names an instance's
    /// layout, so no repr could be selected from it.
    pub(super) fn instance_repr(&self) -> Option<&'a Ident> {
        self.implementor.repr.as_ref().filter(|_| !self.implementor.generics.params.is_empty())
    }

    /// The validity a concrete impl selects by `code`, a constant expression of its number.
    pub(super) fn selected_validity(&self, code: &TokenStream) -> TokenStream {
        let atomiks = &self.implementor.atomiks;
        quote! {
            <#atomiks::__private::ValidityCode<{ #code }> as #atomiks::__private::SelectValidity>
                ::Validity
        }
    }

    /// What starts each conversion: nothing where the locals are constants, else each local bound
    /// from the instance's layout, those named in `unused` as `_`.
    pub(super) fn bind(&self, unused: &[&Ident]) -> Option<TokenStream> {
        self.instance_repr().map(|_| {
            let names = self
                .locals
                .iter()
                .map(|(name, ..)| if unused.contains(&name) { quote!(_) } else { quote!(#name) });
            let laid_out = self.laid_out();
            quote!(let (#(#names,)*) = const { #laid_out };)
        })
    }

    /// `Atom` for the type, of `validity`, `conversions` and the layout's range, in a block beside
    /// `items` and the layout.
    ///
    /// A concrete type's locals are constants, and its repr the narrowest that holds the layout,
    /// or the one stated; a generic type's are what a function evaluates for each instance, which
    /// refuses one wider than the repr stated. The type is checked or bounded `Send` and `Sync`, as
    /// `thread_checks` and `thread_bounds` say, and each generic one of `fields` bounded `Atom` of
    /// a repr it packs as. The block names its items at the derive's definition site, so that
    /// the user's code reaches none of them.
    pub(super) fn implement<'f, F>(
        &self, items: &TokenStream, fields: F, validity: &TokenStream, conversions: &TokenStream,
    ) -> TokenStream
    where
        F: IntoIterator<Item = &'f Field, IntoIter: Clone>,
    {
        let Implementor { ident, atomiks, repr: stated, .. } = self.implementor;
        let (alias, layout) = (&self.repr_alias, &self.name);
        let fields = fields.into_iter();
        let associated = quote! {
            type Repr = #alias;
            type Validity = #validity;
        };
        if let Some(stated) = self.instance_repr() {
            return self.implement_generic(items, fields, stated, &associated, conversions);
        }
        let width = quote!(#layout.width());
        let thread_checks = thread_checks(self.implementor, fields);
        let locals = self.locals.iter().map(|(name, ty, value)| quote!(const #name: #ty = #value;));
        let (repr_type, repr_checks) = stated.as_ref().map_or_else(
            || (repr::narrowest(atomiks, &width), repr::width_check(atomiks, ident, &width)),
            |stated| {
                let integer_check = repr::integer_check(self.implementor, stated);
                let integer = quote!(::core::primitive::#stated);
                let width_check =
                    repr::stated_width_check(atomiks, ident, &integer, stated, &width);
                (repr::selected_from(atomiks, stated), quote!(#integer_check #width_check))
            },
        );
        let where_clause = where_clause(self.implementor, []);
        quote! {
            #thread_checks
            const _: () = {
                #items
                #(#locals)*
                type #alias = #repr_type;
                #repr_checks
                #[automatically_derived]
                const unsafe impl #atomiks::Atom for #ident #where_clause {
                    #associated
                    const REPRS: #atomiks::ReprRange<#alias> = #layout.range();
                    #conversions
                }
            };
        }
    }

    /// `Atom` for the generic type, as [`implement`](Self::implement) writes it, in the repr
    /// `stated`, of `associated`, its repr and validity.
    fn implement_generic<'f, F>(
        &self, items: &TokenStream, fields: F, stated: &Ident, associated: &TokenStream,
        conversions: &TokenStream,
    ) -> TokenStream
    where
        F: Iterator<Item = &'f Field> + Clone,
    {
        let Implementor { ident, generics, atomiks, .. } = self.implementor;
        let (alias, layout) = (&self.repr_alias, &self.name);
        let (impl_generics, ty_generics, _) = generics.split_for_impl();
        let instance = quote!(#ident #ty_generics);
        let repr_type = repr::selected_from(atomiks, stated);
        let integer_check = repr::integer_check(self.implementor, stated);
        let width_assertion = repr::stated_width_assertion(
            atomiks,
            &instance,
            &quote!(::core::primitive::#stated),
            stated,
            &quote!(#layout.width()),
        );
        let names = self.locals.iter().map(|(name, ..)| name);
        let types = self.locals.iter().map(|(_, ty, _)| ty);
        let values = self.locals.iter().map(|(name, _, value)| quote!(let #name = #value;));
        let generic_fields = fields.clone().filter(|field| field.is_generic);
        let layout_where = own_where_clause(
            self.implementor,
            generic_fields.clone().map(|Field { ty, .. }| quote!(#ty: #atomiks::Atom)),
        );
        let field_bounds = generic_fields.map(|Field { ty, .. }| {
            quote!(#ty: [const] #atomiks::Atom<Repr: [const] #atomiks::__private::FieldRepr>)
        });
        let thread_checks = thread_checks(self.implementor, fields.clone());
        let where_clause = where_clause(
            self.implementor,
            field_bounds.chain(thread_bounds(self.implementor, fields)),
        );
        // The layout is the last local, and the last value `lay_out` returns.
        let index = Literal::usize_unsuffixed(self.locals.len().saturating_sub(1));
        let (function, laid_out) = (&self.lay_out, self.laid_out());
        quote! {
            #thread_checks
            const _: () = {
                type #alias = #repr_type;
                #integer_check
                #items
                const fn #function #impl_generics(_: ::core::marker::PhantomData<#instance>)
                    -> (#(#types,)*)
                #layout_where
                {
                    #(#values)*
                    #width_assertion;
                    (#(#names,)*)
                }
                #[automatically_derived]
                const unsafe impl #impl_generics #atomiks::Atom for #instance #where_clause {
                    #associated
                    const REPRS: #atomiks::ReprRange<#alias> = #laid_out.#index.range();
                    #conversions
                }
            };
        }
    }

    /// The local `name` of the instance `Self` is, as the function that lays it out computes it.
    ///
    /// # Panics
    /// `name` is no local pushed before: a fault of the derive's own.
    #[track_caller]
    #[expect(clippy::expect_used, reason = "the derive names only the locals it pushed")]
    pub(super) fn instance_local(&self, name: &Ident) -> TokenStream {
        let laid_out = self.laid_out();
        let index = self
            .locals
            .iter()
            .position(|(local, ..)| local == name)
            .expect("the derive names a local it pushed");
        let index = Literal::usize_unsuffixed(index);
        quote!(#laid_out.#index)
    }

    /// The call that lays out the instance `Self` is.
    fn laid_out(&self) -> TokenStream {
        let lay_out = &self.lay_out;
        quote!(#lay_out(::core::marker::PhantomData::<Self>))
    }
}
