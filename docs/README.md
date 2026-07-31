# rudi

Type-safe dependency injection for Rust, with guaranteed async resolution and macros that eliminate manual wiring boilerplate.

[![ci](https://github.com/leandroluk/rudi/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/leandroluk/rudi/actions/workflows/ci.yml)
[![GitHub](https://img.shields.io/badge/GitHub-leandroluk%2Frudi-181717?logo=github)](https://github.com/leandroluk/rudi)

## Why this exists

Codebases following the hexagonal / ports-and-adapters pattern do the same wiring by hand over and over: config gets read, an adapter gets built from it, the adapter gets registered against its port, repeated for every provider (`LoggerPort` → slog, `DatabasePort` → postgres or mongodb, depending on env). That wiring usually ends up scattered across a growing `main.rs` or a chain of hand-written `init()` calls.

`rudi` centralizes it behind a global, lazy, 1-per-process `Container`, with 2 rules baked in from day one:

- **The library never reads `std::env`.** Reading environment variables is always the consumer's `init()` responsibility — the library stays fail-fast-agnostic and testable.
- **Resolution is always `async`**, even for synchronous constructors. This guarantees builders that need I/O (opening a database connection, etc.) are supported uniformly, without sync-over-async workarounds bolted on later.

## Design principles

- **No manual `Container` passing.** `rudi::container()` fetches the environment/global instance (1 per process, lazy). Consumers never call `Container::new()` themselves outside of tests.
- **`resolve` always returns `Arc<T>`**, never `T` by value — this keeps the API uniform between concrete types and trait objects (`resolve::<Config>()` and `resolve::<Arc<dyn Port>>()` have the same shape), and avoids requiring every registered type to implement `Clone`.
- **Multiple providers for the same port follow one pattern.** Each provider gets its own `init()`; the port-level module decides which one to call (however the consumer chooses — env var, config flag, etc.); the `bind` is always against the `dyn Port`, never the concrete provider type, so downstream code never learns which implementation is behind it.
- **Failures are typed, never a panic inside the library.** `resolve` on an unregistered type returns `Err(RudiError::NotFound { .. })`. Builder errors propagate as `RudiError::BuildFailed`.

## Get started

```toml
[dependencies]
rudi = { path = "crates/rudi" } # or the published version, once released
```

```rust
use rudi::{injectable, Container};

struct Greeter;

#[injectable]
impl Greeter {
    fn build(_c: &Container) -> Self {
        Greeter
    }
}

impl Greeter {
    fn hello(&self) -> &'static str {
        "hello!"
    }
}

#[tokio::main]
async fn main() {
    let c = rudi::container();
    c.register_singleton_injectable::<Greeter>();

    let greeter = c.resolve::<Greeter>().await.unwrap();
    println!("{}", greeter.hello());
}
```

From here:

- [Registering dependencies](guides/registering.md) — the 4 registration modes, plus named variants for coexisting instances of the same type
- [Resolving](guides/resolving.md) — `resolve`/`resolve_named`, the `Arc<T>` contract, error handling
- [Binding a port](guides/binding.md) — `bind`/`bind_with`, trait objects, "last bind wins"
- [Macros](guides/macros.md) — `#[injectable]`, `#[inject]`, `#[derive(Injectable)]`
- [Testing](guides/testing.md) — why never to touch the global container in a test, and `with_container`
- [metacode walkthrough](examples/metacode.md) — the full example, reproducing [`METACODE.md`](https://github.com/leandroluk/rudi/blob/main/METACODE.md)'s original hypothesis with real macros end-to-end

## Errors

`RudiError` (`thiserror`-derived enum, `crates/rudi/src/error.rs`) has 3 variants:

- `NotFound { type_name, name }` — `resolve`/`resolve_named` on a type that was never registered under that name
- `BuildFailed { type_name, source }` — a `register_transient`/`register_singleton`/`bind` builder returned an `Err`
- `DowncastFailed { type_name }` — internal defense; shouldn't occur in normal use (would indicate a bug in `rudi` itself)

Always check with a `match`/`?`, never string-compare the error message.

## Known limitations

- **Circular dependencies aren't detected.** A builder that (directly or transitively) resolves its own type again deadlocks at runtime, inside the `OnceCell` guarding the singleton's initialization. Not handled in v1 — avoid designing a dependency graph with cycles.
- **`#[injectable]`'s `build` parameter doesn't support import aliases.** It must be `Container`/`&Container` written literally — the macro matches on the last path segment of the parameter's type, since proc-macros don't have type-checker information. Different from `#[inject]`, whose `#[container]` marker attribute sidesteps this entirely (see [Macros](guides/macros.md#inject)).
- **`resolve::<Arc<dyn Port>>()` actually returns `Arc<Arc<dyn Port>>`.** A consequence of the "resolve always returns `Arc<T>`" rule applied uniformly even when `T` is itself `Arc<dyn Port>` (from a `bind`/`bind_with` call). Transparent in practice via Rust's auto-deref on method calls (`resolved.info()` works exactly the same), but worth knowing if you're storing the value or pattern-matching on its type.

## About the project

See `.specs/project/PROJECT.md` in the repository for the full vision/scope, and `.specs/features/*/spec.md` + `design.md` for every feature's requirements and the design decisions behind them (including the ones proc-macro language restrictions forced — like `#[injectable]` decorating the `impl` block instead of the `build` fn).
