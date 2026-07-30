use syn::{
    Attribute, Ident, Result, Token, Variant, Visibility, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub(crate) struct SetInput {
    pub(crate) decls: Vec<SetDecl>,
}

impl Parse for SetInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut decls = Vec::new();
        while !input.is_empty() {
            let decl = input.parse::<SetDecl>()?;
            decls.push(decl);
        }
        Ok(Self { decls })
    }
}

#[derive(Clone)]
pub(crate) struct SetDecl {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) name: Ident,
    pub(crate) generics: syn::Generics,
    pub(crate) expr: UnionExpr,
}

impl Parse for SetDecl {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse::<Visibility>()?;
        let name = input.parse::<Ident>()?;
        let mut generics = input.parse::<syn::Generics>()?;
        generics.where_clause = input.parse::<Option<syn::WhereClause>>()?;
        if generics.where_clause.is_none() && input.peek(Token![where]) {
            generics.where_clause = input.parse::<Option<syn::WhereClause>>()?;
        }
        input.parse::<Token![=]>()?;
        if generics.where_clause.is_none() && input.peek(Token![where]) {
            generics.where_clause = input.parse::<Option<syn::WhereClause>>()?;
        }
        let expr = input.parse::<UnionExpr>()?;
        if generics.where_clause.is_none() && input.peek(Token![where]) {
            generics.where_clause = input.parse::<Option<syn::WhereClause>>()?;
        }
        Ok(Self {
            attrs,
            vis,
            name,
            generics,
            expr,
        })
    }
}

#[derive(Clone)]
pub(crate) struct UnionExpr {
    pub(crate) terms: Vec<UnionTerm>,
}

impl Parse for UnionExpr {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let terms = Punctuated::<UnionTerm, Token![|]>::parse_separated_nonempty(input)?;
        Ok(Self {
            terms: terms.into_iter().collect(),
        })
    }
}

#[derive(Clone)]
pub(crate) struct SetRef {
    pub(crate) name: Ident,
    pub(crate) args: Option<syn::AngleBracketedGenericArguments>,
}

#[derive(Clone)]
pub(crate) enum UnionTerm {
    SetRef(SetRef),
    Inline(InlineVariants),
}

impl Parse for UnionTerm {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::token::Brace) {
            return Ok(Self::Inline(input.parse::<InlineVariants>()?));
        }
        if input.peek(Ident) {
            let name = input.parse::<Ident>()?;
            let args = if input.peek(Token![<]) {
                Some(input.parse::<syn::AngleBracketedGenericArguments>()?)
            } else {
                None
            };
            return Ok(Self::SetRef(SetRef { name, args }));
        }
        Err(input.error("union term must be a set identifier (with optional generic arguments) or an inline variant block"))
    }
}

#[derive(Clone)]
pub(crate) struct InlineVariants {
    pub(crate) variants: Vec<Variant>,
}

impl Parse for InlineVariants {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        braced!(content in input);
        let variants = Punctuated::<Variant, Token![,]>::parse_terminated(&content)?;
        Ok(Self {
            variants: variants.into_iter().collect(),
        })
    }
}
