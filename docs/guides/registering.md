# Registering dependencies

```rust
use rudi::Container;

let c = Container::new(); // local container — see Testing for why to always use this in tests
```

`rudi::container()` gets the global, lazy, 1-per-process instance instead — see [Get started](README.md#get-started). Both `Container::new()` and `rudi::container()` return the same `Container` type; every method below works identically on either.

## 1. Instance

Register a value that's already built — e.g. a config read and validated outside the library:

```rust
c.register_instance(MyConfig { level: "info".into() });
```

## 2. Transient

A builder that reruns on every `resolve` — no cache, a fresh instance each time:

```rust
c.register_transient::<MyType, _, _, std::convert::Infallible>(|c| async move {
    Ok(MyType::new())
});
```

The builder's signature is `Fn(Container) -> Fut where Fut: Future<Output = Result<T, E>>`, with `E: std::error::Error + Send + Sync + 'static`. There's no infallible-only overload — for a builder that can't fail, use `std::convert::Infallible` as `E`.

## 3. Singleton

Same builder shape as transient, but cached: the 1st `resolve` runs the builder, every later `resolve` returns the same `Arc` — including under concurrency (the builder is guaranteed to run exactly once, even if several tasks call `resolve` at the same time before the value is ready):

```rust
c.register_singleton::<MyType, _, _, std::convert::Infallible>(|c| async move {
    Ok(MyType::new())
});
```

## 4. Bind (port / trait object)

Registers an implementation resolvable through its trait, not its concrete type — see [Binding a port](binding.md) for the full picture, including the macro-driven `bind` (no manual closure).

```rust
c.bind_with::<dyn MyPort, _, _, std::convert::Infallible>(|c| async move {
    Ok(std::sync::Arc::new(MyAdapter::new()) as std::sync::Arc<dyn MyPort>)
});
```

## Named variants

Every registration mode above has a `_named` counterpart, so several instances of the same type can coexist under different names — e.g. a `primary` and a `replica` database, without inventing artificial wrapper types just to tell them apart:

```rust
c.register_instance_named("primary", DbConfig { uri: "postgres://primary".into() });
c.register_instance_named("replica", DbConfig { uri: "postgres://replica".into() });

c.register_transient_named::<MyType, _, _, std::convert::Infallible>("x", |c| async move { Ok(MyType::new()) });
c.register_singleton_named::<MyType, _, _, std::convert::Infallible>("y", |c| async move { Ok(MyType::new()) });
```

## "Last registration wins"

Registering the same type (with the same name, or both unnamed) twice overwrites the previous entry — there's no error, no accumulation. This is the exact mechanism [port binding](binding.md) relies on to pick a provider by whichever `init()` ran last (e.g. selected by the consumer's own env-based logic).
