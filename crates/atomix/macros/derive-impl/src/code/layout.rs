//! The code of a value laid out over packed fields: the locals that make its layout, constants
//! beside its impl where every instance is laid out alike, else what a function evaluates for each
//! instance, checked against the repr the type states; and the impl that reaches them.
//!
//! A value stored as a pointer has one more local, its pointees' alignment, which its code and its
//! checks read but no type's layout, so the value may be held in its own pointee: a generic one's
//! is what a second function evaluates, which runs the checks.

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{Ident, Path, Type};

use super::bound::{
    BoundSite, field_bounds, own_where_clause, thread_bounds, thread_checks, where_clause,
};
use super::repr;
use crate::model::{Field, Implementor};

/// The code of a value's layout: each local that makes it, the layout itself, `layout`, last, and
/// the names of the items beside them, each named at the derive's definition site.
pub(super) struct LayoutCode<'a> {
    /// The type laid out.
    implementor: &'a Implementor,
    /// Each local, as its name, its type and its value, in the order each is computed.
    locals: Vec<(Ident, TokenStream, TokenStream)>,
    /// The alignment of what a value stored as a pointer points to, the least of its pointer
    /// fields': a local after the others, `alignment`, which no type's layout reads. `None` for a
    /// value stored as bits.
    alignment: Option<TokenStream>,
    /// The layout's name, `layout`.
    name: Ident,
    /// The alignment's name, `alignment`.
    alignment_name: Ident,
    /// The repr's alias, `Repr`.
    repr_alias: Ident,
    /// The name of the function that lays an instance out, `lay_out`.
    lay_out: Ident,
    /// The name of the function that lays an instance stored as a pointer out for its code, its
    /// alignment beside, after checking it, `lay_out_checked`.
    lay_out_checked: Ident,
    /// Whether each instance lays itself out: where the type has parameters, unless laid out once.
    is_laid_out_per_instance: bool,
}

/// What an `Atom` impl holds beside its layout's locals.
pub(super) struct AtomImpl {
    /// The items in the block beside the impl: its discriminants, or its projection's impls.
    pub(super) items: TokenStream,
    /// Its repr.
    pub(super) repr: ImplRepr,
    /// Its validity, a type.
    pub(super) validity: TokenStream,
    /// Its range, an expression of the locals as [`LayoutCode::local`] names them.
    pub(super) reprs: TokenStream,
    /// Each check of the layout, an expression of the locals, located where its refusal points.
    ///
    /// It is a constant beside the impl of a type laid out once, a statement of `lay_out` for one
    /// laid out per instance, or, for a value stored as a pointer, whose checks read the
    /// alignment, of its `lay_out_checked`.
    pub(super) checks: Vec<TokenStream>,
    /// `to_repr` and the rest of its conversions.
    pub(super) conversions: TokenStream,
}

/// The repr an `Atom` impl names.
pub(super) enum ImplRepr {
    /// The narrowest integer that holds the layout, or the one the type states: a packed struct's
    /// or an enum with fields'.
    Integer,
    /// The type a pointer is stored as: a pointer word's pointer field's repr, or a pointer enum's
    /// `*mut ()`.
    Pointer {
        /// The type.
        ty: TokenStream,
        /// How many low bits the value's tags take, an expression of the locals.
        tag_width: TokenStream,
    },
}

impl<'a> LayoutCode<'a> {
    /// The code of `implementor`'s layout, of no locals yet, naming its items at `def_site`.
    pub(super) fn new(implementor: &'a Implementor, def_site: Span) -> Self {
        let name = |name| Ident::new(name, def_site);
        Self {
            implementor,
            locals: Vec::new(),
            alignment: None,
            name: name("layout"),
            alignment_name: name("alignment"),
            repr_alias: name("Repr"),
            lay_out: name("lay_out"),
            lay_out_checked: name("lay_out_checked"),
            is_laid_out_per_instance: !implementor.generics.params.is_empty(),
        }
    }

    /// The layout of a type whose parameters reach markers alone: laid out once, in constants.
    pub(super) fn laid_out_once(self) -> Self {
        Self { is_laid_out_per_instance: false, ..self }
    }

