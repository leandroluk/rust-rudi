# Roadmap

**Current Milestone:** M1 — Core Container
**Status:** Planning

---

## M1 — Core Container

**Goal:** Container global funcional com registro (instância/factory/singleton/transient) e resolve async, sem macros ainda — API programática pura.
**Target:** `cargo test` verde cobrindo os 4 modos de registro + resolve async + named bindings.

### Features

**Container global (`rudi::container()`)** - PLANNED

- Singleton lazy 1-por-processo
- `register_instance`, `register_factory`, `register_singleton`, `register_transient`
- `resolve::<T>()` async, com cache pra singleton e nova instância pra transient

**Named bindings** - PLANNED

- Variante nomeada de register/resolve pra múltiplas instâncias do mesmo tipo (ex: postgres primary/replica)

**Bind de porta (trait objects)** - PLANNED

- `bind::<Impl, dyn Port>()` — registra Impl resolvível via `Arc<dyn Port>`

---

## M2 — Macros

**Goal:** Açúcar sintático via `rudi-macros` cobrindo os exemplos do METACODE.md ponta a ponta.
**Target:** Exemplo do METACODE (logger slog + database postgres/mongodb) compila e roda.

### Features

**`#[injectable]`** - PLANNED
**`#[inject]`** - PLANNED (resolve tipo real do parâmetro, não token literal)
**`#[derive(Injectable)]`** - PLANNED

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
