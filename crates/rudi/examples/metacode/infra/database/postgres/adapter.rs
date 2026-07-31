use rudi::{injectable, Container};

use crate::domain::port::database::DatabaseError;
use crate::domain::port::DatabasePort;
use crate::infra::database::postgres::config::DatabasePostgresConfig;

pub struct DatabasePostgresAdapter {
    config: DatabasePostgresConfig,
}

#[injectable(dyn DatabasePort)]
impl DatabasePostgresAdapter {
    async fn build(c: &Container) -> Self {
        let config = c.resolve::<DatabasePostgresConfig>().await.unwrap();
        Self {
            config: (*config).clone(),
        }
    }
}

impl DatabasePort for DatabasePostgresAdapter {
    fn ping(&self) -> Result<(), DatabaseError> {
        let _ = &self.config.uri; // sqlx::PgPool::connect(&self.config.uri)...
        Ok(())
    }
}
