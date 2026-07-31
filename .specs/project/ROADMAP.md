# Roadmap

**Current Milestone:** M3 — Testes isolados
**Status:** M1 Complete, M2 Complete, M3 Planning

---

## M1 — Core Container

**Goal:** Container global funcional com registro (instância/factory/singleton/transient) e resolve async, sem macros ainda — API programática pura.
**Target:** `cargo test` verde cobrindo os 4 modos de registro + resolve async + named bindings.

### Features

**Container global (`rudi::container()`)** - COMPLETE

- Singleton lazy 1-por-processo (`OnceLock`)
- `register_instance`, `register_transient`, `register_singleton` (+ variantes `_named`)
- `resolve::<T>()` async, com cache pra singleton e nova instância pra transient

**Named bindings** - COMPLETE

- Variante nomeada de register/resolve pra múltiplas instâncias do mesmo tipo (ex: postgres primary/replica)

**Bind de porta (trait objects)** - COMPLETE

- `bind_with::<Port>(builder)` — registra impl resolvível via `Arc<Port>`, builder explícito (sem macro)

---

## M2 — Macros

**Goal:** Açúcar sintático via `rudi-macros` cobrindo os exemplos do METACODE.md ponta a ponta.
**Target:** Exemplo do METACODE (logger slog + database postgres/mongodb) compila e roda.

### Features

**`Injectable` trait + `Container::bind`/`register_singleton_injectable`** - COMPLETE
**`#[injectable]`** - COMPLETE (decora `impl` block, não a fn — restrição de proc-macro, ver design.md)
**`#[inject]`** - COMPLETE (marker attribute `#[container]`, robusto contra alias)
**`#[derive(Injectable)]`** - COMPLETE

---

## M3 — Testes isolados

**Goal:** Consumidores conseguem testar sem vazamento de estado entre testes via container global.

### Features

**`rudi::testing::with_container`** - PLANNED

---

## Future Considerations

- Bindings FFI pra outras linguagens — fora de escopo v1
- Observability/tracing de resolução (debug de grafo de deps)
- Detecção de dependência circular em compile-time (hoje seria runtime panic)
