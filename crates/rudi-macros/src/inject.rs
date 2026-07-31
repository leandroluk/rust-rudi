use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse2, parse_quote, Block, FnArg, ItemFn, Pat, Type};

pub(crate) fn expand(item: TokenStream) -> TokenStream {
    let mut item_fn = match parse2::<ItemFn>(item) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let mut marked: Option<(usize, Pat, Type)> = None;
    for (idx, input) in item_fn.sig.inputs.iter().enumerate() {
        let FnArg::Typed(pt) = input else { continue };
        let has_marker = pt.attrs.iter().any(|a| a.path().is_ident("container"));
        if !has_marker {
            continue;
        }
        if marked.is_some() {
            return syn::Error::new_spanned(
                pt,
                "#[inject] permite no máximo 1 parâmetro marcado #[container]",
            )
            .to_compile_error();
        }
        marked = Some((idx, (*pt.pat).clone(), (*pt.ty).clone()));
    }

    let Some((idx, pat, ty)) = marked else {
        return syn::Error::new_spanned(
            &item_fn.sig,
            "#[inject] exige exatamente 1 parâmetro marcado #[container]",
        )
        .to_compile_error();
    };

    let is_ref = matches!(ty, Type::Reference(_));

    let mut inputs: Punctuated<FnArg, Comma> = Punctuated::new();
    for (i, input) in std::mem::take(&mut item_fn.sig.inputs)
        .into_iter()
        .enumerate()
    {
        if i != idx {
            inputs.push(input);
        }
    }
    item_fn.sig.inputs = inputs;

    let init_stmt = if is_ref {
        quote! { let #pat: #ty = &::rudi::container(); }
    } else {
        quote! { let #pat: #ty = ::rudi::container(); }
    };

    let original_block = &item_fn.block;
    let new_block: Block = parse_quote! {{
        #init_stmt
        #original_block
    }};
    *item_fn.block = new_block;

    quote! { #item_fn }
}
