use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{Error, Ident, Result, Variant};

use crate::set::resolver::{ResolvedSet, ResolvedVariant};
use crate::shared::codegen::enum_impl_helpers;
use crate::shared::derive::merge_debug_derive;
use crate::shared::display::display_arm;
use crate::shared::from_attr::{from_variant_source, is_from_variant};
use crate::shared::sanitize::sanitize_variant_attrs;
use crate::shared::source::source_arm_for_variant;

pub(crate) fn generate_enum_impl(set: &ResolvedSet) -> Result<TokenStream> {
    let enum_ident = &set.name;
    let vis = &set.vis;
    let generics = &set.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let variants: Vec<Variant> = set
        .variants
        .iter()
        .map(|v| sanitize_variant_attrs(&v.variant))
        .collect();
    let display_arms = set
        .variants
        .iter()
        .map(|v| display_arm(enum_ident, &v.variant))
        .collect::<Result<Vec<_>>>()?;
    let source_arms = set
        .variants
        .iter()
        .map(|v| source_arm_for_variant(&v.variant))
        .collect::<Result<Vec<_>>>()?;
    let variant_from_impls = from_impls_for_variants(enum_ident, generics, &set.variants)?;
    let merged_attrs = merge_debug_derive(set.attrs.clone())?;
    let enum_impl_helpers = enum_impl_helpers(enum_ident, generics, &source_arms);
    Ok(quote! {
        #(#merged_attrs)*
        #vis enum #enum_ident #generics { #(#variants),* }
        #enum_impl_helpers
        impl #impl_generics ::core::fmt::Display for #enum_ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self { #(#display_arms),* }
            }
        }
        #(#variant_from_impls)*
    })
}

pub(crate) fn generate_all_from_impls(
    names: &[String],
    resolved: &BTreeMap<String, ResolvedSet>,
) -> Result<Vec<TokenStream>> {
    let mut from_impls = Vec::new();
    for outer_name in names {
        let outer = resolved
            .get(outer_name)
            .ok_or_else(|| Error::new(Span::call_site(), "resolved set must exist"))?;
        let outer_ident = &outer.name;
        let (outer_impl_generics, outer_ty_generics, outer_where_clause) =
            outer.generics.split_for_impl();
        let mut generated_keys = std::collections::BTreeSet::<String>::new();

        // 1. Process set_refs declared or inherited by outer set
        for set_ref in &outer.set_refs {
            let inner_name = set_ref.name.to_string();
            if inner_name == *outer_name {
                continue;
            }
            let inner = match resolved.get(&inner_name) {
                Some(set) => set,
                None => continue,
            };
            let inner_ident = &inner.name;
            let inner_ty = if let Some(args) = &set_ref.args {
                quote!(#inner_ident #args)
            } else if inner.generics.params.is_empty() {
                quote!(#inner_ident)
            } else {
                let (_, inner_ty_generics, _) = inner.generics.split_for_impl();
                quote!(#inner_ident #inner_ty_generics)
            };

            let key = inner_ty.to_string();
            if !generated_keys.insert(key) {
                continue;
            }

            let arms = inner
                .variants
                .iter()
                .map(|v| from_arm(&inner.name, &v.variant))
                .collect::<Result<Vec<_>>>()?;

            from_impls.push(quote! {
                impl #outer_impl_generics ::core::convert::From<#inner_ty> for #outer_ident #outer_ty_generics #outer_where_clause {
                    fn from(value: #inner_ty) -> Self {
                        match value {
                            #(#arms),*
                        }
                    }
                }
            });
        }

        // 2. Process non-generic subset relationships
        for inner_name in names {
            if inner_name == outer_name {
                continue;
            }
            let inner = resolved
                .get(inner_name)
                .ok_or_else(|| Error::new(Span::call_site(), "resolved set must exist"))?;

            if inner.generics.params.is_empty()
                && outer.generics.params.is_empty()
                && inner.members.is_subset_of(&outer.members)
            {
                let inner_ident = &inner.name;
                let key = inner_ident.to_string();
                if !generated_keys.insert(key) {
                    continue;
                }

                let arms = inner
                    .variants
                    .iter()
                    .map(|v| from_arm(&inner.name, &v.variant))
                    .collect::<Result<Vec<_>>>()?;

                from_impls.push(quote! {
                    impl ::core::convert::From<#inner_ident> for #outer_ident {
                        fn from(value: #inner_ident) -> Self {
                            match value {
                                #(#arms),*
                            }
                        }
                    }
                });
            }
        }
    }
    Ok(from_impls)
}

fn from_impls_for_variants(
    enum_ident: &Ident,
    generics: &syn::Generics,
    variants: &[ResolvedVariant],
) -> Result<Vec<TokenStream>> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut used_source_types = BTreeMap::<String, Span>::new();
    let mut impls = Vec::new();
    for resolved in variants {
        let variant = &resolved.variant;
        if !is_from_variant(variant)? {
            continue;
        }
        let (source_ty, ctor) = from_variant_source(variant)?;
        let key = quote!(#source_ty).to_string();
        if used_source_types.contains_key(&key) {
            return Err(Error::new(
                variant.ident.span(),
                format!("duplicate From source type `{}` in `{}`", key, enum_ident),
            ));
        }
        used_source_types.insert(key, variant.ident.span());
        impls.push(quote! {
            impl #impl_generics ::core::convert::From<#source_ty> for #enum_ident #ty_generics #where_clause {
                fn from(value: #source_ty) -> Self {
                    #ctor
                }
            }
        });
    }
    Ok(impls)
}

fn from_arm(inner: &Ident, variant: &Variant) -> Result<TokenStream> {
    let variant_name = &variant.ident;
    match &variant.fields {
        syn::Fields::Unit => Ok(quote! {
            #inner::#variant_name => Self::#variant_name
        }),
        syn::Fields::Named(fields_named) => {
            let idents: Vec<Ident> = fields_named
                .named
                .iter()
                .map(|f| {
                    f.ident
                        .clone()
                        .ok_or_else(|| Error::new_spanned(f, "named field should have ident"))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! {
                #inner::#variant_name { #(#idents),* } => Self::#variant_name { #(#idents),* }
            })
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            let binders = (0..fields_unnamed.unnamed.len())
                .map(|idx| format_ident!("f{idx}"))
                .collect::<Vec<_>>();
            Ok(quote! {
                #inner::#variant_name(#(#binders),*) => Self::#variant_name(#(#binders),*)
            })
        }
    }
}
