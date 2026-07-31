# Binding a port

A "port" here is just a trait — `LoggerPort`, `DatabasePort`, whatever your domain calls it. Binding registers a concrete implementation resolvable **through the trait**, so the rest of the code never learns which concrete adapter is behind it.

## `bind` (macro-driven, recommended)

Requires the implementation to have `#[injectable(dyn Port)]` on its `impl` block — see [Macros](macros.md#injectable):

```rust
c.bind::<LoggerSlogAdapter, dyn LoggerPort>();

let logger = c.resolve::<Arc<dyn LoggerPort>>().await.unwrap();
logger.info("Hello World!");
```

No manual closure needed — `bind::<Impl, Port>()` uses the `Injectable::build` + `Injectable::into_port` that `#[injectable(dyn Port)]` generated.

## `bind_with` (manual, no macro required)

Same effect, with an explicit builder — useful before adopting macros, or for cases the macro's single-port-per-`impl` limitation doesn't fit:

```rust
c.bind_with::<dyn LoggerPort, _, _, std::convert::Infallible>(|c| async move {
    Ok(std::sync::Arc::new(LoggerSlogAdapter::new()) as std::sync::Arc<dyn LoggerPort>)
});
```

## Last bind wins

Calling `bind`/`bind_with` twice for the same port overwrites the previous registration — no error, no accumulation. This is exactly the mechanism that lets a `mod.rs`-level `init()` pick a provider by whatever selection logic the consumer wrote (env var, feature flag, config), without the library itself deciding or reading anything:

```rust
// infra/database/mod.rs — consumer code, not part of rudi
pub fn init(c: &Container, provider: &str) {
    match provider {
        "postgres" => postgres::init(c, "postgres://localhost/db"),
        "mongodb" => mongodb::init(c, "mongodb://localhost/db"),
        other => panic!("Invalid database provider: {other}"),
    }
}
```

Whichever `init` ran last is the one `resolve::<Arc<dyn DatabasePort>>()` returns — downstream code that just calls `resolve` never needs to know which one it got.

## Multiple ports per implementation

`#[injectable(dyn Port)]` only takes 1 port per `impl` block. An adapter implementing 2 traits needs `bind_with` for the 2nd one — the macro-driven `bind` only covers the port declared in the attribute.

## Resolving every implementation of a port

`bind`/`bind_with`'s "last wins" rule fits a single-selection port (1 database provider chosen by env). It doesn't fit the opposite, equally common shape: a healthcheck-style port where you want **every** registered implementation, not just the last one — "resolve everything that knows how to `ping()`, ping each of them."

`bind_many`/`resolve_all` cover that case, with storage completely separate from `bind`/`bind_with` — the same `Impl` can be registered through both, for different ports, with zero interference:

```rust
trait PingablePort: Send + Sync {
    async fn ping(&self) -> Result<(), Error>;
}

// each healthcheckable adapter accumulates instead of overwriting
c.bind_many::<DatabasePostgresAdapter, dyn PingablePort>();
c.bind_many::<LoggerSlogAdapter, dyn PingablePort>();

// resolves every accumulated implementation, in registration order
let pingables = c.resolve_all::<Arc<dyn PingablePort>>().await?;
for p in pingables {
    p.ping().await?;
}
```

- `resolve_all` with no prior `bind_many` for that port returns `Ok(vec![])` — empty is not an error, unlike `resolve`.
- Each accumulated implementation is cached individually (double-init safe under concurrency), the same guarantee `register_singleton`/`bind` give a single slot.
- There's no `bind_many_named`/`resolve_all_named` yet — not needed by any concrete use case so far; add it if one shows up.

See the full [`PingablePort` example](https://github.com/leandroluk/rust-rudi/blob/main/METACODE.md#exemplo-healthcheck-via-multi-bind-pingableport) in `METACODE.md` for the complete wiring, healthcheck fn included.
