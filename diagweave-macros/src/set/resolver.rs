use std::collections::BTreeMap;

use proc_macro2::Span;
use quote::quote;
use syn::visit_mut::{self, VisitMut};
use syn::{Attribute, Error, GenericArgument, GenericParam, Ident, Result, Variant};

use crate::set::bitset::{BitSet, SymbolTable};
use crate::set::parser::{InlineVariants, SetDecl, SetRef, UnionTerm};

#[derive(Clone)]
pub(crate) struct VariantIdentity {
    pub(crate) name: String,
    pub(crate) signature: String,
}

#[derive(Clone)]
pub(crate) struct ResolvedVariant {
    pub(crate) symbol: usize,
    pub(crate) identity: VariantIdentity,
    pub(crate) variant: Variant,
}

#[derive(Clone)]
pub(crate) struct ResolvedSet {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: syn::Visibility,
    pub(crate) name: Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) variants: Vec<ResolvedVariant>,
    pub(crate) members: BitSet,
    pub(crate) set_refs: Vec<SetRef>,
}

pub(crate) fn collect_decls(decls: Vec<SetDecl>) -> Result<BTreeMap<String, SetDecl>> {
    let mut map = BTreeMap::<String, SetDecl>::new();
    for decl in decls {
        let key = decl.name.to_string();
        if let Some(existing) = map.get(&key) {
            return Err(Error::new(
                decl.name.span(),
                format!(
                    "set `{}` is defined more than once; first definition span: {:?}",
                    key,
                    existing.name.span()
                ),
            ));
        }
        map.insert(key, decl);
    }
    Ok(map)
}

pub(crate) fn resolve_set(
    name: &str,
    decls: &BTreeMap<String, SetDecl>,
    resolved: &mut BTreeMap<String, ResolvedSet>,
    stack: &mut Vec<String>,
    symbol_table: &mut SymbolTable,
) -> Result<()> {
    if resolved.contains_key(name) {
        return Ok(());
    }

    if stack.iter().any(|n| n == name) {
        let mut chain = stack.clone();
        chain.push(name.to_owned());
        return Err(Error::new(
            Span::call_site(),
            format!("cyclic dependency detected: {}", chain.join(" -> ")),
        ));
    }

    let decl = decls.get(name).ok_or_else(|| {
        Error::new(
            Span::call_site(),
            format!("referenced undefined set `{name}`"),
        )
    })?;

    stack.push(name.to_owned());
    let mut variants = Vec::new();
    let mut signatures = BTreeMap::new();
    let mut members = BitSet::with_capacity(symbol_table.len());
    let mut set_refs = Vec::new();

    {
        let mut ctx = ResolveContext {
            decls,
            resolved,
            stack,
            symbol_table,
            variants: &mut variants,
            signatures: &mut signatures,
            members: &mut members,
            set_refs: &mut set_refs,
        };

        for term in &decl.expr.terms {
            ctx.resolve_term(term)?;
        }
    }

    stack.pop();
    resolved.insert(
        name.to_owned(),
        ResolvedSet {
            attrs: decl.attrs.clone(),
            vis: decl.vis.clone(),
            name: decl.name.clone(),
            generics: decl.generics.clone(),
            variants,
            members,
            set_refs,
        },
    );
    Ok(())
}

struct ResolveContext<'a> {
    decls: &'a BTreeMap<String, SetDecl>,
    resolved: &'a mut BTreeMap<String, ResolvedSet>,
    stack: &'a mut Vec<String>,
    symbol_table: &'a mut SymbolTable,
    variants: &'a mut Vec<ResolvedVariant>,
    signatures: &'a mut BTreeMap<String, VariantIdentity>,
    members: &'a mut BitSet,
    set_refs: &'a mut Vec<SetRef>,
}

impl<'a> ResolveContext<'a> {
    fn try_push_variant(&mut self, variant: ResolvedVariant) -> Result<()> {
        let name = variant.identity.name.clone();
        match self.signatures.get(&name) {
            Some(existing) if existing.signature == variant.identity.signature => {
                self.members.insert(variant.symbol);
                Ok(())
            }
            Some(_) => Err(Error::new_spanned(
                &variant.variant.ident,
                format!(
                    "variant `{name}` has the same name but a different field shape; cannot deduplicate union members"
                ),
            )),
            None => {
                self.signatures.insert(name, variant.identity.clone());
                self.members.insert(variant.symbol);
                self.variants.push(variant);
                Ok(())
            }
        }
    }

    fn resolve_term(&mut self, term: &UnionTerm) -> Result<()> {
        match term {
            UnionTerm::SetRef(set_ref) => self.resolve_set_ref(set_ref),
            UnionTerm::Inline(inline) => self.resolve_inline(inline),
        }
    }

