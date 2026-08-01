# Debug Introspection (M9, brief)

## Problem Statement

Debugar "por que isso não resolveu" hoje é adivinhação — não tem como listar o que tá registrado nem ver quem depende de quem. Depende de M6 (reusa a pilha de resolução pra capturar arestas observadas).

## Goals

- [ ] `c.debug_entries() -> Vec<DebugEntry>` — lista tudo registrado (tipo, nome opcional, modo: instance/transient/singleton/bind_many)
- [ ] `c.debug_edges() -> Vec<(DebugEntry, DebugEntry)>` — arestas **observadas** (pai→filho) durante resoluções que já aconteceram — não é análise estática do grafo completo, é o que já foi visto em runtime

## Design

- `debug_entries`: itera `Inner.entries` + `Inner.many`, mapeia `Key` pra `DebugEntry { type_name, name, kind }`. `type_name` não existe hoje guardado no `Entry` (só no momento do erro) — precisa passar a guardar `&'static str` em cada `Entry`/`ManySlot` na hora do registro (pequeno acréscimo de campo).
- `debug_edges`: reusa a `task_local` pilha de resolução do M6 — toda vez que `resolve_any` entra com a pilha não-vazia (ou seja, tá resolvendo algo DENTRO de outro resolve), registra a aresta `(topo_da_pilha, tipo_atual)` numa lista compartilhada (`Inner.observed_edges: RwLock<Vec<(DebugEntry, DebugEntry)>>`), deduplicando.
- Ambos são só leitura/debug — não afetam resolução normal em nada além do pequeno overhead de guardar `type_name` (já era calculado, só passa a persistir).

## Requirements (WHEN/THEN)

1. WHEN 3 tipos registrados (instance/transient/singleton) THEN `debug_entries()` SHALL listar os 3, com modo correto
2. WHEN A resolve B (nested, dentro de outro resolve) THEN `debug_edges()` SHALL conter a aresta A→B após essa resolução acontecer
3. WHEN nenhuma resolução aninhada aconteceu ainda THEN `debug_edges()` SHALL retornar vazio (não é análise estática, só observado)

## Success Criteria

- [ ] Teste: `debug_entries` lista registros de cada modo corretamente
- [ ] Teste: `debug_edges` captura aresta depois de uma resolução aninhada de verdade
- [ ] `cargo test --workspace` verde, gate full

## Depends on

M6 (circular-detection) — reusa a infraestrutura de pilha de resolução (`tokio::task_local`).
