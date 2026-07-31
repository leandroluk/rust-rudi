# DI Macros Specification

## Problem Statement

A API programática do M1 (`register_singleton`, `bind_with`, `resolve`) funciona mas exige boilerplate repetitivo: escrever manualmente o closure builder toda vez, e passar `&Container` explícito em toda função que precisa resolver algo. `rudi-macros` (M2) adiciona `#[injectable]`, `#[inject]` e `#[derive(Injectable)]` pra eliminar esse boilerplate, batendo com a superfície de API descrita no METACODE.md.

## Goals

- [ ] `#[injectable]` em `fn build(c: &Container) -> Self` (ou variantes async/fallível) gera `impl Injectable for Tipo`, habilitando `bind::<Impl, Port>()` e `register_singleton_injectable::<T>()` sem closure manual
- [ ] `#[inject]` remove parâmetro marcado `#[container]` da assinatura pública, resolve `rudi::container()` internamente
- [ ] `#[derive(Injectable)]` resolve cada campo do struct do container automaticamente (sem constructor custom)

## Out of Scope

| Feature | Reason |
| --- | --- |
| `rudi::testing::with_container` | M3 separada |
| Detecção de dependência circular | Não é objetivo de nenhuma milestone da v1 |
| `#[inject]` sem marker attribute (inferência por tipo) | Tecnicamente impossível de forma robusta (proc-macro não resolve alias) — decisão em context.md |

---

## User Stories

### P1: `#[injectable]` gera `impl Injectable` ⭐ MVP

**User Story**: Como dev consumidor, quero anotar o `impl Tipo { fn build(c: &Container) -> Self ... }` com `#[injectable]` e ganhar automaticamente a capacidade de registrar esse tipo via `bind`/`register_singleton_injectable`, sem escrever o closure builder manual do M1.

**Why P1**: É o mecanismo central que faz os exemplos do METACODE.md (`LoggerSlogAdapter`, `DatabasePostgresAdapter`, `DatabaseMongodbAdapter`) funcionarem — com 1 ajuste de posicionamento do attribute (vai no `impl` block, não na fn — ver design.md, restrição de linguagem de proc-macros, não escolha arbitrária).

**Acceptance Criteria**:

1. WHEN `#[injectable]` decora `impl Tipo { fn build(c: &Container) -> Self ... }` (síncrona, infalível) THEN macro SHALL gerar `impl Injectable for Tipo` com `type Error = Infallible`, chamando `Tipo::build` internamente
2. WHEN `#[injectable]` decora `impl Tipo { async fn build(c: &Container) -> Result<Self, E> ... }` THEN macro SHALL gerar `impl Injectable` repassando o `Result` como está, sem envolver em `Ok(...)` extra
3. WHEN `#[injectable]` é usado sem argumento THEN `type Port` gerado SHALL ser `Self` (resolução por tipo concreto, sem trait)
4. WHEN `#[injectable(dyn PortTrait)]` é usado THEN `type Port` gerado SHALL ser `dyn PortTrait`, habilitando `c.bind::<Tipo, dyn PortTrait>()`
5. WHEN o `impl` decorado não tem uma fn chamada `build`, ou `build` não tem exatamente 1 parâmetro do tipo `&Container`/`Container` THEN macro SHALL falhar em compile-time com mensagem clara (`compile_error!`)

**Independent Test**: aplicar `#[injectable]` num struct simples, chamar `Container::register_singleton_injectable::<Tipo>()`, resolver e comparar.

---

### P1: `Container::bind::<Impl, Port>()` (sem closure manual) ⭐ MVP

**User Story**: Como dev consumidor, quero chamar `c.bind::<LoggerSlogAdapter, dyn LoggerPort>()` (2 turbofish, sem argumento) exatamente como no METACODE.md, e o container resolver a implementação via o `Injectable` gerado por `#[injectable]`.

**Why P1**: É a sintaxe exata mostrada no METACODE.md para bind de porta — sem isso a macro não cumpre a proposta original.

**Acceptance Criteria**:

1. WHEN `Impl: Injectable<Port = Port>` (via `#[injectable(dyn Port)]`) THEN `c.bind::<Impl, Port>()` SHALL compilar e registrar `Impl` resolvível via `resolve::<Arc<Port>>()`
2. WHEN `Impl: Injectable<Port = Impl>` (via `#[injectable]` sem argumento) THEN `c.bind::<Impl, Impl>()` NÃO É o uso pretendido — usar `register_singleton_injectable::<Impl>()` nesse caso (documentar a diferença)
3. WHEN `bind::<Impl, Port>()` é chamado 2x com `Impl`s diferentes pra mesma `Port` THEN regra "última vence" do M1 SHALL se aplicar (reuso de `register_singleton` por baixo)

**Independent Test**: reproduzir exatamente `c.bind::<LoggerSlogAdapter, dyn LoggerPort>()` do METACODE.md e resolver.

