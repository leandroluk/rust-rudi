# Core Container — Context (decisões do usuário)

## Escopo (definido antes do spec, na inicialização do projeto)

- **Transient**: entra na v1 (não só singleton) — `register_transient` além de `register_singleton`/`register_factory`/`register_instance`.
- **Named bindings**: entram na v1 — múltiplas instâncias do mesmo tipo sob nomes distintos (P2 do spec, CORE-08).
- **Testes isolados**: entram no roadmap (M3, `rudi::testing::with_container`) — fora do escopo deste spec (core-container), mas a API do container deve ser desenhada sem assumir singleton global implícito internamente (facilita M3 depois).
- **`#[inject]` / detecção de `&Container`**: resolve via tipo real (caminho completo), suporta alias de import (`use Container as C`). Fica pra M2 (macros) — não afeta este spec.

## Tipo público, target: open-source (crates.io)

- Edition 2021, MSRV 1.75+.

## Async

- `resolve` é **sempre async**, mesmo pra builders síncronos — grafo de dependências resolvido com await de ponta a ponta, garantindo suporte uniforme a builders que precisam de I/O (conexão de DB, etc).

## Decisões técnicas (spec → design)

- **Retorno de `resolve::<T>()`**: sempre `Arc<T>` — nunca `T` por valor. Uniformiza API entre tipo concreto e `dyn Port` (`resolve::<Arc<dyn Port>>()` também), evita exigir `T: Clone`, e é coerente com singleton compartilhado entre resolves.
  - Impacto: exemplos do METACODE.md que mostram `c.resolve::<LoggerSlogConfig>()` (sem Arc) precisam ser lidos como simplificação do arquivo de hipótese — API real será `c.resolve::<LoggerSlogConfig>().await` retornando `Arc<LoggerSlogConfig>`.
- **Erro de resolve**: enum único `RudiError` (via `thiserror`), não `Box<dyn Error>`. Variantes previstas: `NotFound { type_name, name: Option<String> }`, `BuildFailed { type_name, source }` (pra propagar erro do builder), outras conforme design.
