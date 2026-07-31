# Core Container Specification

## Problem Statement

Projetos rust em padrão hexagonal (ports & adapters) fazem wiring manual: config → adapter → registro → trait object, repetido pra cada porta/provider, geralmente num `main.rs` gigante ou `init()` encadeado à mão. `rudi` centraliza isso num container global type-safe, com resolução async garantida mesmo pra dependências síncronas, pra suportar builders que precisam abrir conexão (DB, etc) sem forçar sync-over-async.

## Goals

- [ ] Container global (`rudi::container()`) — 1 instância lazy por processo, sem consumidor instanciar na mão
- [ ] 4 modos de registro (instance/factory/singleton/transient) + bind de porta, todos resolvíveis via `resolve::<T>()` async
- [ ] Named bindings — múltiplas instâncias do mesmo tipo coexistem sob nomes distintos

## Out of Scope

| Feature | Reason |
| --- | --- |
| Macros (`#[injectable]`, `#[inject]`, `#[derive(Injectable)]`) | Feature M2 separada — este spec é só a API programática do container |
| `rudi::testing::with_container` | Feature M3 separada |
| Detecção de dependência circular em compile-time | Fica runtime panic por ora (documentado como limitação) |
| Leitura de env pela lib | Regra fechada no METACODE.md — lib nunca chama `std::env` |

---

## User Stories

### P1: Registrar e resolver instância pronta ⭐ MVP

**User Story**: Como dev consumidor, quero registrar um valor já construído (ex: config validada) no container, pra que outros construtores possam resolvê-lo sem saber como foi criado.

**Why P1**: É o registro mais simples e é pré-requisito dos outros (factory/singleton resolvem dependências que geralmente são instances registradas primeiro, ex: config).

**Acceptance Criteria**:

1. WHEN consumidor chama `c.register_instance(valor)` THEN container SHALL guardar `valor` associado ao tipo `T` de `valor`
2. WHEN consumidor chama `c.resolve::<T>().await` após registro THEN container SHALL retornar o mesmo valor (clonado ou por referência compartilhada, a depender de `T: Clone` — decidir em design)
3. WHEN consumidor chama `resolve::<T>()` sem registro prévio de `T` THEN container SHALL retornar erro tipado (não panic) identificando o tipo ausente

**Independent Test**: registrar uma struct simples via `register_instance`, resolver e comparar valor.

---

### P1: Registrar factory/singleton com resolução async de dependências ⭐ MVP

**User Story**: Como dev consumidor, quero registrar um construtor (`fn(&Container) -> Fut<Output = T>`) que resolve suas próprias dependências do container, pra montar adapters cuja construção depende de config/outros adapters já registrados.

**Why P1**: É o caso central do METACODE (adapters `build(c: &Container) -> Self` resolvendo config); sem isso a lib não cumpre a proposta.

**Acceptance Criteria**:

1. WHEN consumidor chama `c.register_factory::<T>(builder)` THEN container SHALL armazenar `builder` sem executá-lo
2. WHEN consumidor chama `c.resolve::<T>().await` pela 1ª vez após `register_factory` THEN container SHALL executar `builder(&c).await` e retornar o resultado
3. WHEN `resolve::<T>()` é chamado de novo para um tipo registrado via `register_factory` THEN container SHALL executar `builder` de novo (nova instância a cada resolve — comportamento transient)
4. WHEN consumidor chama `c.register_singleton::<T>()` (sem closure, usa builder do próprio tipo — pré-requisito: macro/trait `Injectable` na M2, mas a API do container deve aceitar builder equivalente já na M1) THEN 1ª resolução SHALL executar o builder e cachear; resoluções seguintes SHALL retornar a instância cacheada sem reexecutar
5. WHEN builder de factory/singleton falha (retorna erro) THEN `resolve` SHALL propagar erro tipado, não panic

**Independent Test**: registrar factory que incrementa contador global a cada chamada; resolver 2x tipo singleton (contador fica em 1) e 2x tipo transient/factory puro (contador vai a 2).

---

### P1: Bind de porta (trait object) com seleção última-vence ⭐ MVP

**User Story**: Como dev consumidor, quero registrar uma implementação concreta contra uma trait (`bind::<Impl, dyn Port>()`), pra que o resto do código resolva só pela porta sem saber qual impl concreta está por trás.

**Why P1**: É o mecanismo que viabiliza troca de provider por env (postgres vs mongodb) sem o `mod.rs` consumidor saber qual foi escolhido — central ao caso de uso do METACODE.

**Acceptance Criteria**:

