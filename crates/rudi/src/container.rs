use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use tokio::sync::OnceCell;

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

// SPEC_DEVIATION: design.md especifica tokio::sync::RwLock; usamos std::sync::RwLock aqui.
// Reason: a tabela de entradas só é acessada de forma síncrona (lookup/insert), nunca
// segurada através de um .await — só o OnceCell do singleton precisa ser async-aware.
// std::sync::RwLock evita overhead de lock async onde não é necessário.
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

    fn insert_entry(&self, key: Key, entry: Entry) {
        let mut entries = self.inner.entries.write().unwrap();
        entries.insert(key, entry);
    }

    /// Registra um valor já construído, resolvível sem nome.
    pub fn register_instance<T: Send + Sync + 'static>(&self, value: T) {
        let key = Key::new(TypeId::of::<T>(), None);
        self.insert_entry(key, Entry::Instance(Arc::new(value)));
    }

    /// Registra um valor já construído sob um nome, coexistindo com outras instâncias do mesmo tipo.
    pub fn register_instance_named<T: Send + Sync + 'static>(&self, name: impl Into<String>, value: T) {
        let name = name.into();
        let key = Key::new(TypeId::of::<T>(), Some(&name));
        self.insert_entry(key, Entry::Instance(Arc::new(value)));
    }

    /// Registra um builder que roda de novo a cada `resolve` (sem cache).
    pub fn register_transient<T, F, Fut, E>(&self, builder: F)
    where
        T: Send + Sync + 'static,
        F: Fn(Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        let key = Key::new(TypeId::of::<T>(), None);
        self.insert_entry(key, Entry::Transient(wrap_builder::<T, F, Fut, E>(builder)));
    }

    /// Variante nomeada de [`Container::register_transient`].
    pub fn register_transient_named<T, F, Fut, E>(&self, name: impl Into<String>, builder: F)
    where
        T: Send + Sync + 'static,
        F: Fn(Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        let name = name.into();
        let key = Key::new(TypeId::of::<T>(), Some(&name));
        self.insert_entry(key, Entry::Transient(wrap_builder::<T, F, Fut, E>(builder)));
    }

    /// Registra um builder cacheado — 1ª resolução executa, demais retornam a mesma instância.
    pub fn register_singleton<T, F, Fut, E>(&self, builder: F)
    where
        T: Send + Sync + 'static,
        F: Fn(Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        let key = Key::new(TypeId::of::<T>(), None);
        self.insert_entry(key, singleton_entry::<T, F, Fut, E>(builder));
    }

    /// Variante nomeada de [`Container::register_singleton`].
    pub fn register_singleton_named<T, F, Fut, E>(&self, name: impl Into<String>, builder: F)
    where
        T: Send + Sync + 'static,
        F: Fn(Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        let name = name.into();
        let key = Key::new(TypeId::of::<T>(), Some(&name));
        self.insert_entry(key, singleton_entry::<T, F, Fut, E>(builder));
    }

    /// Resolve `T`, sem nome.
    pub async fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, RudiError> {
        self.resolve_inner::<T>(None).await
    }

    /// Resolve `T` registrado sob `name`.
    pub async fn resolve_named<T: Send + Sync + 'static>(&self, name: &str) -> Result<Arc<T>, RudiError> {
        self.resolve_inner::<T>(Some(name)).await
    }

    async fn resolve_inner<T: Send + Sync + 'static>(&self, name: Option<&str>) -> Result<Arc<T>, RudiError> {
        let type_name = std::any::type_name::<T>();
        let any = self.resolve_any(TypeId::of::<T>(), name, type_name).await?;
        any.downcast::<T>()
            .map_err(|_| RudiError::DowncastFailed { type_name })
    }

    pub(crate) async fn resolve_any(
        &self,
        type_id: TypeId,
        name: Option<&str>,
        type_name: &'static str,
    ) -> Result<AnyArc, RudiError> {
        enum Action {
            Instance(AnyArc),
            Transient(BoxedFactory),
            Singleton(BoxedFactory, Arc<OnceCell<AnyArc>>),
        }

        let key = Key::new(type_id, name);

        let action = {
            let entries = self.inner.entries.read().unwrap();
            match entries.get(&key) {
                Some(Entry::Instance(value)) => Action::Instance(value.clone()),
                Some(Entry::Transient(factory)) => Action::Transient(factory.clone()),
                Some(Entry::Singleton { factory, cell }) => {
                    Action::Singleton(factory.clone(), cell.clone())
                }
                None => {
                    return Err(RudiError::NotFound {
                        type_name,
                        name: name.map(str::to_string),
                    });
                }
            }
        };

        match action {
            Action::Instance(value) => Ok(value),
            Action::Transient(factory) => (factory.as_ref())(self.clone()).await,
            Action::Singleton(factory, cell) => cell
                .get_or_try_init(|| (factory.as_ref())(self.clone()))
                .await
                .cloned(),
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

fn singleton_entry<T, F, Fut, E>(builder: F) -> Entry
where
    T: Send + Sync + 'static,
    F: Fn(Container) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    Entry::Singleton {
        factory: wrap_builder::<T, F, Fut, E>(builder),
        cell: Arc::new(OnceCell::new()),
    }
}

fn wrap_builder<T, F, Fut, E>(builder: F) -> BoxedFactory
where
    T: Send + Sync + 'static,
    F: Fn(Container) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    let type_name = std::any::type_name::<T>();
    Arc::new(move |c: Container| {
        let fut = builder(c);
        Box::pin(async move {
            fut.await
                .map(|value| Arc::new(value) as AnyArc)
                .map_err(|source| RudiError::BuildFailed {
                    type_name,
                    source: Box::new(source),
                })
        }) as BoxedFuture
    })
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
