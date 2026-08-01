# Roadmap

**Current Milestone:** v1 Complete (+ M4-M9 pós-v1)
**Status:** M1-M9 Complete

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

**`rudi::testing::with_container`** - COMPLETE

---

## M4 — Multi-bind (pós-v1)

**Goal:** Resolver todas as implementações de uma porta de uma vez (padrão healthcheck), sem afetar a semântica "última vence" de `bind`/`bind_with`.
**Motivação:** exemplo `PingablePort` adicionado ao METACODE.md.

### Features

**`bind_many`/`resolve_all`** - COMPLETE

- Storage separado (`Inner::many`) — nunca colide com `bind`/`bind_with` na mesma chave
- `resolve_all` vazio não é erro (diferente de `resolve`)
- Cada slot é cacheado individualmente (double-init safe, mesma garantia do singleton)

---

## M6 — Circular Dependency Detection (pós-v1)

**Goal:** Ciclo de dependência vira erro claro, não deadlock silencioso.

### Features

**Detecção via `tokio::task_local`** - COMPLETE

- Pilha de resolução por cadeia lógica (async-safe sob runtime multi-thread — thread-local seria errado, task pode trocar de thread entre awaits)
- `RudiError::CircularDependency { chain }` com a cadeia legível

---

## M7 — Optional Dependencies (pós-v1)

**Goal:** Dependência que pode não existir (feature flag, plugin opcional) sem forçar erro.

### Features

**`resolve_optional` + `Option<Arc<T>>` em `#[inject]`/`#[derive(Injectable)]`** - COMPLETE

- `NotFound` vira `Ok(None)`; `BuildFailed` continua propagando (ausência ≠ falha)

---

## M8 — Shutdown Hooks (pós-v1)

**Goal:** Singleton com recurso externo (pool, socket) ganha um jeito de "desligar direito".

### Features

**`on_shutdown`/`shutdown`** - COMPLETE

- Registro manual, execução em ordem reversa (LIFO), sequencial

---

## M9 — Debug Introspection (pós-v1)

**Goal:** Debugar "por que isso não resolveu" sem adivinhação. Depende de M6.

### Features

**`debug_entries`/`debug_edges`** - COMPLETE

- `debug_entries`: tudo registrado agora (tipo/nome/modo)
- `debug_edges`: arestas pai→filho observadas em runtime (reusa a pilha do M6), não é análise estática do grafo completo

---

## Future Considerations

- Bindings FFI pra outras linguagens — fora de escopo v1
- Detecção de dependência circular em **compile-time** (hoje é runtime, M6) — exigiria análise estática do grafo de tipos, fora de escopo por ora
