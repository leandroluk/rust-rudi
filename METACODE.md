# hipótese mínima estrutural para injeção de dependência em rust

- estamos criando uma hipótese estrutural para uma biblioteca de injeção de dependência em rust chamada "rudi".
- a intenção é que ela seja type-safe e utilize macros + attributes pra fazer o registro tanto via instância quanto via factory quanto via bind de porta (trait), ex:
- `Container` é ambiente/global dentro da lib (1 só por processo) — ninguém instancia nem repassa na mão, `#[inject]` acha ele sozinho.
- 2 portas de exemplo: `LoggerPort` (1 provider: slog) e `DatabasePort` (2 providers: postgres e mongodb) — pra mostrar o padrão de seleção por env quando tem mais de 1 implementação.

> arquitetura do projeto
```
src/
|- domain/
  |- port/
    |- database.rs
    |- logger.rs
    |- mod.rs
  |- mod.rs
|- infra/
  |- database/
    |- mongodb/
      |- config.rs
      |- adapter.rs
      |- mod.rs
    |- postgres/
      |- config.rs
      |- adapter.rs
      |- mod.rs
    |- mod.rs
  |- logger/
    |- slog/
      |- config.rs
      |- adapter.rs
      |- mod.rs
    |- mod.rs
  |- mod.rs
|- main.rs
```

> src/domain/port/logger.rs
```rs
pub trait LoggerPort: Send + Sync {
    fn log(&self, level: &str, message: &str);
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
}
```

> src/domain/port/database.rs
```rs
pub trait DatabasePort: Send + Sync {
    async fn ping(&self) -> Result<(), Error>;
}
```

> src/domain/port/mod.rs
```rs
pub mod database;
pub mod logger;

pub use database::DatabasePort;
pub use logger::LoggerPort;
```

> src/domain/mod.rs
```rs
pub mod port;
pub use port::{DatabasePort, LoggerPort};
```

> src/infra/logger/slog/config.rs
```rs
use garde::Validate;

// sem Injectable aqui: config nao tem constructor custom nem se auto-resolve,
// ela e' registrada como instancia crua em `init()` (register_instance).
#[derive(Clone, Validate)]
pub struct LoggerSlogConfig {
    #[garde(pattern(r"^(off|crit|error|warn|info|debug|trace)$"))]
    pub level: String,
}
```

> src/infra/logger/slog/adapter.rs
```rs
use std::collections::HashMap;
use rudi::{Container, Injectable};
use slog::{Drain, LevelFilter, Logger};

use crate::domain::port::LoggerPort;
use crate::infra::logger::slog::config::LoggerSlogConfig;

pub struct LoggerSlogAdapter {
    level_map: HashMap<String, LevelFilter>,
    logger: Logger,
    pub config: LoggerSlogConfig,
}

impl LoggerSlogAdapter {
    // #[injectable]: marca essa fn como o construtor de Self pro container
    // (usada por `bind`/`register_singleton`). Recebe `&Container` explicito
    // de proposito — quem chama e' o proprio container por dentro, nao um
    // call site solto, entao nao precisa de `#[inject]` aqui.
    #[injectable]
    pub fn build(c: &Container) -> Self {
        Self {
            config: c.resolve::<LoggerSlogConfig>(),
            logger: slog::Logger::root(slog::Discard, slog::o!()),
            level_map: HashMap::from([
                ("off".to_string(), LevelFilter::Off),
                ("crit".to_string(), LevelFilter::Critical),
                ("error".to_string(), LevelFilter::Error),
                ("warn".to_string(), LevelFilter::Warning),
                ("info".to_string(), LevelFilter::Info),
                ("debug".to_string(), LevelFilter::Debug),
                ("trace".to_string(), LevelFilter::Trace),
            ]),
        }
    }

    fn dispatch(&self, level: &str, message: &str) {
        if let Some(level) = self.level_map.get(level) {
            slog::log!(&self.logger, *level, "{}", message);
        }
    }
}

impl LoggerPort for LoggerSlogAdapter {
    fn log(&self, level: &str, message: &str) { self.dispatch(level, message); }
    fn info(&self, message: &str) { self.dispatch("info", message); }
    fn warn(&self, message: &str) { self.dispatch("warn", message); }
    fn error(&self, message: &str) { self.dispatch("error", message); }
}
```

> src/infra/logger/slog/mod.rs
```rs
pub mod adapter;
pub mod config;