1. WHEN consumidor chama `c.bind::<Impl, dyn Port>()` THEN container SHALL registrar `Impl` resolvível como `Arc<dyn Port>`
2. WHEN consumidor chama `resolve::<Arc<dyn Port>>().await` THEN container SHALL retornar a última `Impl` bindada pra aquela porta (se `bind` for chamado 2x pra mesma `dyn Port`, o 2º vence)
3. WHEN nenhuma impl foi bindada pra `dyn Port` e `resolve::<Arc<dyn Port>>()` é chamado THEN container SHALL retornar erro tipado

**Independent Test**: bindar 2 impls diferentes da mesma trait em sequência, resolver e confirmar que só a 2ª responde.

---

### P2: Named bindings — múltiplas instâncias do mesmo tipo

**User Story**: Como dev consumidor, quero registrar/resolver mais de uma instância do mesmo tipo sob nomes distintos (ex: postgres primary + replica), pra não precisar criar tipos wrapper artificiais só pra diferenciar.

**Why P2**: Resolve um "em aberto" do METACODE; não bloqueia o caso de uso single-provider do MVP, mas é necessário pro roadmap completo.

**Acceptance Criteria**:

1. WHEN consumidor chama `c.register_instance_named::<T>("nome", valor)` (ou variante equivalente pros outros register_*) THEN container SHALL guardar `valor` sob a chave composta `(TypeId, "nome")`
2. WHEN consumidor chama `c.resolve_named::<T>("nome").await` THEN container SHALL retornar a instância registrada sob esse nome
3. WHEN `resolve_named::<T>("nome")` é chamado com nome não registrado THEN container SHALL retornar erro tipado (não confundir com erro de tipo ausente sem nome)
4. WHEN `register_instance::<T>(valor)` (sem nome) e `register_instance_named::<T>("x", valor2)` coexistem THEN ambos SHALL ser resolvíveis independentemente (`resolve::<T>()` retorna o sem-nome, `resolve_named::<T>("x")` retorna o nomeado)

**Independent Test**: registrar `DatabasePostgresConfig` sob "primary" e "replica", resolver ambos os nomes e confirmar URIs diferentes.

---

## Edge Cases

- WHEN `resolve::<T>()` é chamado para tipo nunca registrado THEN container SHALL retornar `Err` tipado com nome do tipo (via `std::any::type_name`), nunca panic
- WHEN 2 registros do mesmo tipo sem nome ocorrem (`register_instance::<T>` chamado 2x) THEN o 2º SHALL sobrescrever o 1º (mesma regra "último vence" do `bind`)
- WHEN dependência circular ocorre entre builders (A resolve B que resolve A) THEN comportamento é runtime deadlock/panic — documentado como limitação conhecida, não é objetivo da v1 detectar
- WHEN `resolve` é chamado de múltiplas threads concorrentemente pro mesmo singleton ainda não construído THEN container SHALL garantir que o builder executa só 1 vez (sem double-init) — requer sincronização interna (`OnceCell`/`Mutex` async-safe)
- WHEN container global (`rudi::container()`) é acessado antes de qualquer `register_*` THEN SHALL retornar instância vazia válida (não erro) — erro só ocorre no `resolve` de tipo ausente

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| CORE-01 | P1: Registrar/resolver instância | Design | Pending |
| CORE-02 | P1: Erro tipado em resolve sem registro | Design | Pending |
| CORE-03 | P1: register_factory + resolve async | Design | Pending |
| CORE-04 | P1: register_singleton com cache | Design | Pending |
| CORE-05 | P1: propagação de erro do builder | Design | Pending |
| CORE-06 | P1: bind porta / trait object | Design | Pending |
| CORE-07 | P1: última bind vence | Design | Pending |
| CORE-08 | P2: named register/resolve | Design | Pending |
| CORE-09 | Edge: concorrência singleton sem double-init | Design | Pending |
| CORE-10 | Edge: container vazio válido antes de qualquer register | Design | Pending |

**ID format:** `CORE-NN`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 10 total, 0 mapped to tasks, 10 unmapped ⚠️ (Design phase ainda não rodou)

---

## Success Criteria

- [ ] Exemplo do METACODE.md (logger slog + database postgres/mongodb, dois containers com env diferente) compila e roda usando só API deste spec (sem macros ainda — construtores chamados manualmente onde macro entraria)
- [ ] `cargo test` cobre as 10 acceptance criteria acima com 1 teste cada no mínimo
- [ ] Nenhuma chamada a `std::env` dentro do crate `rudi` (core)
