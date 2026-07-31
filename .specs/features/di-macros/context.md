# DI Macros — Context (decisões do usuário)

## `#[inject]` — detecção do parâmetro Container

- Proc-macro só enxerga sintaxe (`syn`), nunca tipo resolvido — não dá pra confiar em "path termina em `Container`" porque quebra com qualquer alias (`use rudi::Container as C`).
- Decisão: marker attribute explícito no parâmetro — `#[inject] fn f(#[container] c: AliasQualquer) { ... }`. Macro procura o parâmetro marcado com `#[container]`, remove-o da assinatura pública gerada, resolve `rudi::container()` internamente e injeta como 1ª linha do corpo. Funciona com qualquer alias/rename porque não depende do nome do tipo.
- Trade-off aceito: 1 atributo extra por parâmetro no código-fonte (invisível na assinatura pública pós-expansão) em troca de robustez total contra alias.

## `#[injectable]` — sync vs async, infalível vs `Result`

- `fn build(c: &Container) -> Self` (síncrona, como no METACODE.md) E `async fn build(c: &Container) -> Result<Self, E>` são ambas aceitas.
- Macro inspeciona a assinatura via `syn`:
  - `async fn` vs `fn` → decide se envolve a chamada em `.await` direto ou chama síncrono dentro do closure async gerado.
  - Tipo de retorno `Self` vs `Result<Self, E>` → decide se envolve o resultado em `Ok(...)` (infalível, erro nunca ocorre) ou repassa o `Result` como está.
- Motivação: bater literalmente com o exemplo do METACODE.md (`fn build(c: &Container) -> Self`) sem forçar todo adapter a escrever `async fn` + `Result` quando não precisa.

## Fora de escopo desta feature (M2)

- `rudi::testing::with_container` — feature M3 separada.
- Detecção de dependência circular — não é objetivo de nenhuma milestone da v1.
