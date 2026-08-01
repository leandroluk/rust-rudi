use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

/// Peels `Arc<Inner>` from `ty`, returning `Inner`. `None` if `ty` isn't `Arc<...>`.
pub(crate) fn arc_inner(ty: &Type) -> Option<&Type> {
    path_generic_inner(ty, "Arc")
}

/// Peels `Vec<Inner>` from `ty`, returning `Inner`. `None` if `ty` isn't `Vec<...>`.
pub(crate) fn vec_inner(ty: &Type) -> Option<&Type> {
    path_generic_inner(ty, "Vec")
}

/// Peels `Option<Inner>` from `ty`, returning `Inner`. `None` if `ty` isn't `Option<...>`.
pub(crate) fn option_inner(ty: &Type) -> Option<&Type> {
    path_generic_inner(ty, "Option")
}

fn path_generic_inner<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    if seg.ident != wrapper {
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

fn is_trait_object(ty: &Type) -> bool {
    matches!(ty, Type::TraitObject(_))
}

/// Generates the expression that resolves a field/param declared as `Arc<Inner>`,
/// as `container_expr.resolve(...)`. Handles both concrete `Inner` (resolves
/// directly, single `Arc`) and `dyn Trait` `Inner` (resolves via `Arc<Inner>` —
/// `resolve`'s `T: Sized` bound rules out resolving an unsized `dyn Trait` directly —
/// then flattens the resulting `Arc<Arc<dyn Trait>>` down to `Arc<dyn Trait>`).
/// `None` if `field_ty` isn't `Arc<...>`.
pub(crate) fn resolve_arc_expr(
    field_ty: &Type,
    container_expr: &TokenStream,
) -> Option<TokenStream> {
    let inner = arc_inner(field_ty)?;
    Some(if is_trait_object(inner) {
        quote! {
            {
                let __resolved = #container_expr.resolve::<#field_ty>().await?;
                (*__resolved).clone()
            }
        }
    } else {
        quote! { #container_expr.resolve::<#inner>().await? }
    })
}

/// Generates the expression that resolves a field/param declared as `Option<Arc<Inner>>`,
/// via `resolve_optional` — `None` (of the *dependency*, not a macro-expansion `None`)
/// when `Inner` isn't registered, instead of an error. Same trait-object flattening
/// as [`resolve_arc_expr`], applied inside the `Option::map`. Returns `Option::None`
/// (Rust's own `Option`, not this fn's return) if `field_ty` isn't `Option<Arc<...>>`.
pub(crate) fn resolve_optional_arc_expr(
    field_ty: &Type,
    container_expr: &TokenStream,
) -> Option<TokenStream> {
    let arc_ty = option_inner(field_ty)?;
    let inner = arc_inner(arc_ty)?;
    Some(if is_trait_object(inner) {
        quote! {
            #container_expr.resolve_optional::<#arc_ty>().await?
                .map(|__resolved| (*__resolved).clone())
        }
    } else {
        quote! { #container_expr.resolve_optional::<#inner>().await? }
    })
}

/// Tries [`resolve_optional_arc_expr`] first (`Option<Arc<T>>` — optional dependency),
/// falling back to [`resolve_arc_expr`] (`Arc<T>` — required). `None` if `field_ty`
/// matches neither shape.
pub(crate) fn resolve_field_expr(
    field_ty: &Type,
    container_expr: &TokenStream,
) -> Option<TokenStream> {
    resolve_optional_arc_expr(field_ty, container_expr)
        .or_else(|| resolve_arc_expr(field_ty, container_expr))
}

/// Same idea as [`resolve_arc_expr`], for a field/param declared as `Vec<Arc<Inner>>`,
/// via `resolve_all`. `None` if `field_ty` isn't `Vec<Arc<...>>`.
pub(crate) fn resolve_all_vec_expr(
    field_ty: &Type,
    container_expr: &TokenStream,
) -> Option<TokenStream> {
    let arc_ty = vec_inner(field_ty)?;
    let inner = arc_inner(arc_ty)?;
    Some(if is_trait_object(inner) {
        quote! {
            #container_expr.resolve_all::<#arc_ty>().await?
                .into_iter()
                .map(|__item| (*__item).clone())
                .collect::<::std::vec::Vec<_>>()
        }
    } else {
        quote! { #container_expr.resolve_all::<#inner>().await? }
    })
}
