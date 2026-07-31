use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use rudi::Container;

#[derive(Debug)]
struct Counter(u32);

#[tokio::test]
async fn transient_reruns_builder_on_every_resolve() {
    let c = Container::new();
    let calls = Arc::new(AtomicU32::new(0));

    let calls_clone = calls.clone();
    c.register_transient::<Counter, _, _, std::convert::Infallible>(move |_c| {
        let calls = calls_clone.clone();
        async move {
            let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(Counter(n))
        }
    });

    let first = c.resolve::<Counter>().await.unwrap();
    let second = c.resolve::<Counter>().await.unwrap();

    assert_eq!(first.0, 1);
    assert_eq!(second.0, 2);
}

#[tokio::test]
async fn transient_named_variants_are_independent() {
    let c = Container::new();

    c.register_transient_named::<Counter, _, _, std::convert::Infallible>("a", |_c| async {
        Ok(Counter(100))
    });
    c.register_transient_named::<Counter, _, _, std::convert::Infallible>("b", |_c| async {
        Ok(Counter(200))
    });

    let a = c.resolve_named::<Counter>("a").await.unwrap();
    let b = c.resolve_named::<Counter>("b").await.unwrap();

    assert_eq!(a.0, 100);
    assert_eq!(b.0, 200);
}

#[derive(Debug, thiserror::Error)]
#[error("build failed on purpose")]
struct BuildError;

#[tokio::test]
async fn transient_builder_error_propagates_as_build_failed() {
    let c = Container::new();
    c.register_transient::<Counter, _, _, BuildError>(|_c| async { Err(BuildError) });

    let err = c.resolve::<Counter>().await.unwrap_err();
    assert!(err.to_string().contains("build failed on purpose"));
}
