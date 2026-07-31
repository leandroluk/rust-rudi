# rudi

Injeção de dependência type-safe para Rust, com resolução assíncrona garantida e macros para eliminar boilerplate de wiring manual.

- **Container ambiente** — `rudi::container()` é lazy, 1 instância por processo. Ninguém instancia nem repassa na mão.
- **Resolução sempre assíncrona** — mesmo para tipos/construtores síncronos, garantindo suporte uniforme a builders que precisam de I/O (conexão de banco, etc).
- **Bind de porta (trait objects)** — registre uma implementação concreta contra uma trait (`LoggerPort`, `DatabasePort`, ...), o resto do código resolve só pela porta, sem saber qual implementação está por trás.
- **Sem leitura de env pela lib** — quem lê variável de ambiente é sempre o `init()` do consumidor.

## Instalação

```toml
[dependencies]
rudi = { path = "crates/rudi" } # ou a versão publicada, quando existir
```

## Início rápido

```rust
use rudi::{injectable, Container};

struct Greeter;

#[injectable]
impl Greeter {
    fn build(_c: &Container) -> Self {
        Greeter
    }
}

impl Greeter {
    fn hello(&self) -> &'static str {
        "olá!"
    }
}

#[tokio::main]
async fn main() {
    let c = rudi::container();
    c.register_singleton_injectable::<Greeter>();

    let greeter = c.resolve::<Greeter>().await.unwrap();
    println!("{}", greeter.hello());
}
```

## Registro de dependências

4 formas de registrar algo no container:

```rust
use rudi::Container;

let c = Container::new(); // container local — use isto em testes, nunca o global

// 1. Instância já pronta (ex: config validada fora da lib)
c.register_instance(MyConfig { level: "info".into() });

// 2. Factory que roda de novo a cada resolve (sem cache)
c.register_transient::<MyType, _, _, std::convert::Infallible>(|c| async move {
    Ok(MyType::new())
});

// 3. Factory cacheada — 1ª resolução executa, demais retornam a mesma instância
// (double-init safe sob concorrência)
c.register_singleton::<MyType, _, _, std::convert::Infallible>(|c| async move {
    Ok(MyType::new())
});

// 4. Bind de porta — Impl resolvível via Arc<dyn Port>, sem closure manual,
// usando o Injectable gerado por #[injectable]
c.bind::<MyAdapter, dyn MyPort>();
```

Toda variante tem versão nomeada (`register_instance_named`, `register_transient_named`, `register_singleton_named`) para coexistir múltiplas instâncias do mesmo tipo — por exemplo, um banco `primary` e um `replica`:

```rust
c.register_instance_named("primary", DbConfig { uri: "postgres://primary".into() });
c.register_instance_named("replica", DbConfig { uri: "postgres://replica".into() });

let primary = c.resolve_named::<DbConfig>("primary").await.unwrap();
let replica = c.resolve_named::<DbConfig>("replica").await.unwrap();
```

## Resolução

`resolve` é sempre `async` e sempre retorna `Arc<T>`:

```rust
let value = c.resolve::<MyType>().await?;       // Arc<MyType>
let named = c.resolve_named::<MyType>("x").await?;
let port  = c.resolve::<Arc<dyn MyPort>>().await?; // porta via trait object
```

Tipo não registrado retorna `Err(RudiError::NotFound { .. })` — nunca panic.

## Macros

### `#[injectable]`

Decora o **bloco `impl` inteiro** (não a fn `build` isolada — proc-macro attribute não enxerga o escopo ao redor do item que decora, então precisa ir 1 nível acima). Gera `impl Injectable` a partir de `fn build(c: &Container) -> Self`.

```rust
use rudi::{injectable, Container};

struct LoggerSlogAdapter {
    config: LoggerSlogConfig,
}

#[injectable(dyn LoggerPort)] // porta opcional — sem argumento, resolve por tipo concreto
impl LoggerSlogAdapter {
    async fn build(c: &Container) -> Self {
        let config = c.resolve::<LoggerSlogConfig>().await.unwrap();
        Self { config: (*config).clone() }
    }
}

impl LoggerPort for LoggerSlogAdapter {
    fn info(&self, message: &str) { println!("[info] {message}"); }
    // ...
}
```

Suporta as 4 combinações de assinatura pra `build`: síncrona ou `async`, retornando `Self` (infalível) ou `Result<Self, E>`.

```rust
c.bind::<LoggerSlogAdapter, dyn LoggerPort>();
let logger = c.resolve::<Arc<dyn LoggerPort>>().await.unwrap();
logger.info("Hello World!");
```

### `#[inject]`

Remove um parâmetro marcado `#[container]` da assinatura pública, injetando `rudi::container()` como 1ª linha do corpo. Funciona mesmo com alias de import (`use rudi::Container as C`), porque depende do atributo, não do nome do tipo.

```rust
use rudi::{inject, Container};

#[inject]
fn init(#[container] c: &Container) {
    c.register_instance(MyConfig::default());
}

fn main() {
    init(); // sem argumento — a macro resolve o container sozinha
}
```

### `#[derive(Injectable)]`

Resolve cada campo do struct individualmente, um a um, do container. Todo campo precisa ser `Arc<T>` (mesmo formato que `resolve` sempre retorna):

```rust
use std::sync::Arc;

#[derive(rudi::Injectable)]
struct Service {
    logger: Arc<dyn LoggerPort>,
    config: Arc<MyConfig>,
}

c.register_singleton_injectable::<Service>();
let service = c.resolve::<Service>().await.unwrap();
```

## Testes

Nunca use o container global (`rudi::container()`) em teste — ele é compartilhado entre todos os testes do processo. Use `Container::new()` local, ou o helper `with_container`:

```rust
use rudi::testing::with_container;

#[tokio::test]
async fn my_test() {
    with_container(|c| async move {
        c.register_instance(MyConfig::default());
        let resolved = c.resolve::<MyConfig>().await.unwrap();
        assert_eq!(resolved.level, "info");
    })
    .await;
}
```

## Exemplo completo

Veja [`crates/rudi/examples/metacode/`](crates/rudi/examples/metacode/) — reprodução da árvore `domain`/`infra` descrita em [`METACODE.md`](METACODE.md) (logger + database com 2 providers, seleção por env do lado do consumidor), usando as macros de ponta a ponta:

```bash
cargo run --example metacode -p rudi
```

## Status

Todas as milestones da v1 (`.specs/project/ROADMAP.md`) estão completas:

- **M1** — container core (registro instância/transient/singleton, bind de porta, resolve async, named bindings)
- **M2** — macros (`#[injectable]`, `#[inject]`, `#[derive(Injectable)]`)
- **M3** — `rudi::testing::with_container`

Fora de escopo da v1: bindings FFI, tracing/observability de resolução, detecção de dependência circular em compile-time (hoje é runtime deadlock — ver limitações abaixo).

## Limitações conhecidas

- Dependência circular entre builders não é detectada — vira deadlock em runtime. Documentado, não tratado na v1.
- `#[injectable]` no parâmetro de `build` não aceita alias de import (precisa ser `Container`/`&Container` literal) — diferente do `#[inject]`, que usa marker attribute e por isso não tem essa limitação.
