use std::future::Future;
use std::sync::Arc;

use rudi::{Container, Injectable};

#[derive(Debug)]
struct StandaloneConfig {
    value: u32,
}

impl Injectable for StandaloneConfig {
    type Error = std::convert::Infallible;
    type Port = StandaloneConfig;

    fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(StandaloneConfig { value: 7 }) }
    }

    fn into_port(built: Arc<Self>) -> Arc<Self::Port> {
        built
    }
}

#[tokio::test]
async fn register_singleton_injectable_resolves() {
    let c = Container::new();
    c.register_singleton_injectable::<StandaloneConfig>();

    let resolved = c.resolve::<StandaloneConfig>().await.unwrap();
    assert_eq!(resolved.value, 7);
}

trait Port: Send + Sync {
    fn label(&self) -> &'static str;
}

struct ImplA;
impl Injectable for ImplA {
    type Error = std::convert::Infallible;
    type Port = dyn Port;

    fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(ImplA) }
    }

    fn into_port(built: Arc<Self>) -> Arc<dyn Port> {
        built
    }
}
impl Port for ImplA {
    fn label(&self) -> &'static str {
        "a"
    }
}

struct ImplB;
impl Injectable for ImplB {
    type Error = std::convert::Infallible;
    type Port = dyn Port;

    fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(ImplB) }
    }

    fn into_port(built: Arc<Self>) -> Arc<dyn Port> {
        built
    }
}
impl Port for ImplB {
    fn label(&self) -> &'static str {
        "b"
    }
}

#[tokio::test]
async fn bind_resolves_via_injectable() {
    let c = Container::new();
    c.bind::<ImplA, dyn Port>();

    let resolved = c.resolve::<Arc<dyn Port>>().await.unwrap();
    assert_eq!(resolved.label(), "a");
}

#[tokio::test]
async fn bind_last_wins() {
    let c = Container::new();
    c.bind::<ImplA, dyn Port>();
    c.bind::<ImplB, dyn Port>();

    let resolved = c.resolve::<Arc<dyn Port>>().await.unwrap();
    assert_eq!(resolved.label(), "b");
}
