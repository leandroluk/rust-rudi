use std::collections::HashMap;
use std::sync::Arc;

use rudi::injectable;

use crate::domain::port::LoggerPort;
use crate::infra::logger::slog::config::LoggerSlogConfig;

pub struct LoggerSlogAdapter {
    level_map: HashMap<String, u8>,
    #[allow(dead_code)]
    pub config: Arc<LoggerSlogConfig>,
}

// #[injectable]: marca o `impl` como gerador de `impl Injectable` pro tipo (usado por
// `bind`/`register_singleton_injectable`). Vai no bloco `impl` inteiro, não na fn —
// ver .specs/features/di-macros/design.md pra explicação da restrição de linguagem.
//
// #[inject] no parâmetro: resolve `LoggerSlogConfig` do container pelo próprio tipo,
// sem chamada manual a `c.resolve()` — construtor fica síncrono, todo o async fica
// escondido dentro do `Injectable::build` gerado (ver METACODE.md, regra de detecção
// do construtor).
#[injectable(dyn LoggerPort)]
impl LoggerSlogAdapter {
    fn build(#[inject] config: Arc<LoggerSlogConfig>) -> Self {
        Self {
            config,
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
