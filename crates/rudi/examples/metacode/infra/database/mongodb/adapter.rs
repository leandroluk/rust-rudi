use rudi::{injectable, Container};

use crate::domain::port::database::DatabaseError;
use crate::domain::port::DatabasePort;
use crate::infra::database::mongodb::config::DatabaseMongodbConfig;

pub struct DatabaseMongodbAdapter {
    config: DatabaseMongodbConfig,
}

#[injectable(dyn DatabasePort)]
impl DatabaseMongodbAdapter {
    async fn build(c: &Container) -> Self {
        let config = c.resolve::<DatabaseMongodbConfig>().await.unwrap();
        Self {
            config: (*config).clone(),
        }
    }
}

impl DatabasePort for DatabaseMongodbAdapter {
    fn ping(&self) -> Result<(), DatabaseError> {
        let _ = &self.config.uri; // mongodb::Client::with_uri_str(&self.config.uri)...
        Ok(())
    }
}
