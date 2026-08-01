use std::sync::Arc;

use rudi::{injectable, Container};

struct SyncInfallible;

#[injectable]
impl SyncInfallible {
    fn build(#[container] _c: &Container) -> Self {
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
    async fn build(#[container] _c: &Container) -> Result<Self, BoomError> {
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
    fn build(#[container] _c: &Container) -> Self {
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
    fn build(#[container] _c: Container) -> Self {
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

#[derive(Debug)]
struct MyConfig {
    level: String,
}

struct Adapter {
    config: Arc<MyConfig>,
}

#[injectable]
impl Adapter {
    fn build(#[inject] config: Arc<MyConfig>) -> Self {
        Self { config }
    }
}

#[tokio::test]
async fn inject_concrete_type_param() {
    let c = Container::new();
    c.register_instance(MyConfig {
        level: "info".into(),
    });
    c.register_singleton_injectable::<Adapter>();

    let resolved = c.resolve::<Adapter>().await.unwrap();
    assert_eq!(resolved.config.level, "info");
}

trait InjectablePort: Send + Sync {
    fn label(&self) -> &'static str;
}

struct PortDep;
impl InjectablePort for PortDep {
    fn label(&self) -> &'static str {
        "dep"
    }
}
#[injectable(dyn InjectablePort)]
impl PortDep {
    fn build(#[container] _c: &Container) -> Self {
        PortDep
    }
}

struct ConsumesTraitObject {
    dep: Arc<dyn InjectablePort>,
}

#[injectable]
impl ConsumesTraitObject {
    fn build(#[inject] dep: Arc<dyn InjectablePort>) -> Self {
        Self { dep }
    }
}

#[tokio::test]
async fn inject_trait_object_param() {
    let c = Container::new();
    c.bind::<PortDep, dyn InjectablePort>();
    c.register_singleton_injectable::<ConsumesTraitObject>();

    let resolved = c.resolve::<ConsumesTraitObject>().await.unwrap();
    assert_eq!(resolved.dep.label(), "dep");
}

struct PingA;
impl InjectablePort for PingA {
    fn label(&self) -> &'static str {
        "a"
    }
}
struct PingB;
impl InjectablePort for PingB {
    fn label(&self) -> &'static str {
        "b"
    }
}
#[injectable(dyn InjectablePort)]
impl PingA {
    fn build(#[container] _c: &Container) -> Self {
        PingA
    }
}
#[injectable(dyn InjectablePort)]
impl PingB {
    fn build(#[container] _c: &Container) -> Self {
        PingB
    }
}

struct HealthcheckUsecase {
    pingables: Vec<Arc<dyn InjectablePort>>,
}

#[injectable]
impl HealthcheckUsecase {
    fn new(#[inject_all] pingables: Vec<Arc<dyn InjectablePort>>) -> Self {
        Self { pingables }
    }
}

#[tokio::test]
async fn inject_all_param() {
    let c = Container::new();
    c.bind_many::<PingA, dyn InjectablePort>();
    c.bind_many::<PingB, dyn InjectablePort>();
    c.register_singleton_injectable::<HealthcheckUsecase>();

    let resolved = c.resolve::<HealthcheckUsecase>().await.unwrap();
    let labels: Vec<_> = resolved.pingables.iter().map(|p| p.label()).collect();
    assert_eq!(labels, vec!["a", "b"]);
}

struct MixedParams {
    config: Arc<MyConfig>,
}

#[injectable]
impl MixedParams {
    fn build(#[inject] config: Arc<MyConfig>, #[container] _c: &Container) -> Self {
        Self { config }
    }
}

#[tokio::test]
async fn mixed_inject_and_container_params() {
    let c = Container::new();
    c.register_instance(MyConfig {
        level: "debug".into(),
    });
    c.register_singleton_injectable::<MixedParams>();

    let resolved = c.resolve::<MixedParams>().await.unwrap();
    assert_eq!(resolved.config.level, "debug");
}

/// Fica fora do `impl` decorado — só serve pra chamar o construtor gerado
/// manualmente, síncrono, sem `.await`, provando que a fn original não foi
/// tocada pela macro (só o `impl Injectable` gerado ao lado é que é async).
#[test]
fn ctor_is_callable_manually_without_await() {
    let config = Arc::new(MyConfig {
        level: "manual".into(),
    });
    let adapter = Adapter { config };
    assert_eq!(adapter.config.level, "manual");
}

struct OptionalDep {
    config: Option<Arc<MyConfig>>,
}

#[injectable]
impl OptionalDep {
    fn build(#[inject] config: Option<Arc<MyConfig>>) -> Self {
        Self { config }
    }
}

#[tokio::test]
async fn inject_optional_present() {
    let c = Container::new();
    c.register_instance(MyConfig {
        level: "opt".into(),
    });
    c.register_singleton_injectable::<OptionalDep>();

    let resolved = c.resolve::<OptionalDep>().await.unwrap();
    assert_eq!(resolved.config.as_ref().unwrap().level, "opt");
}

#[tokio::test]
async fn inject_optional_absent() {
    let c = Container::new();
    c.register_singleton_injectable::<OptionalDep>();

    let resolved = c.resolve::<OptionalDep>().await.unwrap();
    assert!(resolved.config.is_none());
}
