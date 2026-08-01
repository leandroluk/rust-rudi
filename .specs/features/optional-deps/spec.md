# Optional Dependencies (M7, brief)

## Problem Statement

Toda dependência hoje é obrigatória — não registrado = `Err`. Padrão comum (feature flags, plugins opcionais, adapter que só existe em alguns ambientes): "resolve se existir, `None` se não, nunca erro".

## Goals

- [ ] `resolve::<Option<T>>()` (ou equivalente) retorna `Ok(None)` se `T` não registrado, `Ok(Some(Arc<T>))` se registrado — nunca `Err(NotFound)` pra esse caso específico
- [ ] `#[inject]` em parâmetro `Option<Arc<T>>` (dentro de `#[injectable]`) resolve como opcional automaticamente

## Design

- Novo método `Container::resolve_optional::<T>() -> Result<Option<Arc<T>>, RudiError>` — chama `resolve_any` internamente, mas converte `RudiError::NotFound` especificamente em `Ok(None)` (outros erros, tipo `BuildFailed`, continuam propagando como `Err` — "não registrado" é diferente de "registrado mas falhou ao construir").
- `#[inject]` na macro: se o parâmetro é `Option<Arc<T>>` (peel de `Option<...>` antes do peel de `Arc<...>`), usa `resolve_optional` em vez de `resolve`.
- `#[derive(Injectable)]`: mesma lógica pros campos.

## Requirements (WHEN/THEN)

1. WHEN `resolve_optional::<T>()` é chamado sem `T` registrado THEN SHALL retornar `Ok(None)`
2. WHEN `resolve_optional::<T>()` é chamado com `T` registrado THEN SHALL retornar `Ok(Some(Arc<T>))`, igual `resolve` normal
3. WHEN builder de `T` registrado falha (`BuildFailed`) THEN `resolve_optional::<T>()` SHALL propagar o erro, NÃO virar `None` (falha ≠ ausência)
4. WHEN `#[inject]` decora parâmetro `Option<Arc<T>>` THEN SHALL resolver via `resolve_optional`, nunca erra por `T` ausente

## Success Criteria

- [ ] Teste: `resolve_optional` sem registro retorna `None`
- [ ] Teste: `resolve_optional` com registro retorna `Some`
- [ ] Teste: builder que falha propaga erro mesmo via `resolve_optional`
- [ ] Teste: `#[inject]` em `Option<Arc<T>>` dentro de `#[injectable]`, com e sem registro prévio
- [ ] `cargo test --workspace` verde, gate full
