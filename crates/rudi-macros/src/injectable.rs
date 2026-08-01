use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse2, Attribute, FnArg, ImplItem, ImplItemFn, ItemImpl, PathArguments, ReturnType, Type,
};

use crate::resolve_codegen::{resolve_all_vec_expr, resolve_field_expr};

enum Marker {
    Container,
    Inject,
    InjectAll,
}

fn marker_kind(attrs: &[Attribute]) -> Option<Marker> {
    attrs.iter().find_map(|a| {
        if a.path().is_ident("container") {
            Some(Marker::Container)
        } else if a.path().is_ident("inject_all") {
            Some(Marker::InjectAll)
        } else if a.path().is_ident("inject") {
            Some(Marker::Inject)
        } else {
            None
        }
    })
}

fn has_receiver(f: &ImplItemFn) -> bool {
    f.sig.inputs.iter().any(|a| matches!(a, FnArg::Receiver(_)))
}

/// Constructor shape `#[injectable]` requires: no `self`, 1+ parameters, every
/// one marked `#[inject]`/`#[inject_all]`/`#[container]`. There's no other shape —
/// Rust has no language-level "constructor", so this is the only signal available.
fn is_ctor(f: &ImplItemFn) -> bool {
    if has_receiver(f) || f.sig.inputs.is_empty() {
        return false;
    }
    f.sig.inputs.iter().all(|arg| match arg {
        FnArg::Typed(pt) => marker_kind(&pt.attrs).is_some(),
        FnArg::Receiver(_) => false,
    })
}

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

    let self_ty = item_impl.self_ty.clone();

    let candidate_indices: Vec<usize> = item_impl
        .items
        .iter()
        .enumerate()
        .filter_map(|(idx, it)| match it {
            ImplItem::Fn(f) if is_ctor(f) => Some(idx),
            _ => None,
        })
        .collect();

    let ctor_idx = match candidate_indices.as_slice() {
        [] => {
            return syn::Error::new_spanned(
                &item_impl,
                "#[injectable] exige uma fn construtor: sem `self`, todo parâmetro marcado \
                 #[inject]/#[inject_all]/#[container]",
            )
            .to_compile_error();
        }
        [only] => *only,
        many => {
            let names = many
                .iter()
                .map(|&i| {
                    let ImplItem::Fn(f) = &item_impl.items[i] else {
                        unreachable!()
                    };
                    f.sig.ident.to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");
            return syn::Error::new_spanned(
                &item_impl,
                format!(
                    "#[injectable] ambíguo — {} fns candidatas a construtor: {names}",
                    many.len()
                ),
            )
            .to_compile_error();
        }
    };

    // Marker attributes (#[inject]/#[inject_all]/#[container]) only mean something
    // WHILE this macro is expanding — once we emit the impl block back out, they'd
    // sit there as unresolvable unknown attributes (rustc doesn't reprocess already-
    // expanded output). Strip them from a clone before splicing it back in; the
    // original `item_impl` (with markers intact) is still used below to read each
    // param's marker kind.
    let mut clean_impl = item_impl.clone();
    let ImplItem::Fn(clean_ctor_fn) = &mut clean_impl.items[ctor_idx] else {
        unreachable!("ctor_idx always points at an ImplItem::Fn")
    };
    for arg in clean_ctor_fn.sig.inputs.iter_mut() {
        if let FnArg::Typed(pt) = arg {
            pt.attrs
                .retain(|a| marker_kind(std::slice::from_ref(a)).is_none());
        }
    }

    let ImplItem::Fn(ctor_fn) = &item_impl.items[ctor_idx] else {
        unreachable!("ctor_idx always points at an ImplItem::Fn")
    };

    expand_ctor(&self_ty, &clean_impl, ctor_fn, attr)
}

fn parse_port_arg(self_ty: &Type, attr: TokenStream) -> TokenStream {
    if attr.is_empty() {
        quote!(#self_ty)
    } else {
        match parse2::<Type>(attr) {
            Ok(t) => quote!(#t),
            Err(e) => e.to_compile_error(),
        }
    }
}

/// Detects `-> Self` vs `-> Result<Self, E>`, returning `(error_ty, is_result)`.
/// Bare `Self` still defaults to `RudiError` (not `Infallible`) — the generated
/// body always has at least 1 `resolve`/`resolve_all` call (`?`-propagated), since
/// every parameter is either injected or the raw container (never truly infallible).
fn extract_return(output: &ReturnType) -> Result<(TokenStream, bool), TokenStream> {
    match output {
        ReturnType::Type(_, ty) => match &**ty {
            Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Result") => {
                let seg = p.path.segments.last().unwrap();
                let PathArguments::AngleBracketed(args) = &seg.arguments else {
                    return Err(syn::Error::new_spanned(
                        ty,
                        "Result do construtor deve ter 2 parâmetros de tipo: Result<Self, Error>",
                    )
                    .to_compile_error());
                };
                match args.args.iter().nth(1) {
                    Some(syn::GenericArgument::Type(e)) => Ok((quote!(#e), true)),
                    _ => Err(syn::Error::new_spanned(
                        ty,
                        "Result do construtor deve ter 2 parâmetros de tipo: Result<Self, Error>",
                    )
                    .to_compile_error()),
                }
            }
            _ => Ok((quote!(::rudi::RudiError), false)),
        },
        ReturnType::Default => Err(syn::Error::new_spanned(
            output,
            "construtor deve retornar `Self` ou `Result<Self, E>`",
        )
        .to_compile_error()),
    }
}

fn expand_ctor(
    self_ty: &Type,
    item_impl: &ItemImpl,
    ctor_fn: &ImplItemFn,
    attr: TokenStream,
) -> TokenStream {
    let fn_name = &ctor_fn.sig.ident;
    let container_expr = quote!(c);

    let mut let_stmts = Vec::new();
    let mut call_args = Vec::new();

    for (i, arg) in ctor_fn.sig.inputs.iter().enumerate() {
        let FnArg::Typed(pt) = arg else {
            unreachable!("is_ctor already validated no receiver")
        };
        let var = format_ident!("__arg{i}");
        let ty = &pt.ty;

        match marker_kind(&pt.attrs) {
            Some(Marker::Container) => {
                // Hands over the `c` already in scope — never goes through `resolve`,
                // there's no "resolve the container from itself" lookup.
                let is_ref = matches!(&**ty, Type::Reference(_));
                if is_ref {
                    let_stmts.push(quote! { let #var: #ty = &#container_expr.clone(); });
                } else {
                    let_stmts.push(quote! { let #var: #ty = #container_expr.clone(); });
                }
            }
            Some(Marker::Inject) => {
                let Some(expr) = resolve_field_expr(ty, &container_expr) else {
                    return syn::Error::new_spanned(
                        ty,
                        "#[inject] parâmetro deve ser Arc<T> ou Option<Arc<T>>",
                    )
                    .to_compile_error();
                };
                let_stmts.push(quote! { let #var = #expr; });
            }
            Some(Marker::InjectAll) => {
                let Some(expr) = resolve_all_vec_expr(ty, &container_expr) else {
                    return syn::Error::new_spanned(
                        ty,
                        "#[inject_all] parâmetro deve ser Vec<Arc<T>> (resolve_all sempre retorna Vec<Arc<T>>)",
                    )
                    .to_compile_error();
                };
                let_stmts.push(quote! { let #var = #expr; });
            }
            None => unreachable!("is_ctor already validated every param is marked"),
        }
        call_args.push(quote!(#var));
    }

    let is_async = ctor_fn.sig.asyncness.is_some();
    let base_call = if is_async {
        quote! { #self_ty::#fn_name(#(#call_args),*).await }
    } else {
        quote! { #self_ty::#fn_name(#(#call_args),*) }
    };

    let (error_ty, is_result) = match extract_return(&ctor_fn.sig.output) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let tail = if is_result {
        base_call
    } else {
        quote! {
            {
                let __value = #base_call;
                ::core::result::Result::Ok(__value)
            }
        }
    };

    let port_ty = parse_port_arg(self_ty, attr);

    quote! {
        #item_impl

        impl ::rudi::Injectable for #self_ty {
            type Error = #error_ty;
            type Port = #port_ty;

            fn build(
                c: ::rudi::Container,
            ) -> impl ::core::future::Future<Output = ::core::result::Result<Self, Self::Error>> + ::core::marker::Send
            {
                async move {
                    #(#let_stmts)*
                    #tail
                }
            }

            fn into_port(built: ::std::sync::Arc<Self>) -> ::std::sync::Arc<Self::Port> {
                built
            }
        }
    }
}
