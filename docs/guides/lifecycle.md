# Lifecycle & debugging

## Optional dependencies

Every dependency is required by default — unregistered means `Err(RudiError::NotFound)`. For the "resolve if it exists, `None` otherwise" case (feature flags, optional plugins, an adapter only present in some environments), use `resolve_optional`:

```rust
let maybe_metrics = c.resolve_optional::<MetricsCollector>().await?; // Ok(None) if unregistered
```

Only `NotFound` becomes `None` — a builder that panics or returns `Err` (`BuildFailed`) still propagates as `Err`. Absence isn't the same as failure.

`#[inject]` parameters and `#[derive(Injectable)]` fields accept `Option<Arc<T>>` for the same effect, no manual `resolve_optional` call needed:

```rust
#[injectable]
impl Service {
    fn build(#[inject] metrics: Option<Arc<MetricsCollector>>) -> Self {
        Self { metrics }
    }
}
```

Works with trait objects too (`Option<Arc<dyn Port>>`) — same double-`Arc` flattening as regular `#[inject]`.

## Circular dependencies

A builder that (directly or transitively) tries to resolve a type already being resolved higher up the same chain gets `Err(RudiError::CircularDependency { chain })` instead of hanging forever:

```rust
c.register_singleton::<A, _, _, RudiError>(|c| async move {
    c.resolve::<B>().await?; // B's builder resolves A again → cycle
    Ok(A)
});
```

Detection tracks the current resolution chain via `tokio::task_local` — scoped to the logical `.await` chain, not the OS thread (a task can move between threads between awaits under a multi-threaded runtime, so thread-local storage would give wrong answers). No compile-time detection — a cycle only surfaces the first time that code path actually resolves.

## Shutdown hooks

Singletons holding an external resource (a connection pool, a socket) have no automatic "close" — register a cleanup closure with `on_shutdown`, run everything with `shutdown`:

```rust
c.register_singleton::<DbPool, _, _, std::convert::Infallible>(|c| async move {
    let pool = DbPool::connect().await;

    let pool_for_hook = pool.clone();
    c.on_shutdown(move |_c| {
        let pool = pool_for_hook.clone();
        async move { pool.close().await; }
    });

    Ok(pool)
});

// later, on graceful shutdown:
c.shutdown().await;
```

Hooks run **sequentially**, in **reverse** registration order (LIFO — like destructors). Registration is manual and explicit: no automatic detection tied to the type system, no `Drop`-like trait to implement. Register a hook right after building the resource (inside the same builder/`init()`), and registration order naturally matches creation order.

## Debugging

Two read-only introspection methods for "why didn't this resolve" debugging:

```rust
for entry in c.debug_entries() {
    println!("{} ({:?}){}", entry.type_name, entry.kind,
        entry.name.map(|n| format!(" [{n}]")).unwrap_or_default());
}

for (parent, child) in c.debug_edges() {
    println!("{} depends on {}", parent.type_name, child.type_name);
}
```

- **`debug_entries()`** lists everything currently registered — type, optional name, and mode (`Instance`/`Transient`/`Singleton`/`Many`). Covers both `entries` (`register_*`/`bind`/`bind_with`) and `many` (`bind_many` — 1 `DebugEntry` per accumulated implementation).
- **`debug_edges()`** returns parent→child edges **observed** during resolutions that have already happened — it reuses the same resolution-chain tracking as circular-dependency detection. This is **not** a static analysis of the whole dependency graph; it only knows about edges some actual `resolve` call has walked through so far. Empty until at least 1 nested resolution has occurred.
