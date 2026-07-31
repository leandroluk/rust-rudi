# metacode walkthrough

[`crates/rudi/examples/metacode/`](https://github.com/leandroluk/rust-rudi/tree/main/crates/rudi/examples/metacode) reproduces the `domain`/`infra` tree described in [`METACODE.md`](https://github.com/leandroluk/rust-rudi/blob/main/METACODE.md) — the original design hypothesis this whole library was built from — using every macro end-to-end:

```bash
cargo run --example metacode -p rudi
```

```
[slog:info] Hello World!
metacode example ok (postgres + slog via rudi container global)
```

## Structure

```
examples/metacode/
├── domain/
│   ├── port/
│   │   ├── database.rs   # DatabasePort trait + DatabaseError
│   │   ├── logger.rs     # LoggerPort trait
│   │   └── mod.rs
│   └── mod.rs
├── infra/
│   ├── database/
│   │   ├── mongodb/{config,adapter,mod}.rs
│   │   ├── postgres/{config,adapter,mod}.rs
│   │   └── mod.rs        # picks the provider
│   ├── logger/
│   │   ├── slog/{config,adapter,mod}.rs
│   │   └── mod.rs
│   └── mod.rs             # wires database + logger together
└── main.rs
```

Same shape as the original hypothesis: a port per capability, a provider per implementation, a `mod.rs` at each level that decides which provider wins.

## 1 documented deviation from `METACODE.md`

Comes from a language restriction the original hypothesis document didn't (couldn't) anticipate — not a stylistic choice: `#[injectable]` decorates the `impl` block, not the `build` fn.

```rust
// METACODE.md's original hypothesis (doesn't compile as written):
impl LoggerSlogAdapter {
    #[injectable]
    pub fn build(c: &Container) -> Self { ... }
}

// What actually works:
#[injectable(dyn LoggerPort)]
impl LoggerSlogAdapter {
    fn build(#[inject] config: Arc<LoggerSlogConfig>) -> Self { ... }
}
```

A proc-macro attribute only ever sees the exact item it decorates — attached to `build` alone, it has no way to learn the enclosing `impl`'s concrete type name, and there's no way to emit the required `impl Injectable for LoggerSlogAdapter` (a sibling top-level item) from inside another `impl` block's braces. See [Macros](../guides/macros.md#injectable).

The `build` fn itself stays 100% synchronous — `#[inject]` on the `config` parameter resolves `LoggerSlogConfig` from the container automatically; all the `async`/`.await` machinery this needs lives inside the `Injectable::build` the macro generates alongside `build`, never in `build` itself.

## What each piece does

- **`domain/port/`** — trait definitions only, no `rudi` dependency at all. Ports are pure domain code.
- **`infra/logger/slog/config.rs`** — `LoggerSlogConfig`, registered as a plain instance (`register_instance`) in `init()`, not through `#[injectable]` — it has no custom construction logic of its own.
- **`infra/logger/slog/adapter.rs`** — `LoggerSlogAdapter`, `#[injectable(dyn LoggerPort)]` on its `impl` block; `build(#[inject] config: Arc<LoggerSlogConfig>)` resolves the config automatically, no manual `c.resolve()` call.
- **`infra/logger/slog/mod.rs`** — `init(c, level)` registers the config, then `c.bind::<LoggerSlogAdapter, dyn LoggerPort>()`.
- **`infra/logger/mod.rs`** — `init(c, provider)` picks which sub-provider's `init` to call (only `"slog"` exists here, but the shape matches `infra/database/mod.rs`, which has 2).
- **`infra/database/{postgres,mongodb}/`** — mirror the logger structure exactly, 1 provider each, both binding against the same `dyn DatabasePort`.
- **`infra/database/mod.rs`** — picks `postgres` vs `mongodb` by a parameter (the library itself never reads env — see [Why this exists](../README.md#why-this-exists) — so this example takes the provider name as a plain argument instead of calling `std::env::var` itself).
- **`main.rs`** — calls `infra::init(&c, "postgres", "slog")`, then resolves `Arc<dyn LoggerPort>` and `Arc<dyn DatabasePort>` and uses them — no code anywhere in `main.rs` knows it's talking to a `LoggerSlogAdapter` or a `DatabasePostgresAdapter` specifically.
