use rudi::Container;

#[derive(Debug)]
struct A;
#[derive(Debug)]
struct B;

#[tokio::test]
async fn direct_cycle_returns_error_not_deadlock() {
    let c = Container::new();

    // E = RudiError (não Infallible) pra poder propagar o erro de dentro do builder
    // via `?` — cada register_singleton embrulha o Err do builder em BuildFailed, então
    // o erro final é BuildFailed(A) -> BuildFailed(B) -> CircularDependency, aninhado,
    // não CircularDependency direto no topo. Checa via string em vez de match exato.
    c.register_singleton::<A, _, _, rudi::RudiError>(|c| async move {
        c.resolve::<B>().await?;
        Ok(A)
    });
    c.register_singleton::<B, _, _, rudi::RudiError>(|c| async move {
        c.resolve::<A>().await?;
        Ok(B)
    });

    let result = c.resolve::<A>().await;
    let err = result.expect_err("cycle must surface as an error, not deadlock");
    assert!(
        err.to_string().contains("circular dependency detected"),
        "expected circular dependency in error chain, got: {err}"
    );
}

#[derive(Debug)]
struct C1;
#[derive(Debug)]
struct C2;
#[derive(Debug)]
struct C3;

#[tokio::test]
async fn non_cyclic_chain_resolves_normally() {
    let c = Container::new();

    c.register_instance(C1);
    c.register_singleton::<C2, _, _, std::convert::Infallible>(|c| async move {
        c.resolve::<C1>().await.unwrap();
        Ok(C2)
    });
    c.register_singleton::<C3, _, _, std::convert::Infallible>(|c| async move {
        c.resolve::<C2>().await.unwrap();
        Ok(C3)
    });

    let result = c.resolve::<C3>().await;
    assert!(result.is_ok());
}

#[derive(Debug)]
struct Unrelated1;
#[derive(Debug)]
struct Unrelated2;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_unrelated_resolutions_do_not_interfere() {
    let c = Container::new();

    c.register_singleton::<Unrelated1, _, _, std::convert::Infallible>(|_c| async {
        Ok(Unrelated1)
    });
    c.register_singleton::<Unrelated2, _, _, std::convert::Infallible>(|_c| async {
        Ok(Unrelated2)
    });

    let (r1, r2) = tokio::join!(c.resolve::<Unrelated1>(), c.resolve::<Unrelated2>());
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}
