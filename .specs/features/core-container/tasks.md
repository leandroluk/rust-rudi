# Core Container Tasks

**Design**: `.specs/features/core-container/design.md`
**Status**: Draft

---

## Execution Plan

### Phase 0: Workspace scaffold (Sequential)

```
T0
```

### Phase 1: Foundation (Sequential)

```
T0 → T1 → T2
```

### Phase 2: Registration APIs (Parallel OK)

```
       ┌→ T3 ─┐
T2 ────┼→ T4 ─┼──→ T6
       └→ T5 ─┘
```

### Phase 3: Bind + Global container (Sequential, depende de tudo da Phase 2)

```
T6 → T7 → T8
```

### Phase 4: Integration test end-to-end (Sequential)

```
T8 → T9
```

---

## Task Breakdown

### T0: Scaffold do workspace cargo

**What**: Criar workspace cargo raiz com crate `rudi` (core) vazio, `Cargo.toml` do workspace, dependências base (`tokio` feature `sync`, `thiserror`).
**Where**: `Cargo.toml` (workspace root), `crates/rudi/Cargo.toml`, `crates/rudi/src/lib.rs` (vazio, só `// TODO`)
**Depends on**: None
**Reuses**: —
**Requirement**: — (infra)

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `cargo build --workspace` passa
- [ ] `crates/rudi/Cargo.toml` declara `tokio = { version = "1", default-features = false, features = ["sync"] }` e `thiserror = "1"` (ou versões atuais — confirmar no Cargo.toml gerado, sem pin arbitrário)
- [ ] Gate check passa: `cargo build --workspace`

**Tests**: none
**Gate**: build

**Commit**: `chore(rudi): scaffold cargo workspace`

---

### T1: `RudiError` (enum de erro)

**What**: Implementar enum `RudiError` com variantes `NotFound`, `BuildFailed`, `DowncastFailed` conforme design.
**Where**: `crates/rudi/src/error.rs`
**Depends on**: T0
**Reuses**: —
**Requirement**: CORE-02, CORE-05

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] 3 variantes implementadas com `#[derive(thiserror::Error, Debug)]` e mensagens conforme design.md
- [ ] `RudiError` é `Send + Sync + 'static`
- [ ] Unit test: cada variante formata mensagem esperada (`.to_string()`)
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(rudi): add RudiError enum`

---

### T2: `Key` + `Entry` + `Inner` (estado interno do container)

**What**: Implementar tipos internos privados `Key` (TypeId + nome opcional, Hash+Eq), `Entry` (Instance/Transient/Singleton com `BoxedFactory`/`BoxedFuture`), e `Inner` (`RwLock<HashMap<Key, Entry>>`).
**Where**: `crates/rudi/src/container.rs` (seções privadas)
**Depends on**: T1
**Reuses**: `RudiError` (T1)
**Requirement**: CORE-09 (base pra double-init safety via `OnceCell` dentro de `Entry::Singleton`)

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `Key` implementa `Hash + Eq + Clone`, diferencia registro nomeado vs sem nome
- [ ] `Entry` tem as 3 variantes conforme design.md, `Singleton` usa `tokio::sync::OnceCell<Arc<dyn Any + Send + Sync>>`
- [ ] Unit test: 2 `Key`s com mesmo `TypeId` e nomes diferentes são `!=`; mesmo `TypeId` e mesmo nome (ou ambos `None`) são `==`
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(rudi): add internal Key/Entry/Inner types`

---

### T3: `register_instance` / `register_instance_named` + `resolve` / `resolve_named` [P]

