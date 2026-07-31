pub mod mongodb;
pub mod postgres;

pub use mongodb::{DatabaseMongodbAdapter, DatabaseMongodbConfig};
pub use postgres::{DatabasePostgresAdapter, DatabasePostgresConfig};

use rudi::Container;

pub fn init(c: &Container, provider: &str) {
    match provider {
        "postgres" => self::postgres::init(c, "postgres://localhost/db"),
        "mongodb" => self::mongodb::init(c, "mongodb://localhost/db"),
        other => panic!("Invalid database provider: {other}"),
    }
}
