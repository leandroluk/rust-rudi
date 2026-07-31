# Macros

`rudi-macros` generates code around the `Injectable` trait (`crates/rudi/src/injectable.rs`) — 3 macros, no runtime logic of their own, pure codegen via `syn`/`quote`.

```rust
pub trait Injectable: Sized + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    type Port: ?Sized + Send + Sync + 'static;

    fn build(c: Container) -> impl Future<Output = Result<Self, Self::Error>> + Send;
    fn into_port(built: Arc<Self>) -> Arc<Self::Port>;
}
```

## `#[injectable]`

Decorates the **whole `impl` block**, not the standalone `build` fn — this is a hard restriction, not a stylistic choice: a proc-macro attribute only receives the exact item it's attached to, with zero visibility into the surrounding scope. Attached directly to `fn build` (like `METACODE.md`'s original hypothesis shows), the macro has no way to know the concrete type's name. Moving 1 level up gives it the whole `syn::ItemImpl`, `self_ty` included:

```rust
struct LoggerSlogAdapter {
    config: LoggerSlogConfig,
}

#[injectable(dyn LoggerPort)] // port argument is optional
impl LoggerSlogAdapter {
    async fn build(c: &Container) -> Self {
        let config = c.resolve::<LoggerSlogConfig>().await.unwrap();
        Self { config: (*config).clone() }
    }

    // any other inherent methods pass through untouched
}
```

`build` supports all 4 combinations:

| | `-> Self` | `-> Result<Self, E>` |
| --- | --- | --- |
| **sync `fn`** | wraps in `Ok(...)` internally | passed through as-is |
| **`async fn`** | `.await`s, then wraps in `Ok(...)` | `.await`s, passed through as-is |

- **No port argument** (`#[injectable]`) → `type Port = Self` — usable with [`register_singleton_injectable`](registering.md), resolved by concrete type.
- **`#[injectable(dyn PortTrait)]`** → `type Port = dyn PortTrait` — usable with [`bind`](binding.md#bind-macro-driven-recommended), resolved through the trait.

### Compile-time validation

- The `impl` block must have exactly 1 fn named `build`.
- `build` must have exactly 1 parameter, `&Container`/`Container` written literally (no alias — see [Known limitations](README.md#known-limitations)).
- The `impl` can't be a trait impl (`impl Trait for Type`) — only `impl Type { ... }`.

Any violation fails at `cargo build` time with a `compile_error!`, never a runtime surprise.

## `#[inject]`

Removes a parameter marked `#[container]` from the public signature, injecting `rudi::container()` as the first statement of the body:

```rust
use rudi::{inject, Container};

#[inject]
fn init(#[container] c: &Container) {
    c.register_instance(MyConfig::default());
}

fn main() {
    init(); // no argument — the macro resolved the container on its own
}
```

Unlike `#[injectable]`, this **does** support import aliases:

```rust
use rudi::Container as C;

#[inject]
async fn setup(#[container] c: &C) {
    c.register_instance(MyConfig::default());
}
```

That's the whole reason `#[inject]` uses a marker attribute instead of matching on the parameter's type name — `#[container]` doesn't care what the type is called, only that it's the one parameter meant to be injected. `#[injectable]`'s `build` parameter can't use this trick (see [design.md](https://github.com/leandroluk/rust-rudi/blob/main/.specs/features/di-macros/design.md) for why the 2 macros ended up with different constraints here).

### Compile-time validation

- Exactly 1 parameter must carry `#[container]` — 0 or 2+ both fail to compile with a clear message.

## `#[derive(Injectable)]`

Resolves each field of a struct individually from the container — for the case where there's no custom construction logic, just "assemble this from what's already registered":

```rust
use std::sync::Arc;

#[derive(rudi::Injectable)]
struct Service {
    logger: Arc<dyn LoggerPort>,
    config: Arc<MyConfig>,
}

c.register_singleton_injectable::<Service>();
let service = c.resolve::<Service>().await.unwrap();
```

Every field must be `Arc<T>` — the same shape `resolve` always returns (see [Resolving](resolving.md#why-always-arct)). Works on named-field structs, tuple structs (fields resolved positionally), and unit structs (`build` just returns `Self`, no resolving at all).

- `type Error` is always `RudiError` — no custom error type for the derive. If you need one, write `#[injectable]` manually instead.
- `type Port` is always `Self` — the derive has no equivalent of `#[injectable(dyn Port)]`; bind through a trait using `#[injectable]` or `bind_with`.

A missing dependency propagates as `RudiError::NotFound`, exactly like calling `resolve` manually would.
