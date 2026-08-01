use rudi::Container;

#[derive(Debug)]
struct MyConfig {
    level: String,
}

#[tokio::test]
async fn resolve_optional_without_registration_returns_none() {
    let c = Container::new();
    let result = c.resolve_optional::<MyConfig>().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn resolve_optional_with_registration_returns_some() {
    let c = Container::new();
    c.register_instance(MyConfig {
        level: "info".into(),
    });

    let result = c.resolve_optional::<MyConfig>().await.unwrap();
    assert_eq!(result.unwrap().level, "info");
}

#[derive(Debug, thiserror::Error)]
#[error("boom")]
struct BoomError;

#[tokio::test]
async fn resolve_optional_propagates_build_failure() {
    let c = Container::new();
    c.register_singleton::<MyConfig, _, _, BoomError>(|_c| async { Err(BoomError) });

    let result = c.resolve_optional::<MyConfig>().await;
    assert!(result.is_err(), "a build failure must not become None");
}
