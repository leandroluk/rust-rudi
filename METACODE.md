# hipótese estrutural para injeção de dependência em rust

- hipótese estrutural pra uma biblioteca de injeção de dependência em rust chamada "rudi".
- type-safe, usa macros + attributes pra registro via instância, factory, singleton ou bind de porta (trait) — sem boilerplate de wiring manual.
- `Container` é ambiente/global dentro da lib (1 só por processo) — ninguém instancia nem repassa na mão; `rudi::container()` acha ele sozinho, ou `#[inject]` remove até essa chamada da assinatura.
- 3 portas de exemplo: `LoggerPort` (1 provider: slog), `DatabasePort` (2 providers: postgres e mongodb, seleção por env) e `PingablePort` (healthcheck: resolve **todos** os adapters que sabem pingar, não só 1).

> arquitetura do projeto
```
src/
|- domain/
  |- port/
    |- database.rs
    |- logger.rs
    |- pingable.rs
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
  |- healthcheck.rs
  |- mod.rs
|- usecase/
  |- user_usecase.rs
  |- healthcheck_usecase.rs
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

> src/domain/port/pingable.rs
```rs
pub trait PingablePort: Send + Sync {
    async fn ping(&self) -> Result<(), Error>;
}
```

> src/domain/port/mod.rs
```rs
pub mod database;
pub mod logger;
pub mod pingable;

pub use database::DatabasePort;
pub use logger::LoggerPort;
pub use pingable::PingablePort;
```

> src/domain/mod.rs
```rs
pub mod port;
pub use port::{DatabasePort, LoggerPort, PingablePort};
```

> src/infra/logger/slog/config.rs
```rs
use garde::Validate;

// sem #[injectable] aqui: config não tem constructor custom nem se auto-resolve,
// é registrada como instância crua em `init()` (register_instance).
#[derive(Clone, Validate)]
pub struct LoggerSlogConfig {
    #[garde(pattern(r"^(off|crit|error|warn|info|debug|trace)$"))]
    pub level: String,
}
```

> src/infra/logger/slog/adapter.rs
```rs
use std::collections::HashMap;
use std::sync::Arc;
use rudi::injectable;
use slog::{Drain, LevelFilter, Logger};

use crate::domain::port::LoggerPort;
use crate::infra::logger::slog::config::LoggerSlogConfig;

pub struct LoggerSlogAdapter {
    level_map: HashMap<String, LevelFilter>,
    logger: Logger,
    pub config: Arc<LoggerSlogConfig>,
}

