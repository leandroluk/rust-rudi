use rudi::{inject, Container};

// #[inject] sempre usa o container GLOBAL (rudi::container()) — cada teste usa seu
// próprio tipo marcador pra não colidir com outros testes rodando em paralelo no
// mesmo processo (mesma cautela do TESTING.md, adaptada porque aqui o alvo do teste
// é justamente o comportamento do container global).

#[derive(Debug, PartialEq)]
struct MarkerA(u32);

#[inject]
fn register_marker_a(#[container] c: &Container) {
    c.register_instance(MarkerA(99));
}

#[tokio::test]
async fn inject_removes_param_and_resolves_container() {
    register_marker_a();

    let resolved = rudi::container().resolve::<MarkerA>().await.unwrap();
    assert_eq!(*resolved, MarkerA(99));
}

// alias de import — prova que #[inject] não depende do nome do tipo (marker attribute).
use rudi::Container as AliasedContainer;

#[derive(Debug, PartialEq)]
struct MarkerB(u32);

#[inject]
async fn async_inject(#[container] c: &AliasedContainer) -> MarkerB {
    c.register_instance(MarkerB(7));
    let resolved = c.resolve::<MarkerB>().await.unwrap();
    MarkerB(resolved.0)
}

#[tokio::test]
async fn inject_works_with_type_alias() {
    let result = async_inject().await;
    assert_eq!(result, MarkerB(7));
}

#[derive(Debug, PartialEq)]
struct MarkerC(bool);

#[inject]
fn owned_inject(#[container] c: Container) -> bool {
    c.register_instance(MarkerC(true));
    true
}

#[tokio::test]
async fn inject_works_with_owned_container_param() {
    assert!(owned_inject());
}
