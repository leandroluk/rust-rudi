use rudi::{Container, DebugKind};

#[derive(Debug)]
struct Config {
    #[allow(dead_code)]
    level: String,
}
#[derive(Debug)]
struct Adapter;
#[derive(Debug)]
struct Leaf;

#[tokio::test]
async fn debug_entries_lists_every_registration_mode() {
    let c = Container::new();
    c.register_instance(Config {
        level: "info".into(),
    });
    c.register_transient::<Adapter, _, _, std::convert::Infallible>(|_c| async { Ok(Adapter) });
    c.register_singleton::<Leaf, _, _, std::convert::Infallible>(|_c| async { Ok(Leaf) });

    let entries = c.debug_entries();
    assert_eq!(entries.len(), 3);

    let kinds: Vec<_> = entries.iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&DebugKind::Instance));
    assert!(kinds.contains(&DebugKind::Transient));
    assert!(kinds.contains(&DebugKind::Singleton));
}

#[derive(Debug)]
struct Parent;
#[derive(Debug)]
struct Child;

#[tokio::test]
async fn debug_edges_captures_nested_resolution() {
    let c = Container::new();

    c.register_instance(Child);
    c.register_singleton::<Parent, _, _, std::convert::Infallible>(|c| async move {
        c.resolve::<Child>().await.unwrap();
        Ok(Parent)
    });

    assert!(c.debug_edges().is_empty(), "no resolution happened yet");

    c.resolve::<Parent>().await.unwrap();

    let edges = c.debug_edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].0.type_name, std::any::type_name::<Parent>());
    assert_eq!(edges[0].1.type_name, std::any::type_name::<Child>());
}