    fn resolve_set_ref(&mut self, set_ref: &SetRef) -> Result<()> {
        let ref_name = set_ref.name.to_string();
        if !self.decls.contains_key(&ref_name) {
            return Err(Error::new_spanned(
                &set_ref.name,
                format!("referenced undefined set `{ref_name}`"),
            ));
        }
        resolve_set(
            &ref_name,
            self.decls,
            self.resolved,
            self.stack,
            self.symbol_table,
        )?;
        self.set_refs.push(set_ref.clone());

        let (source_generics, source_variants, inherited_refs) = {
            let source = self
                .resolved
                .get(&ref_name)
                .ok_or_else(|| Error::new(set_ref.name.span(), "source set should be resolved"))?;
            (
                source.generics.clone(),
                source.variants.clone(),
                source.set_refs.clone(),
            )
        };

        let mut type_map = BTreeMap::<Ident, syn::Type>::new();
        let mut lifetime_map = BTreeMap::<syn::Lifetime, syn::Lifetime>::new();

        if let Some(args) = &set_ref.args {
            let mut arg_iter = args.args.iter();
            for param in &source_generics.params {
                match param {
                    GenericParam::Type(type_param) => {
                        if let Some(arg) = arg_iter.next()
                            && let GenericArgument::Type(ty) = arg
                        {
                            type_map.insert(type_param.ident.clone(), ty.clone());
                        }
                    }
                    GenericParam::Lifetime(lifetime_param) => {
                        if let Some(arg) = arg_iter.next()
                            && let GenericArgument::Lifetime(lt) = arg
                        {
                            lifetime_map.insert(lifetime_param.lifetime.clone(), lt.clone());
                        }
                    }
                    GenericParam::Const(_) => {}
                }
            }
        } else {
            for param in &source_generics.params {
                if let GenericParam::Type(type_param) = param {
                    let id = &type_param.ident;
                    type_map.insert(id.clone(), syn::parse_quote!(#id));
                } else if let GenericParam::Lifetime(lifetime_param) = param {
                    let lt = &lifetime_param.lifetime;
                    lifetime_map.insert(lt.clone(), lt.clone());
                }
            }
        }

        for mut inherited in inherited_refs {
            if let Some(ref mut args) = inherited.args
                && (!type_map.is_empty() || !lifetime_map.is_empty())
            {
                let mut substitutor = GenericSubstitutor {
                    type_map: &type_map,
                    lifetime_map: &lifetime_map,
                };
                substitutor.visit_angle_bracketed_generic_arguments_mut(args);
            }
            self.set_refs.push(inherited);
        }

        for var in source_variants {
            let mut substituted_variant = var.variant.clone();
            substitute_generics(&mut substituted_variant, &type_map, &lifetime_map);
            let sig = variant_signature(&substituted_variant);
            let sym = self.symbol_table.intern(sig.clone());
            let res = ResolvedVariant {
                symbol: sym,
                identity: VariantIdentity {
                    name: substituted_variant.ident.to_string(),
                    signature: sig,
                },
                variant: substituted_variant,
            };
            self.try_push_variant(res)?;
        }
        Ok(())
    }

    fn resolve_inline(&mut self, inline: &InlineVariants) -> Result<()> {
        for variant in &inline.variants {
            let sig = variant_signature(variant);
            let sym = self.symbol_table.intern(sig.clone());
            let res = ResolvedVariant {
                symbol: sym,
                identity: VariantIdentity {
                    name: variant.ident.to_string(),
                    signature: sig,
                },
                variant: variant.clone(),
            };
            self.try_push_variant(res)?;
        }
        Ok(())
    }
}

struct GenericSubstitutor<'a> {
    type_map: &'a BTreeMap<Ident, syn::Type>,
    lifetime_map: &'a BTreeMap<syn::Lifetime, syn::Lifetime>,
}

impl<'b> VisitMut for GenericSubstitutor<'b> {
    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        if let syn::Type::Path(type_path) = ty
            && type_path.qself.is_none()
            && type_path.path.segments.len() == 1
        {
            let ident = &type_path.path.segments[0].ident;
            if let Some(replacement) = self.type_map.get(ident) {
                *ty = replacement.clone();
                return;
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }

    fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
        if let Some(replacement) = self.lifetime_map.get(lifetime) {
            *lifetime = replacement.clone();
        }
        visit_mut::visit_lifetime_mut(self, lifetime);
    }
}

fn substitute_generics(
    variant: &mut Variant,
    type_map: &BTreeMap<Ident, syn::Type>,
    lifetime_map: &BTreeMap<syn::Lifetime, syn::Lifetime>,
) {
    if type_map.is_empty() && lifetime_map.is_empty() {
        return;
    }
    let mut substitutor = GenericSubstitutor {
        type_map,
        lifetime_map,
    };
    substitutor.visit_variant_mut(variant);
}

pub(crate) fn variant_signature(variant: &Variant) -> String {
    let mut shape_only = variant.clone();
    shape_only.attrs = vec![];
    for field in shape_only.fields.iter_mut() {
        field.attrs = vec![];
    }
    quote!(#shape_only).to_string()
}
