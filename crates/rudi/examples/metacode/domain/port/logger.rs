// Superfície completa da porta (simetria com METACODE.md) — o exemplo só chama
// `.info()`, deixando warn/error/log intactos pra ilustrar a interface real.
#[allow(dead_code)]
pub trait LoggerPort: Send + Sync {
    fn log(&self, level: &str, message: &str);
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
}
