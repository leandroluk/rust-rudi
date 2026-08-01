use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use tokio::sync::OnceCell;

use crate::error::RudiError;
use crate::injectable::Injectable;

// Pilha de resolução da cadeia lógica atual — escopo por `.await` aninhado, não por
// thread (uma task pode trocar de thread entre awaits sob runtime multi-thread,
// então thread-local daria falso negativo/positivo). Estabelecida uma única vez na
// entrada "de fora" de `resolve_any`; chamadas recursivas (builder chamando
// `c.resolve()` de novo) reusam o mesmo escopo via `try_with`.
tokio::task_local! {
    static RESOLUTION_STACK: RefCell<Vec<(TypeId, Option<Box<str>>, &'static str)>>;
}

struct StackPopGuard;

impl Drop for StackPopGuard {
    fn drop(&mut self) {
        // Só falha se chamado fora de um escopo ativo, o que não deveria acontecer
        // (o guard só existe enquanto resolve_any_checked, que já está dentro do
        // escopo, está rodando) — `try_with` evita panic mesmo nesse caso hipotético.
        let _ = RESOLUTION_STACK.try_with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

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

/// Slot de `bind_many` — mesma forma de `Entry::Singleton` (builder + cache), mas
/// vive numa lista à parte (`Inner::many`), nunca sobrescrita por outro `bind_many`.
pub(crate) struct ManySlot {
    factory: BoxedFactory,
    cell: Arc<OnceCell<AnyArc>>,
}

pub(crate) type ShutdownHook =
    Arc<dyn Fn(Container) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

// SPEC_DEVIATION: design.md especifica tokio::sync::RwLock; usamos std::sync::RwLock aqui.
// Reason: a tabela de entradas só é acessada de forma síncrona (lookup/insert), nunca
// segurada através de um .await — só o OnceCell do singleton precisa ser async-aware.
// std::sync::RwLock evita overhead de lock async onde não é necessário.
pub(crate) struct Inner {
    pub(crate) entries: RwLock<HashMap<Key, Entry>>,
    // Storage separado de `entries` — `bind_many` nunca sobrescreve/é sobrescrito por
    // `bind`/`bind_with` na mesma chave, mesmo pra Impls que aparecem nos dois.
    pub(crate) many: RwLock<HashMap<Key, Vec<ManySlot>>>,
    pub(crate) shutdown_hooks: RwLock<Vec<ShutdownHook>>,
}

impl Inner {
    fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            many: RwLock::new(HashMap::new()),
            shutdown_hooks: RwLock::new(Vec::new()),
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
    pub fn register_instance_named<T: Send + Sync + 'static>(
        &self,
        name: impl Into<String>,
        value: T,
    ) {
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

    /// Registra `builder` como a implementação de `Port` (trait/porta), resolvível via
    /// `resolve::<Arc<Port>>()`. Se chamado 2x pra mesma `Port`, a 2ª chamada vence
    /// (mesma regra "último registro vence" de `register_instance`).
    ///
    /// `builder` já retorna `Arc<Port>` pronto (o call site faz a coerção
    /// `Arc<Impl> as Arc<dyn Port>`, já que o container não tem como expressar
    /// esse bound genericamente sem a macro `#[injectable]`/`bind` da M2).
    pub fn bind_with<Port, F, Fut, E>(&self, builder: F)
    where
        Port: Send + Sync + 'static + ?Sized,
        F: Fn(Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Arc<Port>, E>> + Send + 'static,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.register_singleton::<Arc<Port>, F, Fut, E>(builder);
    }

    /// Registra `T: Injectable<Port = T>` como singleton, usando o `build` gerado por
    /// `#[injectable]`/`#[derive(Injectable)]` em vez de um closure manual.
    pub fn register_singleton_injectable<T>(&self)
    where
        T: Injectable<Port = T>,
    {
        self.register_singleton::<T, _, _, T::Error>(T::build);
    }

    /// Registra `Impl: Injectable<Port = Port>` resolvível via `resolve::<Arc<Port>>()`.
    /// Se chamado 2x pra mesma `Port`, a 2ª chamada vence (mesma regra de `bind_with`).
    pub fn bind<Impl, Port>(&self)
    where
        Impl: Injectable<Port = Port>,
        Port: Send + Sync + 'static + ?Sized,
    {
        self.register_singleton::<Arc<Port>, _, _, Impl::Error>(|c| async move {
            let built = Arc::new(Impl::build(c).await?);
            Ok(Impl::into_port(built))
        });
    }

    /// Acumula `Impl: Injectable<Port = Port>` na lista de implementações de `Port`,
    /// sem sobrescrever `bind_many`s anteriores pra mesma porta (diferente de `bind`,
    /// que é "última vence"). Ver [`Container::resolve_all`].
    pub fn bind_many<Impl, Port>(&self)
    where
        Impl: Injectable<Port = Port>,
        Port: Send + Sync + 'static + ?Sized,
    {
        let factory = wrap_builder::<Arc<Port>, _, _, Impl::Error>(|c| async move {
            let built = Arc::new(Impl::build(c).await?);
            Ok(Impl::into_port(built))
        });
        let key = Key::new(TypeId::of::<Arc<Port>>(), None);
        let mut many = self.inner.many.write().unwrap();
        many.entry(key).or_default().push(ManySlot {
            factory,
            cell: Arc::new(OnceCell::new()),
        });
    }

    /// Resolve todas as implementações acumuladas via [`Container::bind_many`] pra
    /// `Port`, na ordem em que foram registradas. Vazio (não é erro) se nenhuma
    /// `bind_many` rodou pra essa porta ainda.
    pub async fn resolve_all<T: Send + Sync + 'static>(&self) -> Result<Vec<Arc<T>>, RudiError> {
        let type_name = std::any::type_name::<T>();
        let key = Key::new(TypeId::of::<T>(), None);

        let slots: Vec<(BoxedFactory, Arc<OnceCell<AnyArc>>)> = {
            let many = self.inner.many.read().unwrap();
            match many.get(&key) {
                Some(slots) => slots
                    .iter()
                    .map(|slot| (slot.factory.clone(), slot.cell.clone()))
                    .collect(),
                None => Vec::new(),
            }
        };

        let mut result = Vec::with_capacity(slots.len());
        for (factory, cell) in slots {
            let any = cell
                .get_or_try_init(|| (factory.as_ref())(self.clone()))
                .await?
                .clone();
            let value = any
                .downcast::<T>()
                .map_err(|_| RudiError::DowncastFailed { type_name })?;
            result.push(value);
        }
        Ok(result)
    }

    /// Registra um hook de shutdown — `Container::shutdown` roda todos os hooks
    /// registrados em ordem **reversa** (LIFO), como destructors. Registro é manual:
    /// o consumidor chama isso logo depois de montar o recurso (dentro do próprio
    /// `init()`/builder), então ordem de registro casa com ordem de criação.
    pub fn on_shutdown<F, Fut>(&self, hook: F)
    where
        F: Fn(Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let hook: ShutdownHook = Arc::new(move |c: Container| Box::pin(hook(c)));
        self.inner.shutdown_hooks.write().unwrap().push(hook);
    }

    /// Roda todos os hooks registrados via [`Container::on_shutdown`], em ordem
    /// reversa de registro, esperando cada um terminar antes do próximo (sequencial,
    /// não concorrente — ordem importa).
    pub async fn shutdown(&self) {
        let hooks: Vec<ShutdownHook> = {
            let mut hooks = self.inner.shutdown_hooks.write().unwrap();
            std::mem::take(&mut *hooks)
        };
        for hook in hooks.into_iter().rev() {
            (hook.as_ref())(self.clone()).await;
        }
    }

    /// Resolve `T`, sem nome.
    pub async fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, RudiError> {
        self.resolve_inner::<T>(None).await
    }

    /// Resolve `T` registrado sob `name`.
    pub async fn resolve_named<T: Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Result<Arc<T>, RudiError> {
        self.resolve_inner::<T>(Some(name)).await
    }

    /// Como [`Container::resolve`], mas `T` não registrado vira `Ok(None)` em vez de
    /// `Err`. Falha do builder (`BuildFailed`) continua propagando como `Err` —
    /// "não registrado" é diferente de "registrado mas falhou ao construir".
    pub async fn resolve_optional<T: Send + Sync + 'static>(
        &self,
    ) -> Result<Option<Arc<T>>, RudiError> {
        match self.resolve::<T>().await {
            Ok(value) => Ok(Some(value)),
            Err(RudiError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn resolve_inner<T: Send + Sync + 'static>(
        &self,
        name: Option<&str>,
    ) -> Result<Arc<T>, RudiError> {
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
        if RESOLUTION_STACK.try_with(|_| ()).is_ok() {
            self.resolve_any_checked(type_id, name, type_name).await
        } else {
            RESOLUTION_STACK
                .scope(
                    RefCell::new(Vec::new()),
                    self.resolve_any_checked(type_id, name, type_name),
                )
                .await
        }
    }

    async fn resolve_any_checked(
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

        let name_owned = name.map(Box::from);

        let cycle = RESOLUTION_STACK.with(|stack| {
            let stack = stack.borrow();
            if stack
                .iter()
                .any(|(t, n, _)| *t == type_id && *n == name_owned)
            {
                let mut chain: Vec<&'static str> = stack.iter().map(|(_, _, tn)| *tn).collect();
                chain.push(type_name);
                Some(chain)
            } else {
                None
            }
        });
        if let Some(chain) = cycle {
            return Err(RudiError::CircularDependency { chain });
        }

        RESOLUTION_STACK.with(|stack| {
            stack.borrow_mut().push((type_id, name_owned, type_name));
        });
        let _guard = StackPopGuard;

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
