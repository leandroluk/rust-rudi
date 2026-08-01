# Shutdown Hooks (M8, brief)

## Problem Statement

Singleton hoje não tem "desligar direito" — pool de conexão, socket, etc ficam pra sempre até o processo morrer. Precisa de um jeito de registrar limpeza, chamada em ordem reversa (LIFO, igual destructor) quando o consumidor decide encerrar.

## Goals

- [ ] `c.on_shutdown(hook)` — registra um hook async, executado por `Container::shutdown()`
- [ ] `c.shutdown().await` — roda todos os hooks registrados, em ordem **reversa** de registro (LIFO)
- [ ] Registro é manual/explícito (consumidor chama `on_shutdown` dentro do próprio `init()`/builder) — sem detecção automática via trait, sem acoplar ao sistema de tipos do `Injectable`

## Design

- `Inner.shutdown_hooks: RwLock<Vec<BoxedShutdownHook>>`, `BoxedShutdownHook = Arc<dyn Fn(Container) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>`
- `on_shutdown<F, Fut>(&self, hook: F)` — `F: Fn(Container) -> Fut + Send + Sync + 'static`, `Fut: Future<Output = ()> + Send + 'static`; `push` na lista (ordem de registro preservada)
- `shutdown(&self)` — drena a lista, itera **reverso**, `await` cada hook sequencialmente (não concorrente — ordem importa, é destructor-like)
- Consumidor registra dentro do próprio `init()`, logo depois de montar o recurso — ordem de registro casa naturalmente com ordem de criação

## Requirements (WHEN/THEN)

1. WHEN N hooks são registrados via `on_shutdown` (ordem: h1, h2, h3) THEN `shutdown()` SHALL executá-los na ordem h3, h2, h1
2. WHEN `shutdown()` é chamado sem nenhum hook registrado THEN SHALL retornar sem erro, sem panic
3. WHEN um hook faz algo assíncrono (ex: `tokio::time::sleep`) THEN `shutdown()` SHALL esperar ele terminar antes de rodar o próximo

## Success Criteria

- [ ] Teste: 3 hooks registrados, confirma ordem reversa de execução (via `Vec` compartilhado que cada hook empurra seu próprio ID)
- [ ] Teste: `shutdown()` sem hooks não panica
- [ ] `cargo test --workspace` verde, gate full
