use std::sync::Arc;

use rudi::Container;

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
