use std::sync::Arc;

use rudi::{injectable, Container};

#[derive(Debug)]
struct LoggerCfg {
    level: String,
}

#[derive(Debug)]
struct DbCfg {
    uri: String,
}

#[derive(rudi::Injectable)]
struct Adapter {
    logger: Arc<LoggerCfg>,
    db: Arc<DbCfg>,
}

#[tokio::test]
async fn derive_named_fields_resolves_each() {
    let c = Container::new();
    c.register_instance(LoggerCfg {
        level: "info".into(),
    });
    c.register_instance(DbCfg {
        uri: "postgres://x".into(),
    });

    c.register_singleton_injectable::<Adapter>();
    let resolved = c.resolve::<Adapter>().await.unwrap();

    assert_eq!(resolved.logger.level, "info");
    assert_eq!(resolved.db.uri, "postgres://x");
}

#[derive(rudi::Injectable)]
struct TupleAdapter(Arc<LoggerCfg>, Arc<DbCfg>);

#[tokio::test]
async fn derive_tuple_fields_resolves_positionally() {
    let c = Container::new();
    c.register_instance(LoggerCfg {
        level: "debug".into(),
    });
    c.register_instance(DbCfg {
        uri: "mongodb://y".into(),
    });

    c.register_singleton_injectable::<TupleAdapter>();
    let resolved = c.resolve::<TupleAdapter>().await.unwrap();

    assert_eq!(resolved.0.level, "debug");
    assert_eq!(resolved.1.uri, "mongodb://y");
}

#[derive(rudi::Injectable)]
struct Empty;

#[tokio::test]
async fn derive_unit_struct_builds_without_resolving() {
    let c = Container::new();
    c.register_singleton_injectable::<Empty>();
    let resolved = c.resolve::<Empty>().await;
    assert!(resolved.is_ok());
}

#[derive(rudi::Injectable)]
struct MissingDep {
    #[allow(dead_code)]
    logger: Arc<LoggerCfg>,
}

#[tokio::test]
async fn derive_missing_field_propagates_not_found() {
    let c = Container::new();
    c.register_singleton_injectable::<MissingDep>();
    let err = c.resolve::<MissingDep>().await;
    assert!(err.is_err());
}

// Regressão: campo `Arc<dyn Trait>` nunca foi testado antes — `resolve::<dyn Trait>()`
// não compila (T: Sized implícito), precisa resolver via `Arc<dyn Trait>` + achatar
// o double-Arc resultante. Ver .specs/features/param-inject/spec.md.
trait Port: Send + Sync {
    fn label(&self) -> &'static str;
}

struct PortImpl;
impl Port for PortImpl {
    fn label(&self) -> &'static str {
        "impl"
    }
}
#[injectable(dyn Port)]
impl PortImpl {
    fn build(#[container] _c: &Container) -> Self {
        PortImpl
    }
}

#[derive(rudi::Injectable)]
struct WithTraitObjectField {
    port: Arc<dyn Port>,
}

#[tokio::test]
async fn derive_trait_object_field_resolves() {
    let c = Container::new();
    c.bind::<PortImpl, dyn Port>();
    c.register_singleton_injectable::<WithTraitObjectField>();

    let resolved = c.resolve::<WithTraitObjectField>().await.unwrap();
    assert_eq!(resolved.port.label(), "impl");
}

#[derive(rudi::Injectable)]
struct WithOptionalField {
    logger: Option<Arc<LoggerCfg>>,
}

#[tokio::test]
async fn derive_optional_field_present() {
    let c = Container::new();
    c.register_instance(LoggerCfg {
        level: "opt".into(),
    });
    c.register_singleton_injectable::<WithOptionalField>();

    let resolved = c.resolve::<WithOptionalField>().await.unwrap();
    assert_eq!(resolved.logger.as_ref().unwrap().level, "opt");
}

#[tokio::test]
async fn derive_optional_field_absent() {
    let c = Container::new();
    c.register_singleton_injectable::<WithOptionalField>();

    let resolved = c.resolve::<WithOptionalField>().await.unwrap();
    assert!(resolved.logger.is_none());
}
