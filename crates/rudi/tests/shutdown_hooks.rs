use std::sync::{Arc, Mutex};

use rudi::Container;

#[tokio::test]
async fn hooks_run_in_reverse_registration_order() {
    let c = Container::new();
    let order = Arc::new(Mutex::new(Vec::new()));

    for id in 1..=3 {
        let order = order.clone();
        c.on_shutdown(move |_c| {
            let order = order.clone();
            async move {
                order.lock().unwrap().push(id);
            }
        });
    }

    c.shutdown().await;

    assert_eq!(*order.lock().unwrap(), vec![3, 2, 1]);
}

#[tokio::test]
async fn shutdown_without_hooks_does_not_panic() {
    let c = Container::new();
    c.shutdown().await;
}

#[tokio::test]
async fn hooks_run_sequentially_waiting_for_each() {
    let c = Container::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    let log1 = log.clone();
    c.on_shutdown(move |_c| {
        let log = log1.clone();
        async move {
            tokio::task::yield_now().await;
            log.lock().unwrap().push("first-done");
        }
    });
    let log2 = log.clone();
    c.on_shutdown(move |_c| {
        let log = log2.clone();
        async move {
            log.lock().unwrap().push("second-done");
        }
    });

    c.shutdown().await;

    // registro: first, second — execução reversa: second roda 1º, first 2º.
    assert_eq!(*log.lock().unwrap(), vec!["second-done", "first-done"]);
}
