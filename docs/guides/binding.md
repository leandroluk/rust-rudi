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