**What**: Implementar o par instância (registro imediato de valor pronto) e a resolução genérica (`resolve_any` privado + `resolve`/`resolve_named` públicos fazendo downcast).
**Where**: `crates/rudi/src/container.rs`
**Depends on**: T2
**Reuses**: `Key`, `Entry::Instance`, `RudiError`
**Requirement**: CORE-01, CORE-02, CORE-08, CORE-10

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `register_instance<T>(&self, value: T)` e `register_instance_named<T>(&self, name, value: T)` implementados
- [ ] `resolve_any(&self, type_id, name) -> Result<Arc<dyn Any+Send+Sync>, RudiError>` privado implementado
- [ ] `resolve::<T>(&self).await -> Result<Arc<T>, RudiError>` e `resolve_named::<T>(&self, name).await` públicos, fazendo downcast e retornando `DowncastFailed` só em caso teoricamente impossível (documentar via `debug_assert` se cabível)
- [ ] Unit/integration test cobrindo CORE-01 (registrar e resolver), CORE-02 (resolve sem registro retorna `NotFound`), CORE-10 (`Container::new()` sem nenhum register, resolve retorna erro e não panic)
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit + integration (`tests/register_instance.rs`)
**Gate**: quick

**Commit**: `feat(rudi): add register_instance and resolve APIs`

---

### T4: `register_transient` / `register_transient_named` [P]

**What**: Implementar registro de factory que reexecuta a cada `resolve` (sem cache).
**Where**: `crates/rudi/src/container.rs`
**Depends on**: T2
**Reuses**: `Key`, `Entry::Transient`, `resolve_any` (será integrado em T3 — se T3 e T4 rodam em paralelo, cada um implementa sua branch do `match` em `resolve_any`; merge sequencial na integração faz o `match` completo — ver nota abaixo)
**Requirement**: CORE-03, CORE-05

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `register_transient<T, F, Fut, E>(&self, builder: F)` onde `F: Fn(Container) -> Fut + Send + Sync + 'static`, `Fut: Future<Output = Result<T, E>> + Send`, `E: std::error::Error + Send + Sync + 'static`
- [ ] Builder erro vira `RudiError::BuildFailed`
- [ ] Unit test: builder com contador incrementado a cada chamada — 2 resolves seguidos retornam `Arc` com valores `1` e `2` (nova execução cada vez)
- [ ] Named variant testada (2 nomes diferentes, builders independentes)
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit + integration (`tests/register_transient.rs`)
**Gate**: quick

**Nota de integração**: como T3/T4/T5 mexem na mesma `enum Entry`/`match` de `resolve_any`, a implementação real será feita sequencialmente por 1 agente (mesmo arquivo, edits concorrentes de 3 sub-agentes causariam conflito) — manter `[P]` só como sinalização de independência lógica; orquestrador roda T3→T4→T5 em sequência no mesmo arquivo, mas cada um é revisável isoladamente.

**Commit**: `feat(rudi): add register_transient API`

---

### T5: `register_singleton` / `register_singleton_named` (com `OnceCell`) [P]

**What**: Implementar registro de factory cacheada — 1ª resolução executa e guarda, demais retornam cache; concorrência não duplica execução.
**Where**: `crates/rudi/src/container.rs`
**Depends on**: T2
**Reuses**: `Key`, `Entry::Singleton`, `tokio::sync::OnceCell::get_or_try_init`
**Requirement**: CORE-04, CORE-05, CORE-09

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `register_singleton<T, F, Fut, E>(&self, builder: F)` (mesma assinatura de `register_transient`)
- [ ] 1ª resolução executa builder; resoluções seguintes retornam mesmo `Arc` (ponteiro igual — `Arc::ptr_eq`)
- [ ] Unit test de concorrência: `tokio::join!` disparando 10 resolves simultâneos do mesmo singleton com builder que incrementa contador — contador termina em 1 (CORE-09)
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit + integration (`tests/register_singleton.rs`)
**Gate**: quick

**Commit**: `feat(rudi): add register_singleton API with double-init safety`

---

### T6: Merge/integração das 3 branches de registro em `resolve_any`