pub use adapter::LoggerSlogAdapter;
pub use config::LoggerSlogConfig;

use rudi::Container;

use crate::domain::port::LoggerPort;

// #[inject]: `c` some da assinatura publica, macro resolve do container
// ambiente e injeta como 1a linha do corpo. Call site vira `slog::init()`.
pub fn init(c: &Container) {
    c.register_instance(LoggerSlogConfig {
        level: std::env::var("LOGGER_SLOG_LEVEL").unwrap(),
    });
    c.bind::<LoggerSlogAdapter, dyn LoggerPort>();
}
```

> src/infra/logger/mod.rs
```rs
pub mod slog;

pub use slog::{LoggerSlogAdapter, LoggerSlogConfig};

use rudi::Container;

pub fn init(c: &Container) {
    let provider = std::env::var("LOGGER_PROVIDER").unwrap();
    match provider.as_str() {
        "slog" => slog::init(c),
        other => panic!("Invalid logger provider: {other}"),
    }
}
```

> src/infra/database/postgres/config.rs
```rs
use garde::Validate;

#[derive(Clone, Validate)]
pub struct DatabasePostgresConfig {
    #[garde(pattern(r"^postgres(ql)?://"))]
    pub uri: String,
}
```

> src/infra/database/postgres/adapter.rs
```rs
use rudi::{Container, Injectable};

use crate::domain::port::DatabasePort;
use crate::infra::database::postgres::config::DatabasePostgresConfig;

pub struct DatabasePostgresAdapter {
    config: DatabasePostgresConfig,
}

impl DatabasePostgresAdapter {
    // constructor coerente com o padrão do LoggerSlogAdapter: nada explícito
    // além do `#[injectable]`, só resolve a própria config do container.
    #[injectable]
    pub fn build(c: &Container) -> Self {
        Self { config: c.resolve::<DatabasePostgresConfig>() }
    }
}

impl DatabasePort for DatabasePostgresAdapter {
    fn ping(&self) -> bool {
        true // sqlx::PgPool::connect(&self.config.uri)...
    }
}
```

> src/infra/database/postgres/mod.rs
```rs
use rudi::Container;
use crate::domain::port::DatabasePort;

pub mod adapter;
pub mod config;

pub use adapter::DatabasePostgresAdapter;
pub use config::DatabasePostgresConfig;

pub fn init(c: &Container) {
    c.register_instance(DatabasePostgresConfig {
        uri: std::env::var("DATABASE_POSTGRES_URI").unwrap(),
    });
    c.bind::<DatabasePostgresAdapter, dyn DatabasePort>();
}
```

> src/infra/database/mongodb/config.rs
```rs
use garde::Validate;

#[derive(Clone, Validate)]
pub struct DatabaseMongodbConfig {
    #[garde(pattern(r"^mongodb(\+srv)?://"))]
    pub uri: String,
}
```

> src/infra/database/mongodb/adapter.rs
```rs
use rudi::{Container, Injectable};

use crate::domain::port::DatabasePort;
use crate::infra::database::mongodb::config::DatabaseMongodbConfig;

pub struct DatabaseMongodbAdapter {
    config: DatabaseMongodbConfig,
}

impl DatabaseMongodbAdapter {
    // mesmo padrão do adapter postgres — só troca o tipo de config resolvido.
    #[injectable]
    pub fn build(c: &Container) -> Self {
        Self { config: c.resolve::<DatabaseMongodbConfig>() }
    }
}

impl DatabasePort for DatabaseMongodbAdapter {
    fn ping(&self) -> bool {
        true // mongodb::Client::with_uri_str(&self.config.uri)...
    }
}
```

> src/infra/database/mongodb/mod.rs
```rs
use rudi::Container;
use crate::domain::port::DatabasePort;

pub mod adapter;
pub mod config;

pub use adapter::DatabaseMongodbAdapter;
pub use config::DatabaseMongodbConfig;

pub fn init(c: &Container) {
    c.register_instance(DatabaseMongodbConfig {
        uri: std::env::var("DATABASE_MONGODB_URI").unwrap(),
    });
    c.bind::<DatabaseMongodbAdapter, dyn DatabasePort>();
}
```

> src/infra/database/mod.rs
```rs
pub mod mongodb;
pub mod postgres;

pub use mongodb::{DatabaseMongodbAdapter, DatabaseMongodbConfig};
pub use postgres::{DatabasePostgresAdapter, DatabasePostgresConfig};

