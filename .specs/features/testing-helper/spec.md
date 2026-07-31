# Testing Helper Specification (M3 — brief, tier Medium)

## Problem Statement

`rudi::container()` é global (1 por processo) — testes que registram/resolvem tipos nele vazam estado entre si quando rodam em paralelo (padrão do `cargo test`). `Container::new()` já resolve isso (container local), mas falta um helper ergonômico específico pra testes, batendo com o "em aberto" do METACODE.md.

## Goals

- [ ] `rudi::testing::with_container(f)` cria um `Container::new()` isolado, passa pro closure, retorna o resultado — sem tocar no container global

## Out of Scope

| Feature | Reason |
| --- | --- |
| Snapshot/fork do container global pra testes | Não pedido, complexidade desnecessária — isolamento total (container zerado) é o padrão esperado |

## Requirements (WHEN/THEN)

1. WHEN `with_container(f)` é chamado com `f: FnOnce(Container) -> Fut` THEN SHALL criar `Container::new()`, chamar `f(c)`, retornar `Fut::Output`
2. WHEN 2 chamadas a `with_container` acontecem (mesmo em paralelo) THEN cada uma SHALL ter seu próprio container, sem interferência
3. WHEN o closure registra/resolve algo THEN o container global (`rudi::container()`) SHALL permanecer intocado

## Success Criteria

- [ ] Teste com 2 chamadas paralelas a `with_container` registrando o mesmo tipo sem nome, confirma isolamento (cada uma vê só o que ela mesma registrou)
- [ ] `cargo test -p rudi` verde