**What**: Consolidar T3+T4+T5 num único `match` coerente em `resolve_any`, garantir que os 3 modos de registro coexistem sem regressão (rodar suite completa).
**Where**: `crates/rudi/src/container.rs`
**Depends on**: T3, T4, T5
**Reuses**: Todo o código das 3 tasks anteriores
**Requirement**: CORE-01 a CORE-05, CORE-08, CORE-09, CORE-10

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `resolve_any` trata as 3 variantes de `Entry` num único `match`, sem código morto/duplicado
- [ ] Suite completa de T3+T4+T5 passa sem regressão
- [ ] Gate check passa: `cargo test -p rudi && cargo clippy -p rudi -- -D warnings`

**Tests**: integration (suite acumulada)
**Gate**: full

**Commit**: `refactor(rudi): consolidate resolve_any across registration modes`

---

### T7: `bind_with::<Impl, Port>()` (bind de porta / trait object)

**What**: Implementar `bind_with` — registra `Impl` como singleton sob a chave de `Arc<Port>` (via `TypeId::of::<dyn Port + 'static>()`), permitindo `resolve::<Arc<dyn Port>>()`.
**Where**: `crates/rudi/src/container.rs`
**Depends on**: T6
**Reuses**: `register_singleton` internals (mesma `Entry::Singleton`, chave diferente)
**Requirement**: CORE-06, CORE-07

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `bind_with<Impl, Port: ?Sized + 'static, F, Fut, E>(&self, builder: F)` implementado, `Impl: Port` (ou builder já retorna `Arc<Port>` diretamente — decidir na implementação conforme trait object coercion; documentar no código)
- [ ] `resolve::<Arc<dyn Port>>()` funciona após `bind_with`
- [ ] Unit test CORE-07: 2 `bind_with` sequenciais pra mesma `Port`, resolve retorna resultado da 2ª (última vence)
- [ ] Unit test: `resolve::<Arc<dyn Port>>()` sem nenhum bind retorna `NotFound`
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit + integration (`tests/bind.rs`)
**Gate**: quick

**Commit**: `feat(rudi): add bind_with for trait object resolution`

---

### T8: `container()` global (`OnceLock`)

**What**: Expor `rudi::container() -> Container` no crate root, singleton lazy via `std::sync::OnceLock`, chamando `Container::new()` internamente na 1ª chamada.
**Where**: `crates/rudi/src/lib.rs`
**Depends on**: T7
**Reuses**: `Container::new()` (já existe desde T2/T3)
**Requirement**: — (infra de conveniência, cumpre regra "Container é ambiente/global" do METACODE)

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] `rudi::container()` retorna sempre o mesmo handle (mesmo `Arc` interno) entre chamadas
- [ ] `pub use` de `Container`, `RudiError` no lib.rs (superfície pública mínima)
- [ ] Unit test: 2 chamadas a `container()` resolvem o mesmo valor registrado por uma delas (prova que é o mesmo estado)
- [ ] Gate check passa: `cargo test -p rudi`

**Tests**: unit
**Gate**: quick

**Commit**: `feat(rudi): expose global container() singleton`

---

### T9: Teste de integração end-to-end (exemplo do METACODE sem macros)

**What**: Reproduzir o cenário do METACODE.md (logger + database com 2 providers, 2 containers com "env" diferente) usando só a API pública construída (T0-T8), sem macros — builders chamados manualmente como as macros chamariam depois.
**Where**: `crates/rudi/tests/metacode_scenario.rs`
**Depends on**: T8
**Reuses**: Toda a API pública do crate
**Requirement**: Success Criteria do spec.md (exemplo METACODE compila e roda)

**Tools**:
- MCP: NONE
- Skill: NONE

**Done when**:
- [ ] Teste cria 2 `Container::new()` locais (não o global, pra isolamento — ver TESTING.md notes) simulando "postgres" e "mongodb" via `bind_with` diferente em cada
- [ ] `resolve::<Arc<dyn DatabasePort>>()` e `resolve::<Arc<dyn LoggerPort>>()` funcionam nos 2 containers, cada um resolvendo a impl certa
- [ ] Gate check passa: `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --all -- --check`
- [ ] Test count final documentado no commit message

