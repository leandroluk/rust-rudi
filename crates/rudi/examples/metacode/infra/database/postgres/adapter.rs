use std::sync::Arc;

use rudi::injectable;

use crate::domain::port::database::DatabaseError;
use crate::domain::port::DatabasePort;
use crate::infra::database::postgres::config::DatabasePostgresConfig;

pub struct DatabasePostgresAdapter {
    config: Arc<DatabasePostgresConfig>,
}

#[injectable(dyn DatabasePort)]
impl DatabasePostgresAdapter {
    fn build(#[inject] config: Arc<DatabasePostgresConfig>) -> Self {
        Self { config }
    }
}

impl DatabasePort for DatabasePostgresAdapter {
    fn ping(&self) -> Result<(), DatabaseError> {
        let _ = &self.config.uri; // sqlx::PgPool::connect(&self.config.uri)...
        Ok(())
    }
}
