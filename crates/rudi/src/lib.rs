mod container;
mod error;
mod injectable;
pub mod testing;

pub use container::Container;
pub use error::RudiError;
pub use injectable::Injectable;
// Trait `Injectable` (namespace de tipo) e `#[derive(Injectable)]` (namespace de macro)
// coexistem sob o mesmo identificador — igual `Clone`/`#[derive(Clone)]` no std.
pub use rudi_macros::{inject, injectable, Injectable};

use std::sync::OnceLock;

static GLOBAL: OnceLock<Container> = OnceLock::new();

/// Container ambiente — 1 instância lazy por processo. Ninguém instancia nem repassa
/// na mão; consumidores chamam `rudi::container()` sempre que precisam do handle.
pub fn container() -> Container {
    GLOBAL.get_or_init(Container::new).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn container_returns_same_global_state_across_calls() {
        container().register_instance(42u32);

        let resolved = container().resolve::<u32>().await.unwrap();
        assert_eq!(*resolved, 42);
    }
}
