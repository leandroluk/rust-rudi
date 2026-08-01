/// Modo de registro de um [`DebugEntry`] — ver [`Container::debug_entries`](crate::Container::debug_entries).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugKind {
    Instance,
    Transient,
    Singleton,
    /// Registrado via `bind_many` — pode haver vários `DebugEntry` com o mesmo
    /// `type_name`/`name`, 1 por implementação acumulada.
    Many,
}

/// 1 entrada registrada no container — ver [`Container::debug_entries`](crate::Container::debug_entries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugEntry {
    pub type_name: &'static str,
    pub name: Option<String>,
    pub kind: DebugKind,
}

/// Identidade de um tipo resolvido — usado em [`Container::debug_edges`](crate::Container::debug_edges)
/// (não carrega `kind`; uma aresta é sobre "quem depende de quem", não sobre como
/// o dependente foi registrado).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DebugNode {
    pub type_name: &'static str,
    pub name: Option<String>,
}
