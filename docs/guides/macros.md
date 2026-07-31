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

Decorates the **whole `impl` block**, not a single fn inside it — this is a hard restriction, not a stylistic choice: emitting `impl Injectable for Tipo` (a sibling top-level item) is only possible from a macro invoked at that level. An attribute macro applied to an *inner* fn can only ever expand into more inner fns — Rust doesn't allow an `impl` block nested inside another `impl` block, so there's no way to emit `impl Injectable for Tipo` from inside `impl Tipo { ... }`.

Rust has no language-level "constructor" — no keyword, no trait, nothing the compiler treats specially, `new` is pure convention (even in the standard library). So `#[injectable]` finds the constructor by **shape**, not by name: it scans every fn in the `impl` block with no `self` receiver, and picks the one where **every parameter is marked** `#[inject]`, `#[inject_all]`, or `#[container]`:

```rust
struct LoggerSlogAdapter {
    config: Arc<LoggerSlogConfig>,
}

#[injectable(dyn LoggerPort)] // port argument is optional
impl LoggerSlogAdapter {
    // any name works — `build`, `new`, whatever. Stays 100% synchronous and
    // untouched: callable directly in a test (`LoggerSlogAdapter::build(cfg)`,
    // no `.await`). All the async lives inside the `Injectable::build` this
    // macro generates alongside it.
    fn build(#[inject] config: Arc<LoggerSlogConfig>) -> Self {
        Self { config }
    }

    // any other inherent methods pass through untouched
}
```

- **No port argument** (`#[injectable]`) → `type Port = Self` — usable with [`register_singleton_injectable`](registering.md), resolved by concrete type.
- **`#[injectable(dyn PortTrait)]`** → `type Port = dyn PortTrait` — usable with [`bind`](binding.md#bind-macro-driven-recommended), resolved through the trait.

### Parameter markers

| Marker | Parameter type | Resolves via |
| --- | --- | --- |
| `#[inject]` | `Arc<T>` (`T` concrete or `dyn Trait`) | `resolve::<T>()` — or `resolve::<Arc<T>>()` + flatten when `T` is `dyn Trait` (`resolve`'s implicit `T: Sized` bound rules out resolving an unsized trait object directly) |
| `#[inject_all]` | `Vec<Arc<T>>` | [`resolve_all`](binding.md#resolving-every-implementation-of-a-port), same flattening rule per item |
| `#[container]` | `&Container`/`Container` | hands over the `Container` already in scope — never goes through `resolve` (there's no "resolve the container from itself" lookup) |

```rust
struct HealthcheckUsecase {
    pingables: Vec<Arc<dyn PingablePort>>,
}

#[injectable]
impl HealthcheckUsecase {
    fn new(#[inject_all] pingables: Vec<Arc<dyn PingablePort>>) -> Self {
        Self { pingables }
    }
}
```

Constructors can mix markers freely (`fn build(#[inject] config: Arc<Config>, #[container] c: &Container) -> Self`). Return type follows the same rule as before: bare `Self` or `Result<Self, E>` — bare `Self` defaults `Injectable::Error` to `RudiError` (not `Infallible`), since the generated body always has at least 1 fallible `resolve`/`resolve_all` call to propagate.

### Compile-time validation

- 0 fns match the constructor shape → `compile_error!` ("no candidates").
- 2+ fns match → `compile_error!`, naming every candidate ("ambiguous — pick one shape per `impl` block").
- A parameter marked `#[inject]`/`#[inject_all]` with the wrong shape (not `Arc<T>`/`Vec<Arc<T>>`) → `compile_error!` pointing at that parameter.
- The `impl` can't be a trait impl (`impl Trait for Type`) — only `impl Type { ... }`.

Any violation fails at `cargo build` time, never a runtime surprise.

## `#[inject]`

A **different** macro from the `#[inject]` *marker* used inside `#[injectable]` constructors above — same word, 2 separate mechanisms (one's a `#[proc_macro_attribute]` applied to a whole fn; the other is an inert attribute `#[injectable]` consumes on a single parameter). This one decorates any free fn or method: it removes a parameter marked `#[container]` from the public signature, injecting `rudi::container()` as the first statement of the body:

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

Both this `#[container]` marker and `#[injectable]`'s param markers rely on the same trick — matching an attribute, not the parameter's type name — so both support import aliases:

```rust
use rudi::Container as C;

#[inject]
async fn setup(#[container] c: &C) {
    c.register_instance(MyConfig::default());
}
```

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
