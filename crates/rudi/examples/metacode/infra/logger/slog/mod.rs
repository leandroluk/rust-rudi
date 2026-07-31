pub mod adapter;
pub mod config;

pub use adapter::LoggerSlogAdapter;
pub use config::LoggerSlogConfig;

use rudi::Container;

use crate::domain::port::LoggerPort;

pub fn init(c: &Container, level: &str) {
    c.register_instance(LoggerSlogConfig {
        level: level.to_string(),
    });
    c.bind::<LoggerSlogAdapter, dyn LoggerPort>();
}
