mod derive_injectable;
mod inject;
mod injectable;
mod resolve_codegen;

use proc_macro::TokenStream;

/// Gera `impl rudi::Injectable for Tipo` a partir de um construtor sem `self` onde
/// todo parâmetro é marcado `#[inject]`/`#[inject_all]`/`#[container]`. Decora o
/// bloco `impl` inteiro (não a fn) — ver design.md pra explicação da restrição.
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

/// Gera `impl rudi::Injectable for Tipo` resolvendo cada campo `Arc<T>` do container.
#[proc_macro_derive(Injectable)]
pub fn derive_injectable(item: TokenStream) -> TokenStream {
    derive_injectable::expand(item.into()).into()
}
