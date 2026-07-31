use std::future::Future;

use crate::Container;

/// Cria um [`Container`] local (nunca o global) pra uso isolado em testes.
/// Cada chamada tem seu próprio container — sem vazamento de estado entre testes
/// rodando em paralelo, e sem tocar em `rudi::container()`.
pub async fn with_container<F, Fut, R>(f: F) -> R
where
    F: FnOnce(Container) -> Fut,
    Fut: Future<Output = R>,
{
    let c = Container::new();
    f(c).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_isolated_container_per_call() {
        let a = with_container(|c| async move {
            c.register_instance(1u32);
            *c.resolve::<u32>().await.unwrap()
        })
        .await;

        let b = with_container(|c| async move { c.resolve::<u32>().await.is_err() }).await;

        assert_eq!(a, 1);
        assert!(b, "segundo container não deve ver o registro do primeiro");
    }

    #[derive(Debug)]
    struct GlobalIsolationMarker;

    #[tokio::test]
    async fn does_not_touch_global_container() {
        with_container(|c| async move {
            c.register_instance(GlobalIsolationMarker);
        })
        .await;

        let result = crate::container().resolve::<GlobalIsolationMarker>().await;
        assert!(
            result.is_err(),
            "container global não deve ter sido afetado"
        );
    }

    #[tokio::test]
    async fn parallel_calls_do_not_interfere() {
        let (a, b) = tokio::join!(
            with_container(|c| async move {
                c.register_instance("a");
                c.resolve::<&str>().await.unwrap()
            }),
            with_container(|c| async move {
                c.register_instance("b");
                c.resolve::<&str>().await.unwrap()
            }),
        );

        assert_eq!(*a, "a");
        assert_eq!(*b, "b");
    }
}