---

### P1: `#[inject]` remove parâmetro Container da assinatura pública ⭐ MVP

**User Story**: Como dev consumidor, quero escrever `#[inject] fn init(#[container] c: &Container) { ... }` e chamar `init()` sem argumento no call site — a macro resolve `rudi::container()` sozinha.

**Why P1**: Reproduz a superfície `slog::init()` (chamada sem argumento) descrita no comentário do METACODE.md (`src/infra/logger/slog/mod.rs`).

**Acceptance Criteria**:

1. WHEN `#[inject]` decora uma fn com exatamente 1 parâmetro marcado `#[container]` THEN macro SHALL remover esse parâmetro da assinatura pública gerada
2. WHEN a fn decorada é chamada no call site sem esse argumento THEN macro SHALL injetar `rudi::container()` (ou a instância resolvida) como valor desse parâmetro, na 1ª linha do corpo
3. WHEN nenhum parâmetro tem `#[container]` THEN macro SHALL falhar em compile-time com mensagem clara
4. WHEN mais de 1 parâmetro tem `#[container]` THEN macro SHALL falhar em compile-time com mensagem clara

**Independent Test**: função com `#[container]` recebendo `&Container` sob alias de import (`use rudi::Container as C`), chamada sem argumento, resolve algo do container corretamente.

---

### P2: `#[derive(Injectable)]` — resolução campo a campo

**User Story**: Como dev consumidor, quero usar `#[derive(Injectable)]` num struct sem constructor custom, pra que cada campo seja resolvido individualmente do container (via `resolve::<TipoDoCampo>()`).

**Why P2**: METACODE.md menciona esse caso ("quando não tem constructor custom, cada campo é resolvido do container") mas nenhum dos 2 exemplos completos (Logger/Database) o usa — não bloqueia o MVP.

**Acceptance Criteria**:

1. WHEN `#[derive(Injectable)]` decora um struct com campos nomeados THEN macro SHALL gerar `impl Injectable` cujo `build` resolve cada campo via `c.resolve::<TipoDoCampo>().await` e monta o struct
2. WHEN algum campo falha ao resolver (tipo não registrado) THEN `build` SHALL propagar o erro (`RudiError` convertido para o `Error` associado)
3. WHEN o struct tem campos tuple (não nomeados) THEN macro SHALL suportar do mesmo jeito, na ordem declarada

**Independent Test**: struct com 2 campos de tipos diferentes já registrados no container, `#[derive(Injectable)]`, `register_singleton_injectable`, resolve e confirma os 2 campos.

---

## Edge Cases

- WHEN `#[injectable]` é aplicado a algo que não é uma fn dentro de um `impl` de struct (ex: fn solta) THEN macro SHALL falhar em compile-time
- WHEN `#[inject]` é aplicado a uma fn `async` THEN macro SHALL preservar `async` na assinatura gerada (injeção não interfere em async-ness da fn)
- WHEN `#[derive(Injectable)]` é usado num struct com 0 campos THEN `build` gerado SHALL retornar `Self` sem nenhum `resolve` (caso degenerado válido)

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| MACRO-01 | P1: #[injectable] sync/infalível | Design | Pending |
| MACRO-02 | P1: #[injectable] async/Result | Design | Pending |
| MACRO-03 | P1: #[injectable] Port default = Self | Design | Pending |
| MACRO-04 | P1: #[injectable(dyn Port)] | Design | Pending |
| MACRO-05 | P1: #[injectable] validação compile-time | Design | Pending |
| MACRO-06 | P1: bind::<Impl,Port>() via Injectable | Design | Pending |
| MACRO-07 | P1: bind última-vence (reuso M1) | Design | Pending |
| MACRO-08 | P1: #[inject] remove parâmetro #[container] | Design | Pending |
| MACRO-09 | P1: #[inject] injeta container() no corpo | Design | Pending |
| MACRO-10 | P1: #[inject] validação compile-time (0 ou 2+ #[container]) | Design | Pending |
| MACRO-11 | P2: #[derive(Injectable)] campo a campo | Design | Pending |
| MACRO-12 | P2: #[derive(Injectable)] propagação de erro | Design | Pending |

**ID format:** `MACRO-NN`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 12 total, 0 mapped to tasks, 12 unmapped ⚠️ (Design phase ainda não rodou)

---

## Success Criteria

- [ ] Todos os arquivos `src/infra/**/*.rs` do METACODE.md compilam e rodam literalmente como escritos (usando `rudi::{Container, Injectable}`, `#[injectable]`, `#[inject]`) — critério mais forte que M1 (que só cobria a versão "sem macro")
- [ ] `cargo test` cobre as 12 acceptance criteria
- [ ] Erros de uso incorreto das macros falham em compile-time com mensagem legível (não panic em runtime)
