use crate::shared::display::render_display_template;
use quote::quote;
use syn::{Attribute, Error, Fields, Ident, LitStr, Result, spanned::Spanned};

#[derive(Clone)]
pub(crate) enum ErrorDisplay {
    Template(LitStr),
    Transparent,
}

pub(crate) fn parse_error_display(
    attrs: &[Attribute],
    span: proc_macro2::Span,
) -> Result<ErrorDisplay> {
    let mut parsed: Option<ErrorDisplay> = None;
    for attr in attrs {
        if !attr.path().is_ident("display") {
            continue;
        }
        let current = if let Ok(lit) = attr.parse_args::<LitStr>() {
            ErrorDisplay::Template(lit)
        } else {
            let ident = attr.parse_args::<Ident>()?;
            if ident == "transparent" {
                ErrorDisplay::Transparent
            } else {
                return Err(Error::new_spanned(
                    ident,
                    "unsupported #[display(...)] argument; expected string literal or `transparent`",
                ));
            }
        };
        if parsed.replace(current).is_some() {
            return Err(Error::new_spanned(
                attr,
                "duplicate #[display(...)] attribute",
            ));
        }
    }
    parsed.ok_or_else(|| {
        Error::new(
            span,
            "missing #[display(...)] attribute; expected #[display(\"...\")] or #[display(transparent)]",
        )
    })
}

pub(crate) fn display_expr(
    display: &ErrorDisplay,
    variant_ident: &Ident,
    replacements: &[(String, proc_macro2::TokenStream)],
) -> Result<proc_macro2::TokenStream> {
    match display {
        ErrorDisplay::Template(template) => {
            let (fmt_template, ordered) = render_display_template(template, replacements)?;
            Ok(quote! {
                write!(f, #fmt_template #(, #ordered)*)
            })
        }
        ErrorDisplay::Transparent => {
            if replacements.len() != 1 {
                return Err(Error::new(
                    variant_ident.span(),
                    "#[display(transparent)] requires exactly one field",
                ));
            }
            let inner = &replacements[0].1;
            Ok(quote! {
                write!(f, "{}", #inner)
            })
        }
    }
}

pub(crate) fn replacements(
    fields: &Fields,
    binding: &super::codegen::BindingStyle,
) -> Result<Vec<(String, proc_macro2::TokenStream)>> {
    use super::codegen::BindingStyle;
    match (fields, binding) {
        (Fields::Unit, BindingStyle::Unit) => Ok(Vec::new()),
        (Fields::Named(_), BindingStyle::Named(idents)) => Ok(idents
            .iter()
            .map(|ident| (ident.to_string(), quote! { #ident }))
            .collect()),
        (Fields::Unnamed(_), BindingStyle::Unnamed(idents)) => Ok(idents
            .iter()
            .enumerate()
            .map(|(idx, ident)| (idx.to_string(), quote! { #ident }))
            .collect()),
        _ => Err(Error::new(fields.span(), "internal binding mismatch")),
    }
}
