use proc_macro::TokenStream;

/// Ver `crates/rudi-macros/src/injectable.rs` — implementação real chega em T13.
#[proc_macro_attribute]
pub fn injectable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Ver `crates/rudi-macros/src/inject.rs` — implementação real chega em T14.
#[proc_macro_attribute]
pub fn inject(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Ver `crates/rudi-macros/src/derive_injectable.rs` — implementação real chega em T15.
#[proc_macro_derive(Injectable)]
pub fn derive_injectable(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
