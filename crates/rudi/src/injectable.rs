use std::future::Future;
use std::sync::Arc;

use crate::container::Container;

/// Contrato implementado por `#[injectable]`/`#[derive(Injectable)]`, consumido por
/// [`Container::bind`](crate::Container::bind) e
/// [`Container::register_singleton_injectable`](crate::Container::register_singleton_injectable).
pub trait Injectable: Sized + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    type Port: ?Sized + Send + Sync + 'static;

    fn build(c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send;

    fn into_port(built: Arc<Self>) -> Arc<Self::Port>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Foo;

    impl Injectable for Foo {
        type Error = std::convert::Infallible;
        type Port = Foo;

        fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
            async { Ok(Foo) }
        }

        fn into_port(built: Arc<Self>) -> Arc<Self::Port> {
            built
        }
    }

    #[tokio::test]
    async fn manual_impl_builds() {
        let result = Foo::build(Container::new()).await;
        assert!(result.is_ok());
    }
}
