pub mod adapter;
pub mod config;

pub use adapter::DatabasePostgresAdapter;
pub use config::DatabasePostgresConfig;

use rudi::Container;

use crate::domain::port::DatabasePort;

pub fn init(c: &Container, uri: &str) {
    c.register_instance(DatabasePostgresConfig {
        uri: uri.to_string(),
    });
    c.bind::<DatabasePostgresAdapter, dyn DatabasePort>();
}
