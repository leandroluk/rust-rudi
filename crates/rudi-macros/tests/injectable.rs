use std::sync::Arc;

use rudi::{injectable, Container};

struct SyncInfallible;

#[injectable]
impl SyncInfallible {
    fn build(_c: &Container) -> Self {
        SyncInfallible
    }
}

#[tokio::test]
async fn sync_infallible_build() {
    let c = Container::new();
    c.register_singleton_injectable::<SyncInfallible>();
    let resolved = c.resolve::<SyncInfallible>().await;
    assert!(resolved.is_ok());
}

#[derive(Debug, thiserror::Error)]
#[error("boom")]
struct BoomError;

struct AsyncFallible {
    value: u32,
}

#[injectable]
impl AsyncFallible {
    async fn build(_c: &Container) -> Result<Self, BoomError> {
        Ok(AsyncFallible { value: 9 })
    }
}

#[tokio::test]
async fn async_fallible_build() {
    let c = Container::new();
    c.register_singleton_injectable::<AsyncFallible>();
    let resolved = c.resolve::<AsyncFallible>().await.unwrap();
    assert_eq!(resolved.value, 9);
}

trait Port: Send + Sync {
    fn label(&self) -> &'static str;
}

struct PortImpl;

#[injectable(dyn Port)]
impl PortImpl {
    fn build(_c: &Container) -> Self {
        PortImpl
    }
}

impl Port for PortImpl {
    fn label(&self) -> &'static str {
        "impl"
    }
}

#[tokio::test]
async fn injectable_with_port_binds() {
    let c = Container::new();
    c.bind::<PortImpl, dyn Port>();
    let resolved = c.resolve::<Arc<dyn Port>>().await.unwrap();
    assert_eq!(resolved.label(), "impl");
}

struct OwnedContainerParam;

#[injectable]
impl OwnedContainerParam {
    fn build(_c: Container) -> Self {
        OwnedContainerParam
    }
}

#[tokio::test]
async fn owned_container_param_build() {
    let c = Container::new();
    c.register_singleton_injectable::<OwnedContainerParam>();
    let resolved = c.resolve::<OwnedContainerParam>().await;
    assert!(resolved.is_ok());
}
