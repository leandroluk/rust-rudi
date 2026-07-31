# Testing

## Test Coverage Matrix

| Code Layer | Required Test Type | Parallel-Safe |
| --- | --- | --- |
| Módulo interno (`container.rs`, `error.rs` — lógica de Key/Entry/downcast) | unit (inline `#[cfg(test)] mod tests`) | Yes |
| API pública do crate (fluxo register/resolve ponta a ponta) | integration (`tests/*.rs`) | Yes — cada teste usa seu próprio `Container::new()`, sem estado global compartilhado |
| Macros (`rudi-macros`, futuro M2) | integration (`tests/` com `trybuild` ou expansão real) | Yes |

## Gate Check Commands

- **quick**: `cargo test -p rudi`
- **full**: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`

## Notes

- Testes de container NUNCA usam `rudi::container()` (o global) — sempre `Container::new()` local, pra evitar vazamento de estado entre testes rodando em paralelo (`cargo test` roda testes em threads paralelas por padrão).