// mesmo padrão de seleção por env do infra/logger/mod.rs — só troca a
// variável e as opções do match. `bind` sempre acaba na mesma `dyn DatabasePort`,
// então quem resolve depois nem sabe qual dos dois foi escolhido.

pub fn init(c: &Container) {
    let provider = std::env::var("DATABASE_PROVIDER").unwrap();
    match provider.as_str() {
        "postgres" => postgres::init(c),
        "mongodb" => mongodb::init(c),
        other => panic!("Invalid database provider: {other}"),
    }
}
```

> src/infra/mod.rs
```rs
pub mod database;
pub mod logger;

pub use database::{DatabaseMongodbAdapter, DatabaseMongodbConfig, DatabasePostgresAdapter, DatabasePostgresConfig};
pub use logger::{LoggerSlogAdapter, LoggerSlogConfig};

use rudi::Container;

pub fn init(c: &Container) {
    database::init(c);
    logger::init(c);
}
```

> src/main.rs
```rs
use std::sync::Arc;

use crate::domain::port::{DatabasePort, LoggerPort};

#[tokio::main]
async fn main() {
    assert!(dotenv::dotenv().is_ok());

    std::env::set_var("LOGGER_PROVIDER", "slog");
    std::env::set_var("DATABASE_PROVIDER", "postgres");
    let c1 = rudi::container();
    infra::init(&c1);
    c1.resolve::<Arc<dyn LoggerPort>>().info("Hello World!");
    c1.resolve::<Arc<dyn DatabasePort>>().ping().await.unwrap();

    std::env::set_var("LOGGER_PROVIDER", "slog");
    std::env::set_var("DATABASE_PROVIDER", "mongodb");
    let c2 = rudi::container();
    infra::init(&c2);
    c2.resolve::<Arc<dyn LoggerPort>>().info("Hello World 2!");
    c2.resolve::<Arc<dyn DatabasePort>>().ping().await.unwrap();
}
```

## Como se usa (superfície, sem implementação)

- `rudi::container()` — pega a instância ambiente (lazy, 1 por processo).
- `c.register_instance(valor)` — registra algo já pronto (ex: config lido/validado fora da lib).
- `c.register_factory::<Tipo>(|c| ...)` — registra como construir sob demanda, cacheado após a 1ª chamada.
- `c.register_singleton::<Tipo>()` — igual acima, mas usando o `Injectable` do próprio tipo em vez de passar closure na mão.
- `c.bind::<Impl, dyn Port>()` — registra `Impl` para ser resolvido pela porta (trait), não pelo tipo concreto. Quando 2 impls existem pra mesma porta (postgres/mongodb), só a que rodou `bind` por último/pela seleção de env fica registrada — resolver por trait nunca sabe qual impl concreta está por trás.
- `c.resolve::<Tipo>()` / `c.resolve::<Arc<dyn Port>>()` — busca o que foi registrado.
- `#[injectable]` numa fn `fn build(c: &Container) -> Self` — marca a fn como construtor de `Self` pro container (usado por `bind`/`register_singleton`).
- `#[inject]` numa fn qualquer com parâmetro `&Container` — tira esse parâmetro da assinatura pública; call site chama sem passar nada, macro busca o container ambiente sozinha.
- `#[derive(Injectable)]` no struct — quando não tem constructor custom, cada campo é resolvido do container.

### Regras já fechadas
- Biblioteca nunca chama `std::env` — quem lê env é sempre o `init()` do consumidor.
- Falha de config em `init()` é fail-fast do lado do consumidor, a lib não decide isso.
- `Container` é global/ambiente — consumidor nunca faz `Container::new()`, só usa `rudi::container()` (ou nem isso, se usar `#[inject]`).
- Múltiplos providers pra mesma porta (ex: `DatabasePort` com postgres/mongodb) seguem o mesmo padrão do logger: cada provider tem seu próprio `init()`, o `mod.rs` do nível da porta decide qual chamar via env, e o `bind` sempre é contra a `dyn Port` — nunca contra o tipo concreto do provider.

