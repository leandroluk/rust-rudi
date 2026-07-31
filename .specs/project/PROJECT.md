# rudi

**Vision:** Lib rust de injeção de dependência type-safe, baseada em container global (1 por processo), macros/attributes pra registro via instância, factory, singleton ou bind de porta (trait).
**For:** Devs rust (ecossistema geral, publicação crates.io) que usam padrão hexagonal/ports & adapters e querem DI sem boilerplate manual de wiring.
**Solves:** Wiring manual de dependências (config → adapter → porta) espalhado e repetitivo; seleção de implementação por env (ex: postgres vs mongodb) sem acoplar quem consome à impl concreta.

## Goals

- API pública mínima e type-safe: erros de wiring pegos em compile-time sempre que possível — sucesso: exemplos do METACODE.md compilam e rodam sem alterar assinatura pública proposta.
- Zero acoplamento a env/config: lib nunca lê `std::env` — sucesso: nenhuma chamada a `std::env` fora de código de exemplo/consumidor.
- Resolução assíncrona garantida (mesmo pra tipos síncronos) — sucesso: `resolve` é sempre `async fn`/retorna `Future`, container resolve grafo de dependências fazendo await onde precisar.

## Tech Stack

**Core:**

- Language: Rust, edition 2021, MSRV 1.75+
- Crates: `rudi` (core, runtime) + `rudi-macros` (proc-macro: `#[injectable]`, `#[inject]`, `#[derive(Injectable)]`)

**Key dependencies:**

- `syn` / `quote` / `proc-macro2` — proc-macros
- `tokio` (dev-dependency dos exemplos; core não deve forçar runtime específico se der pra evitar — decidir em design)
- `async-trait` (se precisar, avaliar em design conforme suporte nativo a async fn em traits)
- `garde` — só nos exemplos (validação de config), não é dependência do core

## Scope

**v1 includes:**

- Container global lazy (`rudi::container()`), 1 por processo
- `register_instance`, `register_factory`, `register_singleton`, `register_transient`
- `bind::<Impl, dyn Port>()` — registro de impl contra trait/porta
- `resolve::<T>()` assíncrono (grafo de dependências resolvido via await)
- Named bindings — múltiplas instâncias do mesmo tipo/provider (ex: postgres primary + replica), resolve/bind com nome opcional
- `#[injectable]` em `fn build(c: &Container) -> Self` (ou async)
- `#[inject]` — detecta parâmetro de container por tipo real (resolve caminho completo, não só token literal `&Container`), suporta alias de import
- `#[derive(Injectable)]` — resolve cada campo do struct do container
- Escopos: singleton (cache após 1ª resolução) e transient (nova instância a cada resolve)
- Testes isolados: `rudi::testing::with_container(|c| async { ... })` ou equivalente — container local, não o global

**Explicitly out of scope:**

- Lib não lê variáveis de ambiente — sempre responsabilidade do `init()` do consumidor
- Lib não decide fail-fast de config inválida — delega ao consumidor
- Suporte a outras linguagens/bindings (FFI) — só rust puro

## Constraints

- Timeline: sem prazo fixo definido pelo usuário
- Technical: resolve deve ser async mesmo com tipos/constructors síncronos, pra garantir grafo assíncrono uniforme
- Resources: projeto solo (leandroluk)