// #[injectable]: vai no bloco `impl` inteiro (rust não tem "constructor" de
// linguagem — a macro acha o construtor pela FORMA: fn sem `self`, todo
// parâmetro marcado #[inject]/#[inject_all]/#[container]. Nome não importa).
#[injectable(dyn LoggerPort)]
impl LoggerSlogAdapter {
    // #[inject]: resolve `LoggerSlogConfig` do container pelo próprio tipo.
    // Fica 100% síncrona — o async de resolver fica escondido no
    // `Injectable::build` gerado, nunca aqui.
    fn build(#[inject] config: Arc<LoggerSlogConfig>) -> Self {
        Self {
            config,
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

use rudi::{inject, Container};

use crate::domain::port::LoggerPort;

// #[inject]: `c` some da assinatura pública, macro resolve o container ambiente
// e injeta como 1ª linha do corpo. Call site vira `slog::init()`, sem argumento.
#[inject]
pub fn init(#[container] c: &Container) {
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
        "slog" => slog::init(),
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
use std::sync::Arc;
use rudi::injectable;

use crate::domain::port::{DatabasePort, PingablePort};
use crate::infra::database::postgres::config::DatabasePostgresConfig;

pub struct DatabasePostgresAdapter {
    config: Arc<DatabasePostgresConfig>,
}

// bind de porta única (DatabasePort) — última vence, seleção por env no mod.rs.
#[injectable(dyn DatabasePort)]
impl DatabasePostgresAdapter {
    fn build(#[inject] config: Arc<DatabasePostgresConfig>) -> Self {
        Self { config }
    }
}

impl DatabasePort for DatabasePostgresAdapter {
    async fn ping(&self) -> Result<(), Error> {
        let _ = &self.config.uri; // sqlx::PgPool::connect(&self.config.uri)...
        Ok(())
    }
}

impl PingablePort for DatabasePostgresAdapter {
    async fn ping(&self) -> Result<(), Error> {
        DatabasePort::ping(self).await // reusa a mesma lógica, porta diferente
    }
}
```

> src/infra/database/postgres/mod.rs
```rs
use rudi::Container;
use crate::domain::port::{DatabasePort, PingablePort};

pub mod adapter;
pub mod config;

pub use adapter::DatabasePostgresAdapter;
pub use config::DatabasePostgresConfig;

pub fn init(c: &Container) {
    c.register_instance(DatabasePostgresConfig {
        uri: std::env::var("DATABASE_POSTGRES_URI").unwrap(),
    });
    c.bind::<DatabasePostgresAdapter, dyn DatabasePort>();
    // bind_many: não sobrescreve quem já registrou PingablePort antes — acumula.
    c.bind_many::<DatabasePostgresAdapter, dyn PingablePort>();
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
use std::sync::Arc;
use rudi::injectable;

use crate::domain::port::{DatabasePort, PingablePort};
use crate::infra::database::mongodb::config::DatabaseMongodbConfig;

pub struct DatabaseMongodbAdapter {
    config: Arc<DatabaseMongodbConfig>,
}

#[injectable(dyn DatabasePort)]
impl DatabaseMongodbAdapter {
    fn build(#[inject] config: Arc<DatabaseMongodbConfig>) -> Self {
        Self { config }
    }
}

impl DatabasePort for DatabaseMongodbAdapter {
    async fn ping(&self) -> Result<(), Error> {
        let _ = &self.config.uri; // mongodb::Client::with_uri_str(&self.config.uri)...
        Ok(())
    }
}

impl PingablePort for DatabaseMongodbAdapter {
    async fn ping(&self) -> Result<(), Error> {
        DatabasePort::ping(self).await
    }
}
```

> src/infra/database/mongodb/mod.rs
```rs
use rudi::Container;
use crate::domain::port::{DatabasePort, PingablePort};

pub mod adapter;
pub mod config;

pub use adapter::DatabaseMongodbAdapter;
pub use config::DatabaseMongodbConfig;

pub fn init(c: &Container) {
    c.register_instance(DatabaseMongodbConfig {
        uri: std::env::var("DATABASE_MONGODB_URI").unwrap(),
    });
    c.bind::<DatabaseMongodbAdapter, dyn DatabasePort>();
    c.bind_many::<DatabaseMongodbAdapter, dyn PingablePort>();
}
```

> src/infra/database/mod.rs
```rs
pub mod mongodb;
pub mod postgres;

pub use mongodb::{DatabaseMongodbAdapter, DatabaseMongodbConfig};
pub use postgres::{DatabasePostgresAdapter, DatabasePostgresConfig};

// seleção por env — bind sempre acaba na mesma `dyn DatabasePort`, quem resolve
// depois nem sabe qual dos dois foi escolhido. bind_many (PingablePort) já
// acontece dentro de cada `init` acima, sem esse `match` decidir nada ali.
pub fn init(c: &Container) {
    let provider = std::env::var("DATABASE_PROVIDER").unwrap();
    match provider.as_str() {
        "postgres" => postgres::init(c),
        "mongodb" => mongodb::init(c),
        other => panic!("Invalid database provider: {other}"),
    }
}
```

> src/infra/healthcheck.rs
```rs
use std::sync::Arc;
use rudi::{inject, Container};
use crate::domain::port::PingablePort;

// #[inject]: call site vira `healthcheck::run()`, sem argumento.
#[inject]
pub async fn run(#[container] c: &Container) -> Result<(), Error> {
    // resolve_all: TODOS os bind_many registrados pra essa porta, não só 1.
    let pingables = c.resolve_all::<Arc<dyn PingablePort>>().await?;
    for p in pingables {
        p.ping().await?;
    }
    Ok(())
}
```

> src/infra/mod.rs
```rs
pub mod database;
pub mod healthcheck;
pub mod logger;

pub use database::{DatabaseMongodbAdapter, DatabaseMongodbConfig, DatabasePostgresAdapter, DatabasePostgresConfig};
pub use logger::{LoggerSlogAdapter, LoggerSlogConfig};

use rudi::Container;

pub fn init(c: &Container) {
    database::init(c);
    logger::init(c);
}
```

> src/usecase/user_usecase.rs
```rs
use std::sync::Arc;
use rudi::injectable;

use crate::domain::port::{PostPort, UserPort};

pub struct UserUsecase {
    user_repo: Arc<dyn UserPort>,
    post_repo: Arc<dyn PostPort>,
}

#[injectable]
impl UserUsecase {
    // qualquer nome serve — `new`, `build`, `create`. Continua síncrona,
    // testável direto (`UserUsecase::new(mock_a, mock_b)`, sem `.await`).
    fn new(
        #[inject] user_repo: Arc<dyn UserPort>,
        #[inject] post_repo: Arc<dyn PostPort>,
    ) -> Self { Self { user_repo, post_repo } }

    pub async fn execute(&self) -> Result<(), Error> { Ok(()) }
}
```

> src/usecase/healthcheck_usecase.rs
```rs
use std::sync::Arc;
use rudi::injectable;

use crate::domain::port::PingablePort;

pub struct HealthcheckUsecase {
    pingables: Vec<Arc<dyn PingablePort>>,
}

#[injectable]
impl HealthcheckUsecase {
    // #[inject_all]: mesma ideia do #[inject], mas via resolve_all — todo mundo
    // que fez bind_many pra essa porta, não só 1.
    fn new(#[inject_all] pingables: Vec<Arc<dyn PingablePort>>) -> Self {
        Self { pingables }
    }

    pub async fn execute(&self) -> Result<(), Error> {
        for p in &self.pingables {
            p.ping().await?;
        }
        Ok(())
    }
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
    c1.resolve::<Arc<dyn LoggerPort>>().await.unwrap().info("Hello World!");
    c1.resolve::<Arc<dyn DatabasePort>>().await.unwrap().ping().await.unwrap();
    infra::healthcheck::run().await.unwrap();

    std::env::set_var("LOGGER_PROVIDER", "slog");
    std::env::set_var("DATABASE_PROVIDER", "mongodb");
    let c2 = rudi::container();
    infra::init(&c2);
    c2.resolve::<Arc<dyn LoggerPort>>().await.unwrap().info("Hello World 2!");
    c2.resolve::<Arc<dyn DatabasePort>>().await.unwrap().ping().await.unwrap();
    infra::healthcheck::run().await.unwrap();
}
```

## Como se usa (superfície, sem implementação)

- `rudi::container()` — pega a instância ambiente (lazy, 1 por processo).
- `Container::new()` — container local, sempre usado em teste (nunca o ambiente).
- `c.register_instance(valor)` — registra algo já pronto (ex: config lido/validado fora da lib).
- `c.register_transient::<Tipo>(builder)` — builder roda de novo a cada `resolve`, sem cache.
- `c.register_singleton::<Tipo>(builder)` — builder cacheado, 1ª resolução roda, demais retornam a mesma instância (double-init safe).
- `c.register_singleton_injectable::<Tipo>()` — igual singleton, mas usando o `Injectable` gerado por `#[injectable]`/`#[derive(Injectable)]` em vez de passar closure na mão.
- `c.bind_with::<dyn Port>(builder)` / `c.bind::<Impl, dyn Port>()` — registra `Impl` resolvível pela porta (trait), não pelo tipo concreto. Última chamada vence — resolver por trait nunca sabe qual impl concreta está por trás.
- `c.bind_many::<Impl, dyn Port>()` — acumula (não sobrescreve) implementações da mesma porta.
- `c.resolve::<Tipo>()` / `c.resolve_named::<Tipo>(nome)` — busca o que foi registrado; sempre `async`, sempre retorna `Arc<Tipo>`; erro tipado (não panic) se não registrado.
- `c.resolve_all::<Arc<dyn Port>>()` — todas as implementações acumuladas via `bind_many` pra essa porta; vazio (não erro) se nenhuma.
- `#[injectable]` no bloco `impl` — acha o construtor pela forma (sem `self`, todo parâmetro marcado), gera `impl Injectable`. `#[injectable(dyn Port)]` liga a porta; sem argumento, resolve por tipo concreto.
- `#[inject]`/`#[inject_all]`/`#[container]` nos parâmetros do construtor — resolve por tipo (`Arc<T>`), por tipo em lote (`Vec<Arc<T>>` via `resolve_all`), ou entrega o `Container` cru, respectivamente.
- `#[inject]` (o outro, em cima de qualquer fn) — tira o parâmetro marcado `#[container]` da assinatura pública; call site chama sem passar nada.
- `#[derive(Injectable)]` no struct — sem constructor custom, cada campo (`Arc<T>`) é resolvido individualmente do container.
- `rudi::testing::with_container(|c| async { ... })` — container isolado por chamada, pra teste.

### Regras fechadas

- Biblioteca nunca chama `std::env` — quem lê env é sempre o `init()` do consumidor.
- Falha de config em `init()` é fail-fast do lado do consumidor, a lib não decide isso.
- `Container` é global/ambiente — consumidor não instancia na mão, usa `rudi::container()` (ou nem isso, com `#[inject]`); `Container::new()` só existe pra isolar teste.
- Múltiplos providers pra mesma porta com seleção única (ex: `DatabasePort` postgres/mongodb) seguem o padrão do logger: cada provider tem seu `init()`, o `mod.rs` do nível da porta decide qual chamar via env, `bind` sempre contra a `dyn Port`.
- Múltiplas implementações da mesma porta resolvidas **juntas** (healthcheck) usam `bind_many`/`resolve_all` — storage separado de `bind`, mesma `Impl` pode estar nos dois pra portas diferentes sem conflito.
- `resolve` sempre `async`, sempre `Arc<T>` — mesmo pra construtores síncronos, garante suporte uniforme a builders que precisam de I/O.
- `#[injectable]` fica no `impl` block, nunca na fn isolada — emitir `impl Injectable for Tipo` exige estar fora de qualquer `impl`, e atributo em fn associada só pode virar outra fn associada.
- Rust não tem "constructor" de linguagem — `#[injectable]` acha o construtor pela forma da assinatura (todo parâmetro marcado), nunca pelo nome.