### Em aberto
- Escopo transient (hoje só singleton)?
- Múltiplas instâncias do **mesmo** provider (ex: postgres primary + postgres replica) — como nomear/resolver cada uma, já que `bind`/`resolve` são por tipo?
- Testes isolados: container global compartilhado entre testes é problema — precisa de algo tipo `rudi::testing::with_container(|c| {...})` pra escopo local?
- `#[inject]` detecta o parâmetro por token literal `&Container` (proc-macro não resolve tipo) — quebra se o consumidor importar com alias (`use rudi::Container as C`). Documentar como limitação ou resolver de outro jeito?
- `bind`/`resolve` hoje são "1 impl por porta, última vence" — não dá pra pegar **todo mundo** que implementa uma porta de uma vez (padrão comum de healthcheck: "resolve todos que sabem pingar e pinga cada um"). Precisa de `bind_many`/`resolve_all` separados de `bind`/`resolve` (sem mudar a semântica "última vence" que já existe), ver exemplo abaixo.

## Exemplo: healthcheck via multi-bind (`PingablePort`)

Motivação: `LoggerSlogAdapter`, `DatabasePostgresAdapter` e `DatabaseMongodbAdapter` acima já têm (ou poderiam ter) uma forma de "estou saudável?". Um healthcheck quer resolver **todos** os adapters que implementam essa porta, não só o último bindado — diferente do `DatabasePort`/`LoggerPort`, onde só 1 impl concreta interessa por vez.

> src/domain/port/pingable.rs
```rs
pub trait PingablePort: Send + Sync {
    async fn ping(&self) -> Result<(), Error>;
}
```

> src/domain/port/mod.rs (adição)
```rs
pub mod pingable;
pub use pingable::PingablePort;
```

> src/infra/database/postgres/adapter.rs (adição — mesmo `impl`, porta extra)
```rs
impl PingablePort for DatabasePostgresAdapter {
    async fn ping(&self) -> Result<(), Error> {
        self.config.uri.len(); // sqlx::PgPool::ping()...
        Ok(())
    }
}
```

> src/infra/database/postgres/mod.rs (`init` ganha 1 linha)
```rs
pub fn init(c: &Container) {
    c.register_instance(DatabasePostgresConfig {
        uri: std::env::var("DATABASE_POSTGRES_URI").unwrap(),
    });
    c.bind::<DatabasePostgresAdapter, dyn DatabasePort>();
    // bind_many: não sobrescreve quem já registrou PingablePort antes — acumula.
    c.bind_many::<DatabasePostgresAdapter, dyn PingablePort>();
}
```

> src/infra/database/mongodb/mod.rs (`init` ganha 1 linha, mesmo padrão)
```rs
pub fn init(c: &Container) {
    c.register_instance(DatabaseMongodbConfig {
        uri: std::env::var("DATABASE_MONGODB_URI").unwrap(),
    });
    c.bind::<DatabaseMongodbAdapter, dyn DatabasePort>();
    c.bind_many::<DatabaseMongodbAdapter, dyn PingablePort>();
}
```

> src/infra/healthcheck.rs
```rs
use std::sync::Arc;
use rudi::Container;
use crate::domain::port::PingablePort;

// #[inject]: sem argumento no call site, macro resolve rudi::container() sozinha.
pub async fn run(#[container] c: &Container) -> Result<(), Error> {
    // resolve_all: TODOS os bind_many registrados pra essa porta, não só o último.
    let pingables = c.resolve_all::<Arc<dyn PingablePort>>().await?;
    for p in pingables {
        p.ping().await?;
    }
    Ok(())
}
```

> src/main.rs (adição ao fluxo já existente)
```rs
// depois de infra::init(&c1) — só postgres bindou PingablePort nesse container,
// já que só um DATABASE_PROVIDER roda por vez (mongodb não chega a inicializar).
healthcheck::run().await.unwrap();
```

### Regra nova (`bind_many`/`resolve_all`)
- `bind` continua "última vence" — semântica intocada, usada quando só 1 impl concreta pode existir por vez (seleção por env).
- `bind_many::<Impl, dyn Port>()` **acumula** em vez de sobrescrever — cada chamada adiciona mais uma implementação à lista da porta.
- `resolve_all::<Arc<dyn Port>>()` retorna todas as implementações acumuladas via `bind_many` pra aquela porta (vazio se nenhuma, não é erro — diferente de `resolve` normal, que erra em tipo não registrado).
- `bind_many` e `bind` são independentes: a mesma `Impl` pode aparecer nos dois (uma porta "seleção única" e outra porta "grupo") sem conflito, como no exemplo acima (`DatabasePort` via `bind`, `PingablePort` via `bind_many`).
