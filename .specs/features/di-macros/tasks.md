# DI Macros Tasks

**Design**: `.specs/features/di-macros/design.md`
**Status**: Done — T10-T16 implementados, 29 testes (unit+integration+compile-fail via trybuild) + example `metacode` rodando de ponta a ponta, gate full verde

---

## Execution Plan

### Phase 1: Core trait + methods (Sequential)

```
T10 → T11
```

### Phase 2: Macro crate scaffold (Sequential, depende do core)

```
T11 → T12
```

### Phase 3: As 3 macros (Sequential — mesmo crate/lib.rs compartilhando parse helpers)

```
T12 → T13 → T14 → T15
```

### Phase 4: Integração end-to-end (Sequential)

```
T15 → T16
```

---

## Task Breakdown

### T10: `Injectable` trait no core

**What**: Definir `pub trait Injectable` em `crates/rudi/src/injectable.rs` conforme design.md (RPITIT `build`, associated `Error`/`Port`, `into_port`).
**Where**: `crates/rudi/src/injectable.rs`, `crates/rudi/src/lib.rs` (mod + pub use)
**Depends on**: None (core já tem `Container`/`RudiError` do M1)
**Reuses**: `Container`, `RudiError`
**Requirement**: base de MACRO-01 a MACRO-07

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] Trait compila com a assinatura exata do design.md
- [ ] Unit test: impl manual (sem macro) de `Injectable` pra um struct de teste, confirma que compila e é chamável
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(rudi): add Injectable trait`

---

### T11: `Container::bind` e `register_singleton_injectable`

**What**: Implementar os 2 métodos novos em `Container`, delegando pra `register_singleton` existente.
**Where**: `crates/rudi/src/container.rs`
**Depends on**: T10
**Reuses**: `register_singleton<T,F,Fut,E>` (M1)
**Requirement**: MACRO-06, MACRO-07

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] `bind<Impl: Injectable<Port=Port>, Port: ?Sized+Send+Sync+'static>(&self)` implementado
- [ ] `register_singleton_injectable<T: Injectable<Port=T>>(&self)` implementado
- [ ] Integration test (`tests/injectable_manual.rs`, `impl Injectable` escrito à mão — sem macro ainda): bind de porta e resolve funcionam; register_singleton_injectable e resolve funcionam; 2 binds seguidos → última vence
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: integration
**Gate**: quick

**Commit**: `feat(rudi): add Container::bind and register_singleton_injectable`

---

### T12: Scaffold `rudi-macros` crate

**What**: Novo crate `crates/rudi-macros` (`proc-macro = true`), workspace member, deps `syn`/`quote`/`proc-macro2`, dev-dep `trybuild` (compile-fail tests), `lib.rs` vazio com os 3 pontos de entrada declarados (`#[proc_macro_attribute]`/`#[proc_macro_derive]`) retornando input sem alteração (placeholder).
**Where**: `Cargo.toml` (workspace members), `crates/rudi-macros/Cargo.toml`, `crates/rudi-macros/src/lib.rs`
**Depends on**: T11
**Reuses**: —
**Requirement**: infra

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] `cargo build --workspace` passa com o novo crate
- [ ] `crates/rudi/Cargo.toml` ganha dependência opcional/direta em `rudi-macros` (reexportando as macros — `pub use rudi_macros::{injectable, inject, Injectable as InjectableDerive}` ou padrão equivalente de "macro crate" + "facade crate")
- [ ] Gate check passa: `cargo build --workspace`

**Tests**: none
**Gate**: build

**Commit**: `chore(rudi-macros): scaffold proc-macro crate`

---

### T13: `#[injectable]` macro

**What**: Implementar o algoritmo do design.md (parse `ImplItemFn`, detectar sync/async e `Self`/`Result`, gerar `impl Injectable`).
**Where**: `crates/rudi-macros/src/injectable.rs`
**Depends on**: T12
**Reuses**: `Injectable` (T10)
**Requirement**: MACRO-01 a MACRO-05

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] Suporta os 4 combos: sync+`Self`, sync+`Result`, async+`Self` (envolve await sem sentido mas válido), async+`Result` (caso real do design)
- [ ] `#[injectable]` sem argumento → `Port = Self`; `#[injectable(dyn X)]` → `Port = dyn X`
- [ ] Compile-fail test (`trybuild`) pra: fn não chamada `build`, fn fora de `impl`, fn com parâmetro Container ausente
- [ ] Integration test: struct real com `#[injectable]`, `register_singleton_injectable`/`bind`, resolve
- [ ] Gate check passa: `cargo test --workspace`

**Tests**: integration (+ compile-fail via trybuild)
**Gate**: full

**Commit**: `feat(rudi-macros): implement #[injectable]`

---

### T14: `#[inject]` macro

**What**: Implementar o algoritmo do design.md (remove parâmetro `#[container]`, injeta `rudi::container()` no corpo).
**Where**: `crates/rudi-macros/src/inject.rs`
**Depends on**: T13
**Reuses**: —
**Requirement**: MACRO-08 a MACRO-10

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] Fn com 1 parâmetro `#[container]` (tipo `&Container` ou alias `&C`) compila sem esse parâmetro na assinatura pública, chamável sem argumento
- [ ] Preserva `async`, generics e demais parâmetros
- [ ] Compile-fail test: 0 `#[container]`, 2+ `#[container]`
- [ ] Integration test: fn `#[inject] fn f(#[container] c: &Container) { c.register_instance(...) }`, chamada sem argumento, `rudi::container()` reflete o registro
- [ ] Gate check passa: `cargo test --workspace`

