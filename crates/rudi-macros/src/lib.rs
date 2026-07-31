mod inject;
mod injectable;

use proc_macro::TokenStream;

/// Gera `impl rudi::Injectable for Tipo` a partir de `impl Tipo { fn build(c: &Container) -> Self }`.
/// Decora o bloco `impl` inteiro (não a fn) — ver design.md pra explicação da restrição.
#[proc_macro_attribute]
pub fn injectable(attr: TokenStream, item: TokenStream) -> TokenStream {
    injectable::expand(attr.into(), item.into()).into()
}

/// Remove o parâmetro marcado `#[container]` da assinatura pública e injeta
/// `rudi::container()` como 1ª statement do corpo.
#[proc_macro_attribute]
pub fn inject(_attr: TokenStream, item: TokenStream) -> TokenStream {
    inject::expand(item.into()).into()
}

/// Ver `crates/rudi-macros/src/derive_injectable.rs` — implementação real chega em T15.
#[proc_macro_derive(Injectable)]
pub fn derive_injectable(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