    /// The layout of a value stored as a pointer, whose pointer fields are of `pointers`: its
    /// alignment is the least of theirs.
    pub(super) fn stored_as_pointers<'t, I: IntoIterator<Item = &'t Type>>(
        self, pointers: I,
    ) -> Self {
        let atomix = self.atomix();
        let alignments =
            pointers.into_iter().map(|ty| quote!(<#ty as #atomix::Atom>::POINTEE_ALIGNMENT));
        let alignment = quote!(#atomix::__private::PointeeAlignment::least(&[#(#alignments),*]));
        Self { alignment: Some(alignment), ..self }
    }

    /// The path to atomix.
    pub(super) const fn atomix(&self) -> &'a Path {
        &self.implementor.atomix
    }

    /// The layout's name, the last local's.
    pub(super) const fn name(&self) -> &Ident {
        &self.name
    }

    /// The repr's alias.
    pub(super) const fn repr_alias(&self) -> &Ident {
        &self.repr_alias
    }

    /// The alignment's name, which a value stored as a pointer's conversions and checks read.
    pub(super) const fn alignment_name(&self) -> &Ident {
        &self.alignment_name
    }

    /// Adds the local `name`, of type `ty`, whose value is `value`, after the others.
    pub(super) fn push(&mut self, name: &Ident, ty: TokenStream, value: TokenStream) {
        self.locals.push((name.clone(), ty, value));
    }

    /// The repr the type states where each instance lays itself out in it; `None` where every
    /// instance is laid out alike, so that its locals are constants.
    ///
    /// Parse refuses a packed struct or an enum with fields laid out per instance that states no
    /// repr: no constant names an instance's layout, so no repr could be selected from it.
    pub(super) fn instance_repr(&self) -> Option<&'a Ident> {
        self.implementor.repr.as_ref().filter(|_| self.is_laid_out_per_instance())
    }

    /// Whether its locals are what `lay_out` computes for each instance, rather than constants.
    pub(super) const fn is_laid_out_per_instance(&self) -> bool {
        self.is_laid_out_per_instance
    }

    /// Each local as a constant beside the impl, for a type laid out once.
    pub(super) fn constants(&self) -> TokenStream {
        let constants =
            self.locals.iter().map(|(name, ty, value)| quote!(const #name: #ty = #value;));
        quote!(#(#constants)*)
    }

    /// `lay_out`, the function that computes each local of the instance its argument
    /// names, in turn, then runs `checks` on them and returns them all; bounded by the
    /// type's own predicates and `predicates`.
    pub(super) fn lay_out_function<I: IntoIterator<Item = TokenStream>>(
        &self, checks: &TokenStream, predicates: I,
    ) -> TokenStream {
        let Implementor { ident, generics, .. } = self.implementor;
        let (impl_generics, ty_generics, _) = generics.split_for_impl();
        let names = self.locals.iter().map(|(name, ..)| name);
        let types = self.locals.iter().map(|(_, ty, _)| ty);
        let values = self.locals.iter().map(|(name, _, value)| quote!(let #name = #value;));
        let where_clause = own_where_clause(self.implementor, predicates);
        let function = &self.lay_out;
        quote! {
            const fn #function #impl_generics(_: ::core::marker::PhantomData<#ident #ty_generics>)
                -> (#(#types,)*)
            #where_clause
            {
                #(#values)*
                #checks
                (#(#names,)*)
            }
        }
    }

    /// The functions that lay an instance out, each bounded by the type's own predicates and
    /// `predicates`: `lay_out`, which runs `checks`; or, for a value stored as a pointer, whose
    /// checks read its alignment, `lay_out`, which a type's layout may read and which checks
    /// nothing, and `lay_out_checked`, which runs them.
    fn lay_out_functions<P, I>(&self, checks: &[TokenStream], predicates: P) -> TokenStream
    where
        P: Fn() -> I,
        I: IntoIterator<Item = TokenStream>,
    {
        let Some(alignment) = &self.alignment else {
            return self.lay_out_function(&quote!(#(#checks;)*), predicates());
        };
        let unchecked = self.lay_out_function(&TokenStream::new(), predicates());
        let checked = self.lay_out_checked_function(alignment, checks, predicates());
        quote!(#unchecked #checked)
    }

    /// `lay_out_checked`, the function that takes the locals `lay_out` computes for the instance
    /// its argument names, adds `alignment`, runs `checks` on them and returns them all; bounded as
    /// `lay_out` is.
    ///
    /// Only the conversions call it, once the instance is laid out, so no type's layout reads the
    /// alignment.
    fn lay_out_checked_function<I: IntoIterator<Item = TokenStream>>(
        &self, alignment: &TokenStream, checks: &[TokenStream], predicates: I,
    ) -> TokenStream {
        let Implementor { ident, generics, atomix, .. } = self.implementor;
        let (impl_generics, ty_generics, _) = generics.split_for_impl();
        let names: Vec<&Ident> = self.locals.iter().map(|(name, ..)| name).collect();
        let types = self.locals.iter().map(|(_, ty, _)| ty);
        let where_clause = own_where_clause(self.implementor, predicates);
        let (lay_out, function, alignment_name) =
            (&self.lay_out, &self.lay_out_checked, &self.alignment_name);
        let instance = quote!(::core::marker::PhantomData<#ident #ty_generics>);
        quote! {
            const fn #function #impl_generics(_: #instance)
                -> (#(#types,)* #atomix::__private::PointeeAlignment,)
            #where_clause
            {
                let (#(#names,)*) = #lay_out(::core::marker::PhantomData::<#ident #ty_generics>);
                let #alignment_name = #alignment;
                #(#checks;)*
                (#(#names,)* #alignment_name,)
            }
        }
    }

    /// The validity a concrete impl selects by `code`, a constant expression of its number.
    pub(super) fn selected_validity(&self, code: &TokenStream) -> TokenStream {
        let atomix = &self.implementor.atomix;
        quote! {
            <#atomix::__private::ValidityCode<{ #code }> as #atomix::__private::SelectValidity>
                ::Validity
        }
    }

    /// What starts each conversion: nothing where the locals are constants, else each local bound
    /// from the instance's layout, the alignment too where it is stored as a pointer, those named
    /// in `unused` as `_`.
    pub(super) fn bind(&self, unused: &[&Ident]) -> Option<TokenStream> {
        self.is_laid_out_per_instance().then(|| {
            let alignment = self.alignment.is_some().then_some(&self.alignment_name);
            let names = self
                .locals
                .iter()
                .map(|(name, ..)| name)
                .chain(alignment)
                .map(|name| if unused.contains(&name) { quote!(_) } else { quote!(#name) });
            let function = if alignment.is_some() { &self.lay_out_checked } else { &self.lay_out };
            quote!(let (#(#names,)*) = const { #function(::core::marker::PhantomData::<Self>) };)
        })
    }

    /// `Atom` for the type, as `atom` says, in a block beside its items and the layout.
    ///
    /// The locals of a type laid out once are constants, each check a constant beside the impl,
    /// and its repr the narrowest that holds the layout, or the one stated, or its pointer's; those
    /// of a type laid out per instance are what a function evaluates for each instance, which runs
    /// each check, and refuses one wider than the repr stated. Such a value stored as a pointer
    /// runs its checks, which read its alignment, in a second function, which only its
    /// conversions call. The type is checked or bounded `Send` and `Sync`, as `thread_checks`
    /// and `thread_bounds` say, and each of `fields` laid out per instance bounded as
    /// `field_bounds` says. The block names its items at the derive's
    /// definition site, so that the user's code reaches none of them.
    pub(super) fn implement<'f, F>(&self, fields: F, atom: AtomImpl) -> TokenStream
    where
        F: IntoIterator<Item = &'f Field, IntoIter: Clone>,
    {
        let Implementor { ident, generics, atomix, .. } = self.implementor;
        let AtomImpl { items, repr, validity, reprs, mut checks, conversions } = atom;
        let (alias, layout) = (&self.repr_alias, &self.name);
        let fields = fields.into_iter();
        let thread_checks = thread_checks(self.implementor, fields.clone());
        let width = quote!(#layout.width());
        let (impl_generics, ty_generics, _) = generics.split_for_impl();
        let instance = quote!(#ident #ty_generics);
        let private = quote!(#atomix::__private);
        // A value stored as a pointer's tag width, and its alignment: a constant where it is laid
        // out once, or the expression the impl evaluates for each instance.
        let pointer_constants = |alignment: &TokenStream| match &repr {
            ImplRepr::Pointer { tag_width, .. } => Some(quote! {
                const TAG_WIDTH: ::core::primitive::u32 = #tag_width;
                const POINTEE_ALIGNMENT: #private::PointeeAlignment = #alignment;
            }),
            ImplRepr::Integer => None,
        };
        // The repr of a type laid out per instance: its pointer's, or the integer it states,
        // checked to hold each instance; parse refuses one laid out in an integer that states none.
        let per_instance_repr = match &repr {
            ImplRepr::Pointer { ty, .. } => {
                self.is_laid_out_per_instance().then(|| (ty.clone(), None))
            },
            ImplRepr::Integer => self.instance_repr().map(|stated| {
                let integer = quote!(::core::primitive::#stated);
                checks.push(repr::instance_stated_width_assertion(
                    atomix, &instance, &integer, stated, &width,
                ));
                let integer_check = repr::integer_check(self.implementor, stated);
                let selected = repr::selected_from(atomix, stated);
                (quote!(#alias), Some(quote!(type #alias = #selected; #integer_check)))
            }),
        };
        if let Some((repr_type, repr_items)) = per_instance_repr {
            let functions = self.lay_out_functions(&checks, || {
                field_bounds(atomix, fields.clone(), BoundSite::LayOut)
            });
            let pointer_constants = self.alignment.as_ref().and_then(pointer_constants);
            let field_bounds = field_bounds(atomix, fields.clone(), BoundSite::Impl);
            let where_clause = where_clause(
                self.implementor,
                field_bounds.chain(thread_bounds(self.implementor, fields)),
            );
            return quote! {
                #thread_checks
                const _: () = {
                    #repr_items
                    #items
                    #functions
                    #[automatically_derived]
                    const unsafe impl #impl_generics #atomix::Atom for #instance #where_clause {
                        type Repr = #repr_type;
                        type Validity = #validity;
                        const REPRS: #atomix::ReprRange<#repr_type> = #reprs;
                        #pointer_constants
                        #conversions
                    }
                };
            };
        }
        let alignment_name = &self.alignment_name;
        let alignment = self.alignment.as_ref().map(
            |alignment| quote!(const #alignment_name: #private::PointeeAlignment = #alignment;),
        );
        let pointer_constants = pointer_constants(&quote!(#alignment_name));
        let (repr_type, repr_checks) = self.constant_repr(repr, &width);
        let constants = self.constants();
        let where_clause = where_clause(self.implementor, thread_bounds(self.implementor, fields));
        quote! {
            #thread_checks
            const _: () = {
                #items
                #constants
                #alignment
                type #alias = #repr_type;
                #repr_checks
                #(const _: () = #checks;)*
                #[automatically_derived]
                const unsafe impl #impl_generics #atomix::Atom for #instance #where_clause {
                    type Repr = #alias;
                    type Validity = #validity;
                    const REPRS: #atomix::ReprRange<#alias> = #reprs;
                    #pointer_constants
                    #conversions
                }
            };
        }
    }

    /// The repr of a type laid out once, `width` bits wide, and the checks that refuse it where
    /// that repr cannot hold it: its pointer's, with none, or the narrowest integer that holds
    /// `width` bits, or the one stated.
    fn constant_repr(
        &self, repr: ImplRepr, width: &TokenStream,
    ) -> (TokenStream, Option<TokenStream>) {
        let Implementor { ident, atomix, repr: stated, .. } = self.implementor;
        match (repr, stated) {
            (ImplRepr::Pointer { ty, .. }, _) => (ty, None),
            (ImplRepr::Integer, None) => {
                (repr::narrowest(atomix, width), Some(repr::width_check(atomix, ident, width)))
            },
            (ImplRepr::Integer, Some(stated)) => {
                let integer_check = repr::integer_check(self.implementor, stated);
                let integer = quote!(::core::primitive::#stated);
                let width_check = repr::stated_width_check(atomix, ident, &integer, stated, width);
                (repr::selected_from(atomix, stated), Some(quote!(#integer_check #width_check)))
            },
        }
    }

    /// The local `name`: itself where the layout's locals are constants, else as `lay_out`
    /// computes it for the instance `Self` is.
    pub(super) fn local(&self, name: &Ident) -> TokenStream {
        if self.is_laid_out_per_instance() { self.instance_local(name) } else { quote!(#name) }
    }

    /// The local `name` of the instance `Self` is, as the function that lays it out computes it.
    ///
    /// # Panics
    /// `name` is no local pushed before: a fault of the derive's own.
    #[track_caller]
    #[expect(clippy::expect_used, reason = "the derive names only the locals it pushed")]
    fn instance_local(&self, name: &Ident) -> TokenStream {
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
