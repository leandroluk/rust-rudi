pub mod adapter;
pub mod config;

pub use adapter::DatabaseMongodbAdapter;
pub use config::DatabaseMongodbConfig;

use rudi::Container;

use crate::domain::port::DatabasePort;

pub fn init(c: &Container, uri: &str) {
    c.register_instance(DatabaseMongodbConfig {
        uri: uri.to_string(),
    });
    c.bind::<DatabaseMongodbAdapter, dyn DatabasePort>();
}
