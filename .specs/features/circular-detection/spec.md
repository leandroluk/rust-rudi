# Circular Dependency Detection (M6, brief)

## Problem Statement

Hoje, dependência circular (A resolve B que resolve A) trava pra sempre — o `OnceCell` do singleton de A espera a própria inicialização de A terminar, que nunca acontece. Sem mensagem de erro, sem timeout, só deadlock silencioso. Vira erro claro em vez de travar.

## Goals

- [ ] Ciclo detectado retorna `Err(RudiError::CircularDependency { chain })` em vez de travar
- [ ] `chain` legível: lista de type names na ordem da cadeia (ex: `["A", "B", "A"]`)
- [ ] Funciona corretamente sob runtime multi-thread (não pode usar thread-local puro — task pode trocar de thread entre `.await`s)

## Design

- Rastreio via `tokio::task_local!` (`RefCell<Vec<(TypeId, Option<String>, &'static str)>>`) — escopo por cadeia de resolução lógica, não por container nem por thread. `LocalKey::scope`/`.with()` funcionam sem runtime tokio ativo de verdade (só custo de feature flag `rt`, confirmado na doc oficial).
- `resolve_any` estabelece o escopo (`task_local.scope(...)`) só na entrada de fora (1ª chamada da cadeia); chamadas recursivas (builder chamando `c.resolve()` de novo) reusam o escopo já ativo via `try_with`.
- Cada resolução checa se `(TypeId, name)` já está na pilha ANTES de empilhar; se sim, retorna erro com a cadeia completa. Push/pop via RAII guard (`Drop`), sobrevive a `?`/panic.
- `Cargo.toml` do core ganha feature `rt` do tokio (só compile-time, sem custo de runtime).

## Requirements (WHEN/THEN)

1. WHEN builder de A resolve B, e builder de B resolve A (mesmo container, mesma cadeia) THEN `resolve` SHALL retornar `Err(RudiError::CircularDependency)` em vez de travar
2. WHEN não há ciclo (grafo normal, mesmo profundo) THEN resolução SHALL funcionar exatamente como antes, sem overhead perceptível
3. WHEN 2 resoluções concorrentes (não relacionadas) rodam ao mesmo tempo sob runtime multi-thread THEN pilha de 1 SHALL nunca vazar/interferir na da outra

## Success Criteria

- [ ] Teste: A→B→A (2 níveis) detecta ciclo, mensagem cita os 2 tipos
- [ ] Teste: A→B→C (sem ciclo) resolve normalmente
- [ ] Teste: 2 resoluções concorrentes não relacionadas (`tokio::join!`) não se interferem
- [ ] `cargo test --workspace` verde, gate full
