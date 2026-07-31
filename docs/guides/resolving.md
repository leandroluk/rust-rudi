# Resolving

`resolve` is always `async` — even when what's registered is a plain synchronous value — and always returns `Arc<T>`:

```rust
let value = c.resolve::<MyType>().await?;          // Arc<MyType>
let named = c.resolve_named::<MyType>("x").await?; // Arc<MyType>, registered under "x"
```

## Why always `Arc<T>`

Two options were on the table: return `T` by value (requiring `T: Clone`) or always return `Arc<T>`. `Arc<T>` won because:

- It doesn't require every registered type to implement `Clone`.
- It keeps the API shape identical whether `T` is a plain struct or a trait object (`resolve::<Config>()` and `resolve::<Arc<dyn Port>>()` both "just work" the same way).
- It's the natural fit for a cached singleton shared across every caller.

## Trait objects

Resolving through a [bound port](binding.md) uses the same `resolve`, with the target type being `Arc<dyn Port>`:

```rust
let logger = c.resolve::<Arc<dyn LoggerPort>>().await?;
logger.info("Hello World!");
```

> **Note:** this actually returns `Arc<Arc<dyn LoggerPort>>` under the hood — a consequence of the "always `Arc<T>`" rule applied even when `T` is itself `Arc<dyn Port>`. Calling `.info()` still works transparently through Rust's auto-deref on method calls; see [Known limitations](README.md#known-limitations) if you need to store or match on the value's exact type.

## Errors

An unregistered type (or an unregistered name) returns `Err(RudiError::NotFound { type_name, name })` — never a panic:

```rust
match c.resolve::<MyType>().await {
    Ok(value) => { /* ... */ }
    Err(rudi::RudiError::NotFound { type_name, name }) => {
        eprintln!("{type_name} was never registered (name: {name:?})");
    }
    Err(e) => return Err(e.into()),
}
```

A builder (`register_transient`/`register_singleton`/`bind`) that returns `Err` surfaces as `RudiError::BuildFailed { type_name, source }`, with `source` carrying the original error boxed as `Box<dyn std::error::Error + Send + Sync>` — `errors.Is`-style downcasting isn't built in; match on `BuildFailed` and inspect `source` directly if you need the concrete error back.

## Concurrency

Resolving a singleton whose builder hasn't run yet, from several tasks at once, runs the builder exactly once — every caller gets the same `Arc`, none of them see a partially-built value or trigger a duplicate build.
