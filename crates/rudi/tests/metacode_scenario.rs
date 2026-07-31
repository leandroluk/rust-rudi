//! Reproduz o cenário do METACODE.md (logger + database com 2 providers,
//! 2 containers simulando "env" diferente) só com a API pública do container
//! (sem macros — builders chamados manualmente como `#[injectable]` chamaria
//! a partir da M2). Container é sempre local (`Container::new()`), nunca o
//! global, seguindo a nota de isolamento de testes em `.specs/codebase/TESTING.md`.

use std::sync::Arc;

use rudi::Container;

trait LoggerPort: Send + Sync {
    fn info(&self, message: &str) -> String;
}

trait DatabasePort: Send + Sync {
    fn ping(&self) -> &'static str;
}

struct LoggerSlogAdapter;

impl LoggerPort for LoggerSlogAdapter {
    fn info(&self, message: &str) -> String {
        format!("[slog] {message}")
    }
}

#[derive(Clone)]
struct DatabasePostgresConfig {
    uri: String,
}

struct DatabasePostgresAdapter {
    config: DatabasePostgresConfig,
}

impl DatabasePort for DatabasePostgresAdapter {
    fn ping(&self) -> &'static str {
        "postgres"
    }
}

#[derive(Clone)]
struct DatabaseMongodbConfig {
    uri: String,
}

struct DatabaseMongodbAdapter {
    config: DatabaseMongodbConfig,
}

impl DatabasePort for DatabaseMongodbAdapter {
    fn ping(&self) -> &'static str {
        "mongodb"
    }
}

// Equivalente a src/infra/logger/slog/mod.rs::init(c) do METACODE.
fn init_logger(c: &Container) {
    c.bind_with::<dyn LoggerPort, _, _, std::convert::Infallible>(|_c| async {
        Ok(Arc::new(LoggerSlogAdapter) as Arc<dyn LoggerPort>)
    });
}

// Equivalente a src/infra/database/postgres/mod.rs::init(c).
fn init_database_postgres(c: &Container, uri: &str) {
    c.register_instance(DatabasePostgresConfig {
        uri: uri.to_string(),
    });
    c.bind_with::<dyn DatabasePort, _, _, std::convert::Infallible>(|c| async move {
        let config = c.resolve::<DatabasePostgresConfig>().await.unwrap();
        Ok(Arc::new(DatabasePostgresAdapter {
            config: (*config).clone(),
        }) as Arc<dyn DatabasePort>)
    });
}

// Equivalente a src/infra/database/mongodb/mod.rs::init(c).
fn init_database_mongodb(c: &Container, uri: &str) {
    c.register_instance(DatabaseMongodbConfig {
        uri: uri.to_string(),
    });
    c.bind_with::<dyn DatabasePort, _, _, std::convert::Infallible>(|c| async move {
        let config = c.resolve::<DatabaseMongodbConfig>().await.unwrap();
        Ok(Arc::new(DatabaseMongodbAdapter {
            config: (*config).clone(),
        }) as Arc<dyn DatabasePort>)
    });
}

// Equivalente a src/infra/database/mod.rs::init(c) — seleção por "env" (aqui, parâmetro
// direto, já que a lib nunca lê std::env; quem decidiria via env seria o consumidor).
fn init_database(c: &Container, provider: &str) {
    match provider {
        "postgres" => init_database_postgres(c, "postgres://localhost/db"),
        "mongodb" => init_database_mongodb(c, "mongodb://localhost/db"),
        other => panic!("Invalid database provider: {other}"),
    }
}

#[tokio::test]
async fn metacode_scenario_postgres_container() {
    let c1 = Container::new();
    init_logger(&c1);
    init_database(&c1, "postgres");

    let logger = c1.resolve::<Arc<dyn LoggerPort>>().await.unwrap();
    assert_eq!(logger.info("Hello World!"), "[slog] Hello World!");

    let db = c1.resolve::<Arc<dyn DatabasePort>>().await.unwrap();
    assert_eq!(db.ping(), "postgres");
}

#[tokio::test]
async fn metacode_scenario_mongodb_container() {
    let c2 = Container::new();
    init_logger(&c2);
    init_database(&c2, "mongodb");

    let logger = c2.resolve::<Arc<dyn LoggerPort>>().await.unwrap();
    assert_eq!(logger.info("Hello World 2!"), "[slog] Hello World 2!");

    let db = c2.resolve::<Arc<dyn DatabasePort>>().await.unwrap();
    assert_eq!(db.ping(), "mongodb");
}

#[tokio::test]
async fn two_containers_with_different_providers_do_not_interfere() {
    let c1 = Container::new();
    init_database(&c1, "postgres");

    let c2 = Container::new();
    init_database(&c2, "mongodb");

    let db1 = c1.resolve::<Arc<dyn DatabasePort>>().await.unwrap();
    let db2 = c2.resolve::<Arc<dyn DatabasePort>>().await.unwrap();

    assert_eq!(db1.ping(), "postgres");
    assert_eq!(db2.ping(), "mongodb");
}
