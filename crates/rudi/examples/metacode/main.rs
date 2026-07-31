//! Reprodução literal da árvore descrita em METACODE.md, usando as macros reais
//! de `rudi-macros` (`#[injectable]`, `#[derive(Injectable)]`). 1 desvio documentado
//! em relação ao arquivo original (restrição de linguagem, não escolha):
//! `#[injectable]` decora o bloco `impl` inteiro, não a fn `build` isolada — ver
//! `.specs/features/di-macros/design.md`. Construtores ficam 100% síncronos via
//! `#[inject]` nos parâmetros (`fn build(#[inject] config: Arc<Config>) -> Self`) —
//! o async de `resolve()` fica todo escondido dentro do `Injectable::build` gerado.

mod domain;
mod infra;

use std::sync::Arc;

use domain::port::{DatabasePort, LoggerPort};

#[tokio::main]
async fn main() {
    let c1 = rudi::container();
    infra::init(&c1, "postgres", "slog");
    c1.resolve::<Arc<dyn LoggerPort>>()
        .await
        .unwrap()
        .info("Hello World!");
    c1.resolve::<Arc<dyn DatabasePort>>()
        .await
        .unwrap()
        .ping()
        .unwrap();

    println!("metacode example ok (postgres + slog via rudi container global)");
}
