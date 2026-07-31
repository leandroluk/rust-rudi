pub mod port;

// Re-export de conveniência (simetria com METACODE.md) — não usado dentro deste
// binário de exemplo, só faria diferença pra quem consumisse este módulo como lib.
#[allow(unused_imports)]
pub use port::{DatabasePort, LoggerPort};
