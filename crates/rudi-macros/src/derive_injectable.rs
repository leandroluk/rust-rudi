use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

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

    let build_expr = match &data.fields {
        Fields::Named(fields) => {
            let mut assigns = Vec::new();
            for f in &fields.named {
                let ident = f.ident.as_ref().unwrap();
                let Some(inner) = arc_inner(&f.ty) else {
                    return syn::Error::new_spanned(
                        &f.ty,
                        "campo de #[derive(Injectable)] deve ser Arc<T> (resolve sempre retorna Arc<T>)",
                    )
                    .to_compile_error();
                };
                assigns.push(quote! { #ident: c.resolve::<#inner>().await?, });
            }
            quote! { Self { #(#assigns)* } }
        }
        Fields::Unnamed(fields) => {
            let mut assigns = Vec::new();
            for f in &fields.unnamed {
                let Some(inner) = arc_inner(&f.ty) else {
                    return syn::Error::new_spanned(
                        &f.ty,
                        "campo de #[derive(Injectable)] deve ser Arc<T> (resolve sempre retorna Arc<T>)",
                    )
                    .to_compile_error();
                };
                assigns.push(quote! { c.resolve::<#inner>().await?, });
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

fn arc_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    if seg.ident != "Arc" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };
    args.args.iter().find_map(|a| match a {
        GenericArgument::Type(t) => Some(t),
        _ => None,
    })
}
