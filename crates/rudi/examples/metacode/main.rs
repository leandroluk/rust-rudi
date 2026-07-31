//! Reprodução literal da árvore descrita em METACODE.md, usando as macros reais
//! de `rudi-macros` (`#[injectable]`, `#[derive(Injectable)]`). 2 desvios documentados
//! em relação ao arquivo original (ambos por restrição de linguagem, não escolha):
//! 1. `#[injectable]` decora o bloco `impl` inteiro, não a fn `build` isolada — ver
//!    `.specs/features/di-macros/design.md`.
//! 2. Construtores resolvendo do container são `async fn build`, não `fn build`
//!    síncrona — `resolve()` do core é sempre async (decisão de M1).

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
