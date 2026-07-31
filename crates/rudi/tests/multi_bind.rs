use std::future::Future;
use std::sync::Arc;

use rudi::{Container, Injectable};

trait Pingable: Send + Sync {
    fn label(&self) -> &'static str;
}

struct PingA;
impl Injectable for PingA {
    type Error = std::convert::Infallible;
    type Port = dyn Pingable;

    fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(PingA) }
    }

    fn into_port(built: Arc<Self>) -> Arc<dyn Pingable> {
        built
    }
}
impl Pingable for PingA {
    fn label(&self) -> &'static str {
        "a"
    }
}

struct PingB;
impl Injectable for PingB {
    type Error = std::convert::Infallible;
    type Port = dyn Pingable;

    fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(PingB) }
    }

    fn into_port(built: Arc<Self>) -> Arc<dyn Pingable> {
        built
    }
}
impl Pingable for PingB {
    fn label(&self) -> &'static str {
        "b"
    }
}

#[tokio::test]
async fn bind_many_accumulates_in_order() {
    let c = Container::new();
    c.bind_many::<PingA, dyn Pingable>();
    c.bind_many::<PingB, dyn Pingable>();

    let all = c.resolve_all::<Arc<dyn Pingable>>().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].label(), "a");
    assert_eq!(all[1].label(), "b");
}

#[tokio::test]
async fn resolve_all_without_bind_many_returns_empty_not_error() {
    let c = Container::new();
    let all = c.resolve_all::<Arc<dyn Pingable>>().await.unwrap();
    assert!(all.is_empty());
}

trait OtherPort: Send + Sync {}
impl OtherPort for PingA {}
impl Injectable for PingASingle {
    type Error = std::convert::Infallible;
    type Port = dyn OtherPort;

    fn build(_c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send {
        async { Ok(PingASingle) }
    }

    fn into_port(built: Arc<Self>) -> Arc<dyn OtherPort> {
        built
    }
}
struct PingASingle;
impl OtherPort for PingASingle {}

#[tokio::test]
async fn bind_and_bind_many_are_independent_storage() {
    let c = Container::new();

    // mesma "família" de tipo (PingA) usada em bind_many pra Pingable...
    c.bind_many::<PingA, dyn Pingable>();
    // ...e um tipo distinto (PingASingle) usado em bind normal pra outra porta.
    c.bind::<PingASingle, dyn OtherPort>();

    let all = c.resolve_all::<Arc<dyn Pingable>>().await.unwrap();
    assert_eq!(all.len(), 1);

    let single = c.resolve::<Arc<dyn OtherPort>>().await;
    assert!(single.is_ok());
}

#[tokio::test]
async fn resolve_all_caches_each_slot_individually() {
    let c = Container::new();

    c.bind_many::<PingA, dyn Pingable>();
    let first = c.resolve_all::<Arc<dyn Pingable>>().await.unwrap();
    let second = c.resolve_all::<Arc<dyn Pingable>>().await.unwrap();

    assert!(Arc::ptr_eq(&first[0], &second[0]));
}
