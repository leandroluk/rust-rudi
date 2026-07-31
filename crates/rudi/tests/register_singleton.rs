use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use rudi::Container;

#[derive(Debug)]
struct Counter(u32);

#[tokio::test]
async fn singleton_caches_after_first_resolve() {
    let c = Container::new();
    let calls = Arc::new(AtomicU32::new(0));

    let calls_clone = calls.clone();
    c.register_singleton::<Counter, _, _, std::convert::Infallible>(move |_c| {
        let calls = calls_clone.clone();
        async move {
            let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(Counter(n))
        }
    });

    let first = c.resolve::<Counter>().await.unwrap();
    let second = c.resolve::<Counter>().await.unwrap();

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.0, 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn singleton_builder_runs_once_under_concurrency() {
    let c = Container::new();
    let calls = Arc::new(AtomicU32::new(0));

    let calls_clone = calls.clone();
    c.register_singleton::<Counter, _, _, std::convert::Infallible>(move |_c| {
        let calls = calls_clone.clone();
        async move {
            let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
            tokio::task::yield_now().await;
            Ok(Counter(n))
        }
    });

    let mut handles = Vec::new();
    for _ in 0..10 {
        let c = c.clone();
        handles.push(tokio::spawn(async move { c.resolve::<Counter>().await.unwrap() }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn singleton_named_variants_are_independent_caches() {
    let c = Container::new();

    c.register_singleton_named::<Counter, _, _, std::convert::Infallible>("a", |_c| async {
        Ok(Counter(10))
    });
    c.register_singleton_named::<Counter, _, _, std::convert::Infallible>("b", |_c| async {
        Ok(Counter(20))
    });

    let a = c.resolve_named::<Counter>("a").await.unwrap();
    let b = c.resolve_named::<Counter>("b").await.unwrap();

    assert_eq!(a.0, 10);
    assert_eq!(b.0, 20);
}
