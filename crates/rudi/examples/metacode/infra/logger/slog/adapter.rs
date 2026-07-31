use std::collections::HashMap;

use rudi::{injectable, Container};

use crate::domain::port::LoggerPort;
use crate::infra::logger::slog::config::LoggerSlogConfig;

pub struct LoggerSlogAdapter {
    level_map: HashMap<String, u8>,
    #[allow(dead_code)]
    pub config: LoggerSlogConfig,
}

// #[injectable]: marca o `impl` como gerador de `impl Injectable` pro tipo (usado por
// `bind`/`register_singleton_injectable`). Vai no bloco `impl` inteiro, não na fn —
// ver .specs/features/di-macros/design.md pra explicação da restrição de linguagem.
#[injectable(dyn LoggerPort)]
impl LoggerSlogAdapter {
    // async (não sync como no METACODE.md) porque `resolve` do core é sempre async —
    // ver .specs/features/core-container/context.md, decisão de design fechada na M1.
    async fn build(c: &Container) -> Self {
        let config = c.resolve::<LoggerSlogConfig>().await.unwrap();
        Self {
            config: (*config).clone(),
            level_map: HashMap::from([
                ("off".to_string(), 0),
                ("info".to_string(), 1),
                ("warn".to_string(), 2),
                ("error".to_string(), 3),
            ]),
        }
    }

    fn dispatch(&self, level: &str, message: &str) {
        if self.level_map.contains_key(level) {
            println!("[slog:{level}] {message}");
        }
    }
}

impl LoggerPort for LoggerSlogAdapter {
    fn log(&self, level: &str, message: &str) {
        self.dispatch(level, message);
    }
    fn info(&self, message: &str) {
        self.dispatch("info", message);
    }
    fn warn(&self, message: &str) {
        self.dispatch("warn", message);
    }
    fn error(&self, message: &str) {
        self.dispatch("error", message);
    }
}
