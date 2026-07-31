pub mod slog;

pub use slog::{LoggerSlogAdapter, LoggerSlogConfig};

use rudi::Container;

pub fn init(c: &Container, provider: &str) {
    match provider {
        "slog" => self::slog::init(c, "info"),
        other => panic!("Invalid logger provider: {other}"),
    }
}
