use std::sync::Arc;

use rudi::Container;

trait Port: Send + Sync {
    fn label(&self) -> &'static str;
}

struct ImplA;
impl Port for ImplA {
    fn label(&self) -> &'static str {
        "a"
    }
}

struct ImplB;
impl Port for ImplB {
    fn label(&self) -> &'static str {
        "b"
    }
}

#[tokio::test]
async fn last_bind_wins() {
    let c = Container::new();

    c.bind_with::<dyn Port, _, _, std::convert::Infallible>(|_c| async {
        Ok(Arc::new(ImplA) as Arc<dyn Port>)
    });
    c.bind_with::<dyn Port, _, _, std::convert::Infallible>(|_c| async {
        Ok(Arc::new(ImplB) as Arc<dyn Port>)
    });

    let resolved = c.resolve::<Arc<dyn Port>>().await.unwrap();
    assert_eq!(resolved.label(), "b");
}

#[tokio::test]
async fn resolve_without_bind_returns_not_found() {
    let c = Container::new();
    let err = match c.resolve::<Arc<dyn Port>>().await {
        Err(e) => e,
        Ok(_) => panic!("expected NotFound error"),
    };
    assert!(err.to_string().contains("Port"));
}
