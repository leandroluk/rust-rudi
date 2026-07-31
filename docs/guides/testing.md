# Testing

## Never touch the global container in a test

`rudi::container()` is a single `OnceLock<Container>` for the whole process — every `#[tokio::test]` in the same test binary shares it, and `cargo test` runs tests concurrently by default. A test that registers `MyConfig` on the global container can leak that registration into a completely unrelated test running at the same time, or "last registration wins" it away entirely.

## `Container::new()` — a plain local container

The simplest fix: never call `rudi::container()` in test code, use `Container::new()` instead:

```rust
#[tokio::test]
async fn my_test() {
    let c = Container::new();
    c.register_instance(MyConfig::default());

    let resolved = c.resolve::<MyConfig>().await.unwrap();
    assert_eq!(resolved.level, "info");
}
```

## `with_container` — the same thing, as a closure

`rudi::testing::with_container` does exactly this, wrapped in a helper that reads a little closer to "here's my test's isolated scope":

```rust
use rudi::testing::with_container;

#[tokio::test]
async fn my_test() {
    with_container(|c| async move {
        c.register_instance(MyConfig::default());
        let resolved = c.resolve::<MyConfig>().await.unwrap();
        assert_eq!(resolved.level, "info");
    })
    .await;
}
```

Each call gets its own container — 2 calls to `with_container`, even running concurrently via `tokio::join!`, never see each other's registrations:

```rust
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
```

Neither `Container::new()` nor `with_container` ever touch `rudi::container()` — the global container is left exactly as it was before the test ran.
