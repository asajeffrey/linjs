/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate proc_macro;
extern crate syn;
extern crate synstructure;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

//  -------------------------------------------------------------------------------------------------------

#[proc_macro_derive(JSRootable)]
pub fn derive_js_rootable(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_js_rootable(&ast);
    gen.parse().unwrap()
}

fn impl_js_rootable(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let (_, ty_generics, _) = ast.generics.split_for_impl();

    let impl_generics = ast.generics.ty_params.iter().map(|ty| quote! { #ty });
    let impl_generics = quote! { #(#impl_generics),* };

    // append the lifetime constraints to the generic type parameters
    let lifetime_constraints = ast.generics.ty_params.iter().map(|ty| {
        let ident = &ty.ident;
        quote! { #ident: 'b }
    });

    let where_clause_predicates = ast.generics.where_clause.predicates.iter().map(|pred| quote! { #pred });
    let where_clause_items = lifetime_constraints.chain(where_clause_predicates).collect::<Vec<_>>();
    let where_clause = if where_clause_items.is_empty() {
        quote! { }
    } else {
        quote! { where #(#where_clause_items),* }
    };

    // For types without any liftime parameters, we provide a trivial
    // implementation of `JSRootable`.
    if ast.generics.lifetimes.is_empty() {
        return quote! {
            #[allow(unsafe_code)]
            unsafe impl<'a, #impl_generics> ::linjs::JSRootable<'a> for #name #ty_generics #where_clause {
                type Aged = #name #ty_generics;
            }
        }
    }

    // we assume there's only one lifetime param, not named 'b
    assert!(ast.generics.lifetimes.len() == 1, "deriving JSRootable requires a single lifetime");

    let impl_lifetime = &ast.generics.lifetimes[0].lifetime.ident;
    assert!(impl_lifetime != "'b", "deriving JSRootable requires the lifetime to not be named 'b");

    // the `Aged` associated type params are the ty_params without their bounds
    let aged_ty_params = ast.generics.ty_params.iter().map(|ty| {
        let ident = &ty.ident;
        quote! { #ident }
    });
    let aged_ty_params = quote! { #(#aged_ty_params),* };

    quote! {
        #[allow(unsafe_code)]
        unsafe impl<#impl_lifetime, 'b, #impl_generics> ::linjs::JSRootable<'b> for #name #ty_generics #where_clause {
            type Aged = #name<'b, #aged_ty_params>;
        }
    }
}

//  -------------------------------------------------------------------------------------------------------

#[proc_macro_derive(JSTransplantable)]
pub fn derive_js_transplantable(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_js_transplantable(&ast);
    gen.parse().unwrap()
}

fn impl_js_transplantable(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // For types without any generic parameters, we provide a trivial
    // implementation of `JSTransplantable`.
    if ast.generics.ty_params.is_empty() {
        return quote! {
            #[allow(unsafe_code)]
            unsafe impl<#impl_lifetimes, Z> ::linjs::JSTransplantable<Z> for #name #ty_generics #where_clause {
                type Aged = #name #ty_generics;
            }
        }
    }

    // we assume there's only one type param, not named Z
    assert!(ast.generics.ty_params.len() == 1, "deriving JSTransplantable requires a single type parameter");

    let impl_ty_param = &ast.generics.ty_params[0].ident;
    assert!(impl_lifetime != "Z", "deriving JSTransplantable requires the type parameter to not be named Z");

    let lifetimes = ast.generics.lifetimes.iter().map(|lifetime| lifetime.ident.collect());
    
    quote! {
        #[allow(unsafe_code)]
        unsafe impl<#lifetimes, #impl_ty_param, Z> ::linjs::JSTransplantable<Z> for #name #ty_generics #where_clause {
            type Transplanted = #name<#(#lifetimes),*, Z>;
        }
    }
}

//  -------------------------------------------------------------------------------------------------------

#[proc_macro_derive(HasClass)]
pub fn derive_has_class(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_has_class(&ast);
    gen.parse().unwrap()
}

fn impl_has_class(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let class_name = syn::Ident::new(format!("{}Class", name));

    let default_lifetimes = [ syn::LifetimeDef::new("'a") ];    
    let instance_lifetimes = if ast.generics.lifetimes.is_empty() {
        &default_lifetimes[..]
    } else {
        &*ast.generics.lifetimes
    };
    let instance_lifetime = &instance_lifetimes[0];
    let class_lifetimes = &instance_lifetimes[1..];

    let default_ty_params = [ syn::TyParam::from(syn::Ident::new("C")) ];
    let instance_ty_params = if ast.generics.ty_params.is_empty() {
        &default_ty_params[..]
    } else {
        &*ast.generics.ty_params
    };
    let instance_ty_param = &instance_ty_params[0];
    let class_ty_params = &instance_ty_params[1..];

    let class_generics = if class_lifetimes.is_empty() && class_ty_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#class_lifetimes),* , #(class_ty_params),*> }
    };

    quote! {
        pub struct #class_name;
        impl #impl_generics ::linjs::HasClass for #name #ty_generics #where_clause {
            type Class = #class_name #class_generics;
        }
        impl <#(#instance_lifetimes),* , #(#instance_ty_params),*> ::linjs::HasInstance<#instance_lifetime, #instance_ty_param> for #class_name #class_generics #where_clause {
            type Instance = #name #ty_generics;
        }
    }
}

//  -------------------------------------------------------------------------------------------------------

#[proc_macro_derive(JSTraceable)]
pub fn expand_token_stream(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_string(&input.to_string()).parse().unwrap()
}

fn expand_string(input: &str) -> String {
    let mut type_ = syn::parse_macro_input(input).unwrap();

    let style = synstructure::BindStyle::Ref.into();
    let match_body = synstructure::each_field(&mut type_, &style, |binding| {
        Some(quote! { #binding.trace(tracer); })
    });

    let name = &type_.ident;
    let (impl_generics, ty_generics, where_clause) = type_.generics.split_for_impl();
    let mut where_clause = where_clause.clone();
    for param in type_.generics.ty_params.iter().skip(1) {
        where_clause.predicates.push(syn::WherePredicate::BoundPredicate(syn::WhereBoundPredicate {
            bound_lifetimes: Vec::new(),
            bounded_ty: syn::Ty::Path(None, param.ident.clone().into()),
            bounds: vec![syn::TyParamBound::Trait(
                syn::PolyTraitRef {
                    bound_lifetimes: Vec::new(),
                    trait_ref: syn::parse_path("::linjs::JSTraceable").unwrap(),
                },
                syn::TraitBoundModifier::None
            )],
        }))
    }

    let tokens = quote! {
        #[allow(unsafe_code)]
        unsafe impl #impl_generics ::linjs::JSTraceable for #name #ty_generics #where_clause {
            #[inline]
            #[allow(unused_variables, unused_imports)]
            unsafe fn trace(&self, tracer: *mut ::linjs::JSTracer) {
                use ::linjs::JSTraceable;
                match *self {
                    #match_body
                }
            }
        }
    };

    tokens.to_string()
}
