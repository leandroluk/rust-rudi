// Sem Injectable aqui: config não tem constructor custom nem se auto-resolve,
// ela é registrada como instância crua em `init()` (register_instance).
#[derive(Clone, Debug)]
pub struct LoggerSlogConfig {
    #[allow(dead_code)]
    pub level: String,
}
