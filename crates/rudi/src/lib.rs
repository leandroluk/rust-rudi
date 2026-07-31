mod container;
mod error;
mod injectable;

pub use container::Container;
pub use error::RudiError;
pub use injectable::Injectable;

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