**Tests**: integration (+ compile-fail via trybuild)
**Gate**: full

**Commit**: `feat(rudi-macros): implement #[inject]`

---

### T15: `#[derive(Injectable)]` macro

**What**: Implementar o algoritmo do design.md (resolve campo a campo, `type Error = RudiError`, `type Port = Self`).
**Where**: `crates/rudi-macros/src/derive_injectable.rs`
**Depends on**: T14
**Reuses**: `resolve::<T>()` (M1)
**Requirement**: MACRO-11, MACRO-12

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] Struct com campos nomeados: cada campo resolvido via `c.resolve::<Tipo>().await?`
- [ ] Struct tuple: mesma lógica, ordem posicional
- [ ] Struct vazio (0 campos): `build` retorna `Self` sem nenhum resolve
- [ ] Campo não registrado → erro propaga como `RudiError::NotFound` (teste confirma via `Err` do resolve)
- [ ] Compile-fail test: derive em enum/union
- [ ] Gate check passa: `cargo test --workspace`

**Tests**: integration (+ compile-fail via trybuild)
**Gate**: full

**Commit**: `feat(rudi-macros): implement #[derive(Injectable)]`

---

### T16: Reprodução literal do METACODE.md com macros

**What**: Recriar a árvore `src/domain` + `src/infra` (logger/slog, database/postgres, database/mongodb) do METACODE.md literalmente, usando `#[injectable]`/`#[inject]` reais, num crate de exemplo/teste dentro do workspace.
**Where**: `crates/rudi/examples/metacode/` (ou `crates/rudi-macros/tests/metacode_full.rs` com módulos inline, decidir na implementação conforme ergonomia de `cargo test`)
**Depends on**: T15
**Reuses**: Toda a API pública de `rudi` + `rudi-macros`
**Requirement**: Success Criteria do spec.md (arquivos do METACODE compilam literalmente)

**Tools**: MCP: NONE / Skill: NONE

**Done when**:
- [ ] Código de `src/infra/logger/slog/adapter.rs` e `mod.rs` do METACODE.md reproduzido literalmente (ajustando só imports de crate), compila e resolve `LoggerPort`
- [ ] Código de `src/infra/database/{postgres,mongodb}` reproduzido literalmente, 2 containers com provider diferente resolvem `DatabasePort` corretamente
- [ ] Gate check passa: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`

**Tests**: integration
**Gate**: full

**Commit**: `test(rudi): reproduce METACODE example literally with macros`

---

## Parallel Execution Map

```
T10 ──→ T11 ──→ T12 ──→ T13 ──→ T14 ──→ T15 ──→ T16
```

Tudo sequencial — cada macro reusa parse helpers do mesmo crate (`rudi-macros`) e a suite de testes de integração cresce cumulativamente; paralelizar arriscaria conflito de edição no mesmo `lib.rs`/`Cargo.toml`.

---

## Task Granularity Check

| Task | Scope | Status |
| --- | --- | --- |
| T10: Injectable trait | 1 arquivo, 1 trait | ✅ Granular |
| T11: bind + register_singleton_injectable | 2 métodos cohesivos, 1 concern | ✅ Granular |
| T12: scaffold rudi-macros | 1 setup | ✅ Granular |
| T13: #[injectable] | 1 macro | ✅ Granular |
| T14: #[inject] | 1 macro | ✅ Granular |
| T15: #[derive(Injectable)] | 1 macro | ✅ Granular |
| T16: reprodução METACODE | 1 cenário de teste | ✅ Granular |

---

## Diagram-Definition Cross-Check

| Task | Depends On (body) | Diagram Shows | Status |
| --- | --- | --- | --- |
| T10 | None | raiz | ✅ Match |
| T11 | T10 | T10→T11 | ✅ Match |
| T12 | T11 | T11→T12 | ✅ Match |
| T13 | T12 | T12→T13 | ✅ Match |
| T14 | T13 | T13→T14 | ✅ Match |
| T15 | T14 | T14→T15 | ✅ Match |
| T16 | T15 | T15→T16 | ✅ Match |

---

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| --- | --- | --- | --- | --- |
| T10 | módulo interno (core) | unit | unit | ✅ OK |
| T11 | API pública (core) | integration | integration | ✅ OK |
| T12 | scaffold (sem lógica) | none | none | ✅ OK |
| T13 | macro | integration + compile-fail | integration + compile-fail | ✅ OK |
| T14 | macro | integration + compile-fail | integration + compile-fail | ✅ OK |
| T15 | macro | integration + compile-fail | integration + compile-fail | ✅ OK |
| T16 | cenário completo | integration | integration | ✅ OK |

---

## Tools per Task

Nenhuma task requer MCP externo. `context7` fica disponível sob demanda se surgir dúvida pontual de API `syn`/`quote`/`trybuild` durante a implementação (nenhuma dúvida bloqueante identificada até aqui).
