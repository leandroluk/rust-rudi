# Multi-bind (M4) Specification (brief, tier Medium)

## Problem Statement

`bind`/`bind_with` são "última vence" — só 1 impl concreta resolvível por porta. Padrão comum (healthcheck: "pega todo mundo que sabe pingar") precisa do oposto: acumular várias impls da mesma porta e resolver todas de uma vez. Ver exemplo `PingablePort` adicionado ao METACODE.md.

## Goals

- [ ] `c.bind_many::<Impl, dyn Port>()` — acumula (não sobrescreve) implementações de uma porta
- [ ] `c.resolve_all::<Arc<dyn Port>>()` — retorna todas as implementações acumuladas via `bind_many`
- [ ] `bind`/`bind_with` (M1/M2) continuam com semântica intocada — storage completamente separado
- [ ] Mesma `Impl` pode estar registrada em `bind` (porta A) e `bind_many` (porta B) sem conflito

## Out of Scope

| Feature | Reason |
| --- | --- |
| `bind_many_named`/`resolve_all_named` | Não pedido no exemplo (METACODE.md), sem caso de uso concreto ainda — evitar API não usada |
| `bind_many_with` (builder manual, sem macro) | Fora do exemplo motivador; adicionar depois se surgir necessidade real |

## Requirements (WHEN/THEN)

1. WHEN `bind_many::<Impl, Port>()` é chamado N vezes pra mesma `Port` (mesma ou diferentes `Impl`s) THEN `resolve_all::<Arc<Port>>()` SHALL retornar N itens, na ordem de registro
2. WHEN `resolve_all::<Arc<Port>>()` é chamado sem nenhum `bind_many` prévio pra essa porta THEN SHALL retornar `Ok(vec![])` — vazio não é erro (diferente de `resolve` normal)
3. WHEN a mesma `Impl` é registrada via `bind` (porta A) e `bind_many` (porta B) THEN ambas resoluções SHALL funcionar independentemente, sem um afetar o outro
4. WHEN um item de `bind_many` é resolvido 2x via `resolve_all` THEN cada item individual SHALL ser cacheado (singleton) — mesma garantia de double-init safety do `register_singleton`/`bind` existente

## Success Criteria

- [ ] Teste com 2 `bind_many` pra mesma porta (impls diferentes), `resolve_all` retorna as 2, em ordem
- [ ] Teste `resolve_all` sem bind prévio retorna vazio, não erro
- [ ] Teste `bind` + `bind_many` da mesma `Impl` em portas diferentes, ambos resolvem certo
- [ ] `cargo test -p rudi` verde
