use std::sync::Arc;

use rudi::injectable;

use crate::domain::port::database::DatabaseError;
use crate::domain::port::DatabasePort;
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
    fn ping(&self) -> Result<(), DatabaseError> {
        let _ = &self.config.uri; // mongodb::Client::with_uri_str(&self.config.uri)...
        Ok(())
    }
}
