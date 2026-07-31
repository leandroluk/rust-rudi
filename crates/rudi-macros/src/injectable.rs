use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, FnArg, ImplItem, ItemImpl, PathArguments, ReturnType, Type};

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_impl = match parse2::<ItemImpl>(item) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    if item_impl.trait_.is_some() {
        return syn::Error::new_spanned(
            &item_impl,
            "#[injectable] deve decorar `impl Tipo { ... }` sem trait (não `impl Trait for Tipo`)",
        )
        .to_compile_error();
    }

    let self_ty = &item_impl.self_ty;

    let build_fn = item_impl.items.iter().find_map(|item| match item {
        ImplItem::Fn(f) if f.sig.ident == "build" => Some(f),
        _ => None,
    });

    let build_fn = match build_fn {
        Some(f) => f,
        None => {
            return syn::Error::new_spanned(
                &item_impl,
                "#[injectable] exige uma fn `build` dentro do impl",
            )
            .to_compile_error();
        }
    };

    if build_fn.sig.inputs.len() != 1 {
        return syn::Error::new_spanned(
            &build_fn.sig,
            "fn `build` deve ter exatamente 1 parâmetro (&Container ou Container)",
        )
        .to_compile_error();
    }
    let param = build_fn.sig.inputs.first().unwrap();
    let param_ty = match param {
        FnArg::Typed(pt) => &*pt.ty,
        FnArg::Receiver(_) => {
            return syn::Error::new_spanned(param, "fn `build` não pode receber `self`")
                .to_compile_error();
        }
    };
    let (inner_ty, is_ref) = match param_ty {
        Type::Reference(r) => (&*r.elem, true),
        other => (other, false),
    };
    let is_container = matches!(inner_ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Container"));
    if !is_container {
        return syn::Error::new_spanned(
            param_ty,
            "parâmetro de `build` deve ser `&Container` ou `Container` (sem alias — ver documentação de #[injectable])",
        )
        .to_compile_error();
    }

    let is_async = build_fn.sig.asyncness.is_some();

    let (error_ty, is_result) = match &build_fn.sig.output {
        ReturnType::Type(_, ty) => match &**ty {
            Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Result") => {
                let seg = p.path.segments.last().unwrap();
                let PathArguments::AngleBracketed(args) = &seg.arguments else {
                    return syn::Error::new_spanned(
                        ty,
                        "Result de `build` deve ter 2 parâmetros de tipo: Result<Self, Error>",
                    )
                    .to_compile_error();
                };
                match args.args.iter().nth(1) {
                    Some(syn::GenericArgument::Type(e)) => (quote!(#e), true),
                    _ => {
                        return syn::Error::new_spanned(
                            ty,
                            "Result de `build` deve ter 2 parâmetros de tipo: Result<Self, Error>",
                        )
                        .to_compile_error();
                    }
                }
            }
            _ => (quote!(::std::convert::Infallible), false),
        },
        ReturnType::Default => {
            return syn::Error::new_spanned(
                &build_fn.sig,
                "fn `build` deve retornar `Self` ou `Result<Self, E>`",
            )
            .to_compile_error();
        }
    };

    let port_ty: TokenStream = if attr.is_empty() {
        quote!(#self_ty)
    } else {
        match parse2::<Type>(attr) {
            Ok(t) => quote!(#t),
            Err(e) => return e.to_compile_error(),
        }
    };

    let call_arg = if is_ref { quote!(&c) } else { quote!(c) };
    let base_call = if is_async {
        quote!(#self_ty::build(#call_arg).await)
    } else {
        quote!(#self_ty::build(#call_arg))
    };
    let body = if is_result {
        base_call
    } else {
        quote! {
            {
                let __value = #base_call;
                ::core::result::Result::Ok(__value)
            }
        }
    };

    quote! {
        #item_impl

        impl ::rudi::Injectable for #self_ty {
            type Error = #error_ty;
            type Port = #port_ty;

            fn build(
                c: ::rudi::Container,
            ) -> impl ::core::future::Future<Output = ::core::result::Result<Self, Self::Error>> + ::core::marker::Send
            {
                async move { #body }
            }

            fn into_port(built: ::std::sync::Arc<Self>) -> ::std::sync::Arc<Self::Port> {
                built
            }
        }
    }
}
