# Constructor injection via `#[inject]`/`#[inject_all]` params (M5, brief)

## Problem Statement

`#[injectable]` hoje só aceita `fn build(c: &Container) -> Self`, com resolução manual de cada dependência no corpo (`c.resolve::<X>().await`). Padrão mais ergonômico (TS/Nest-style: `constructor(@Inject() x: X)`): cada parâmetro do "construtor" marcado com o que precisa ser injetado, resolvido automaticamente, construtor em si fica síncrono e testável sem `.await`.

## Bug encontrado no processo

`#[derive(Injectable)]` (M2) nunca foi testado com campo `Arc<dyn Trait>` (só tipos concretos). `c.resolve::<dyn Trait>()` não compila — `resolve<T: Send+Sync+'static>` exige `T: Sized` implícito, `dyn Trait` é unsized. Precisa resolver via `resolve::<Arc<dyn Trait>>()` (T=`Arc<dyn Trait>`, Sized) e achatar o double-Arc resultante. Corrigido junto (mesmo helper compartilhado).

## Goals

- [ ] `#[injectable]` no `impl` block aceita 2º estilo de constructor: fn sem `self`, todo parâmetro marcado `#[inject]`/`#[inject_all]`/`#[container]` (qualquer nome de fn — Rust não tem "constructor" de linguagem, detecção é por forma)
- [ ] `#[inject]` em parâmetro `Arc<T>` → resolve por tipo (`resolve::<T>()` se `T` concreto; `resolve::<Arc<T>>()` + achatar se `T` é `dyn Trait`)
- [ ] `#[inject_all]` em parâmetro `Vec<Arc<T>>` → `resolve_all` (mesma lógica de achatamento)
- [ ] `#[container]` em parâmetro continua funcionando igual (injeta o `Container` cru, sem resolve)
- [ ] Construtor em si (a fn marcada) fica **intocado** — síncrono se foi escrito síncrono, chamável direto em teste sem `.await`. Só o `Injectable::build` gerado é async.
- [ ] Corrigir `#[derive(Injectable)]` pra campos `Arc<dyn Trait>` (bug acima)

## Out of Scope

| Feature | Reason |
| --- | --- |
| `#[inject]`/`#[inject_all]` fora de `#[injectable]` (ex: em fn livre) | Sem caso de uso — o `#[inject]` fn-attribute (M2) já cobre "resolve container cru sem argumento"; isso aqui é especificamente sobre construtor injetável |
| Parâmetros não-marcados misturados com marcados no mesmo constructor | `Injectable::build` só tem o `Container` disponível — não tem de onde vir um argumento manual extra |

## Requirements (WHEN/THEN)

1. WHEN `impl Tipo` tem 1 fn sem `self` com todo parâmetro marcado THEN `#[injectable]` SHALL gerar `Injectable::build` que resolve cada parâmetro e chama essa fn
2. WHEN 0 fns qualificam (nem `build` clássico, nem construtor com params marcados) THEN `compile_error!`
3. WHEN 2+ fns qualificam (ambíguo) THEN `compile_error!`
4. WHEN parâmetro marcado `#[inject]` é `Arc<TipoConcreto>` THEN resolve direto (`resolve::<TipoConcreto>()`)
5. WHEN parâmetro marcado `#[inject]` é `Arc<dyn Trait>` THEN resolve via `Arc<dyn Trait>` + achata
6. WHEN parâmetro marcado `#[inject_all]` é `Vec<Arc<T>>` (concreto ou `dyn Trait`) THEN resolve via `resolve_all`, mesma lógica de achatamento por item
7. WHEN `#[derive(Injectable)]` tem campo `Arc<dyn Trait>` THEN compila e resolve corretamente (regressão corrigida)

## Success Criteria

- [ ] Teste: construtor com `#[inject]` concreto + `#[inject]` trait-object + `#[container]` misturados, todos resolvidos certo
- [ ] Teste: `#[inject_all]` com `Vec<Arc<dyn Port>>`, populado via `bind_many` prévio
- [ ] Teste: `#[derive(Injectable)]` com campo `Arc<dyn Trait>` (regressão)
- [ ] Compile-fail: 2 fns candidatas no mesmo impl (ambíguo); 0 fns candidatas; parâmetro parcialmente marcado (alguns sim, outros não) numa fn que não é `build`
- [ ] Construtor gerado continua chamável manualmente sem `.await` (síncrono, testável)
- [ ] `cargo test --workspace` verde, gate full
