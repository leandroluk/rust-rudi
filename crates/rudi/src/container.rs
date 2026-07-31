use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{OnceCell, RwLock};

use crate::error::RudiError;

pub(crate) type AnyArc = Arc<dyn Any + Send + Sync>;
pub(crate) type BoxedFuture = Pin<Box<dyn Future<Output = Result<AnyArc, RudiError>> + Send>>;
pub(crate) type BoxedFactory = Arc<dyn Fn(Container) -> BoxedFuture + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    type_id: TypeId,
    name: Option<Box<str>>,
}

impl Key {
    pub(crate) fn new(type_id: TypeId, name: Option<&str>) -> Self {
        Self {
            type_id,
            name: name.map(Box::from),
        }
    }
}

pub(crate) enum Entry {
    Instance(AnyArc),
    Transient(BoxedFactory),
    Singleton {
        factory: BoxedFactory,
        cell: Arc<OnceCell<AnyArc>>,
    },
}

pub(crate) struct Inner {
    pub(crate) entries: RwLock<HashMap<Key, Entry>>,
}

impl Inner {
    fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }
}

/// Handle de container de injeção de dependência. Clone é barato (`Arc` por dentro).
#[derive(Clone)]
pub struct Container {
    pub(crate) inner: Arc<Inner>,
}

impl Container {
    /// Cria um container local independente (não é o container global — ver `rudi::container()`).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner::new()),
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_with_same_type_and_no_name_are_equal() {
        let a = Key::new(TypeId::of::<u32>(), None);
        let b = Key::new(TypeId::of::<u32>(), None);
        assert_eq!(a, b);
    }

    #[test]
    fn keys_with_same_type_and_same_name_are_equal() {
        let a = Key::new(TypeId::of::<u32>(), Some("primary"));
        let b = Key::new(TypeId::of::<u32>(), Some("primary"));
        assert_eq!(a, b);
    }

    #[test]
    fn keys_with_same_type_and_different_names_are_not_equal() {
        let a = Key::new(TypeId::of::<u32>(), Some("primary"));
        let b = Key::new(TypeId::of::<u32>(), Some("replica"));
        assert_ne!(a, b);
    }

    #[test]
    fn keys_with_same_type_named_vs_unnamed_are_not_equal() {
        let a = Key::new(TypeId::of::<u32>(), None);
        let b = Key::new(TypeId::of::<u32>(), Some("primary"));
        assert_ne!(a, b);
    }

    #[test]
    fn keys_with_different_types_are_not_equal() {
        let a = Key::new(TypeId::of::<u32>(), None);
        let b = Key::new(TypeId::of::<u64>(), None);
        assert_ne!(a, b);
    }
}