**Tests**: integration
**Gate**: full

**Commit**: `test(rudi): add end-to-end scenario matching METACODE example`

---

## Parallel Execution Map

```
Phase 0 (Sequential):
  T0

Phase 1 (Sequential):
  T0 ──→ T1 ──→ T2

Phase 2 (mesmo arquivo — logicamente paralelo, executado sequencial por 1 agente):
  T2 complete, then:
    T3 → T4 → T5   (nota: [P] lógico, mas mesmo arquivo container.rs — ver Nota de integração em T4)

Phase 3 (Sequential):
  T3,T4,T5 complete, then:
    T6 ──→ T7 ──→ T8

Phase 4 (Sequential):
  T8 complete, then:
    T9
```

---

## Task Granularity Check

| Task | Scope | Status |
| --- | --- | --- |
| T0: Scaffold workspace | 1 setup (multi-arquivo, mas 1 concern: bootstrap) | ✅ Granular |
| T1: RudiError | 1 arquivo, 1 enum | ✅ Granular |
| T2: Key/Entry/Inner | 3 tipos cohesivos, 1 concern (estado interno) | ✅ Granular |
| T3: register_instance + resolve | 2 funções + fn privada, 1 concern (instância) | ✅ Granular |
| T4: register_transient | 2 funções, 1 concern | ✅ Granular |
| T5: register_singleton | 2 funções, 1 concern | ✅ Granular |
| T6: merge resolve_any | 1 função (consolidação) | ✅ Granular |
| T7: bind_with | 1 função, 1 concern | ✅ Granular |
| T8: container() global | 1 função | ✅ Granular |
| T9: e2e test | 1 arquivo de teste | ✅ Granular |

---

## Diagram-Definition Cross-Check

| Task | Depends On (body) | Diagram Shows | Status |
| --- | --- | --- | --- |
| T0 | None | — (raiz) | ✅ Match |
| T1 | T0 | T0→T1 | ✅ Match |
| T2 | T1 | T1→T2 | ✅ Match |
| T3 | T2 | T2→T3 | ✅ Match |
| T4 | T2 | T2→T4 (via T3→T4 sequencial na prática, dependência lógica é T2) | ✅ Match |
| T5 | T2 | T2→T5 (via T4→T5 sequencial na prática, dependência lógica é T2) | ✅ Match |
| T6 | T3, T4, T5 | T3,T4,T5→T6 | ✅ Match |
| T7 | T6 | T6→T7 | ✅ Match |
| T8 | T7 | T7→T8 | ✅ Match |
| T9 | T8 | T8→T9 | ✅ Match |

---

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| --- | --- | --- | --- | --- |
| T0 | scaffold (sem lógica) | none | none | ✅ OK |
| T1 | módulo interno (error.rs) | unit | unit | ✅ OK |
| T2 | módulo interno (container.rs privado) | unit | unit | ✅ OK |
| T3 | API pública | unit + integration | unit + integration | ✅ OK |
| T4 | API pública | unit + integration | unit + integration | ✅ OK |
| T5 | API pública | unit + integration | unit + integration | ✅ OK |
| T6 | API pública (consolidação) | integration | integration | ✅ OK |
| T7 | API pública | unit + integration | unit + integration | ✅ OK |
| T8 | API pública | unit | unit | ✅ OK |
| T9 | cenário completo | integration | integration | ✅ OK |

---

## Tools per Task — confirmação pendente

Nenhuma task requer MCP externo (context7 só seria necessário se surgisse dúvida de API de `tokio`/`thiserror` durante implementação — usar sob demanda). Nenhuma skill externa aplicável (mermaid-studio/codenavi não instalados, verificado no início do projeto — repo tinha só METACODE.md).
