use rudi::Container;

#[derive(Debug, PartialEq)]
struct Config {
    value: u32,
}

#[tokio::test]
async fn register_and_resolve_instance() {
    let c = Container::new();
    c.register_instance(Config { value: 42 });

    let resolved = c.resolve::<Config>().await.unwrap();
    assert_eq!(resolved.value, 42);
}

#[tokio::test]
async fn resolve_without_registration_returns_not_found() {
    let c = Container::new();

    let err = c.resolve::<Config>().await.unwrap_err();
    assert!(err.to_string().contains("Config"));
}

#[tokio::test]
async fn empty_container_resolve_does_not_panic() {
    let c = Container::new();
    let result = c.resolve::<Config>().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn named_and_unnamed_instances_coexist() {
    let c = Container::new();
    c.register_instance(Config { value: 1 });
    c.register_instance_named("primary", Config { value: 2 });

    let unnamed = c.resolve::<Config>().await.unwrap();
    let named = c.resolve_named::<Config>("primary").await.unwrap();

    assert_eq!(unnamed.value, 1);
    assert_eq!(named.value, 2);
}

#[tokio::test]
async fn resolve_named_with_missing_name_returns_not_found() {
    let c = Container::new();
    c.register_instance_named("primary", Config { value: 1 });

    let err = c.resolve_named::<Config>("replica").await.unwrap_err();
    assert!(err.to_string().contains("replica"));
}
