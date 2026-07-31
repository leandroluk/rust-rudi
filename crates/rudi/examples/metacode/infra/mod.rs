pub mod database;
pub mod logger;

// Re-exports de conveniência (simetria com METACODE.md) — não usados dentro deste
// binário de exemplo, só fariam diferença pra quem consumisse este módulo como lib.
#[allow(unused_imports)]
pub use database::{
    DatabaseMongodbAdapter, DatabaseMongodbConfig, DatabasePostgresAdapter, DatabasePostgresConfig,
};
#[allow(unused_imports)]
pub use logger::{LoggerSlogAdapter, LoggerSlogConfig};

use rudi::Container;

pub fn init(c: &Container, database_provider: &str, logger_provider: &str) {
    database::init(c, database_provider);
    logger::init(c, logger_provider);
}
