use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Data, DeriveInput, Fields};

use crate::resolve_codegen::resolve_field_expr;

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let input = match parse2::<DeriveInput>(input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };
    let name = &input.ident;

    let Data::Struct(data) = &input.data else {
        return syn::Error::new_spanned(&input, "#[derive(Injectable)] só suporta struct")
            .to_compile_error();
    };

    let container_expr = quote!(c);

    let build_expr = match &data.fields {
        Fields::Named(fields) => {
            let mut assigns = Vec::new();
            for f in &fields.named {
                let ident = f.ident.as_ref().unwrap();
                let Some(resolve_expr) = resolve_field_expr(&f.ty, &container_expr) else {
                    return syn::Error::new_spanned(
                        &f.ty,
                        "campo de #[derive(Injectable)] deve ser Arc<T> ou Option<Arc<T>>",
                    )
                    .to_compile_error();
                };
                assigns.push(quote! { #ident: #resolve_expr, });
            }
            quote! { Self { #(#assigns)* } }
        }
        Fields::Unnamed(fields) => {
            let mut assigns = Vec::new();
            for f in &fields.unnamed {
                let Some(resolve_expr) = resolve_field_expr(&f.ty, &container_expr) else {
                    return syn::Error::new_spanned(
                        &f.ty,
                        "campo de #[derive(Injectable)] deve ser Arc<T> ou Option<Arc<T>>",
                    )
                    .to_compile_error();
                };
                assigns.push(quote! { #resolve_expr, });
            }
            quote! { Self( #(#assigns)* ) }
        }
        Fields::Unit => quote! { Self },
    };

    quote! {
        impl ::rudi::Injectable for #name {
            type Error = ::rudi::RudiError;
            type Port = #name;

            fn build(
                c: ::rudi::Container,
            ) -> impl ::core::future::Future<Output = ::core::result::Result<Self, Self::Error>> + ::core::marker::Send
            {
                async move { ::core::result::Result::Ok(#build_expr) }
            }

            fn into_port(built: ::std::sync::Arc<Self>) -> ::std::sync::Arc<Self::Port> {
                built
            }
        }
    }
}
