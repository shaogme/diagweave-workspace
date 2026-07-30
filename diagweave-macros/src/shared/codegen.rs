use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub(crate) fn enum_impl_helpers(
    enum_ident: &Ident,
    generics: &syn::Generics,
    source_arms: &[TokenStream],
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics #enum_ident #ty_generics #where_clause {
            pub fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                <Self as ::core::error::Error>::source(self)
            }
        }
        impl #impl_generics ::core::error::Error for #enum_ident #ty_generics #where_clause {
            fn source(&self) -> ::core::option::Option<&(dyn ::core::error::Error + 'static)> {
                match self {
                    #(#source_arms),*
                }
            }
        }
        impl #impl_generics ::diagweave::report::DiagnosticError for #enum_ident #ty_generics #where_clause {}
    }
}
