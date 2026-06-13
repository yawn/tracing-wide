# `tracing-wide`

> [!CAUTION]
> This is *NOT* an official [tokio](https://tokio.rs) / [tokio-tracing](https://github.com/tokio-rs/tracing) product or associated crate.

Wide structured events for tokio [`tracing`](https://docs.rs/tracing): one struct
per event, defined once, carrying every observability-relevant field for that
site. The message text stays static; all variance lives in typed fields. The
core is `no_std` and runs in WASM — everything else is opt-in behind features.

```rust,ignore
use serde::Serialize;
use tracing_wide::{event, message};

/// A request finished handling.                 // doc comment, recorded in the catalogue
#[message(
    msg = "request completed",                   // static text, the catalogue's unique key
    level = info,                                // severity (default: info)
    tags = ["analytics", "api"],                 // routing intent for potential subscribers
    owner = "platform",                          // arbitrary metadata, recorded in the catalogue
)]
#[derive(Default, Serialize)]                    // Serialize: opt-in, per type
struct RequestCompleted {
    /// Route template, e.g. `/users/:id`.       // field docs, recorded in the catalogue
    route: &'static str,                         // required: set at the event! site
    #[field(unit = "ms")]                        // arbitrary field metadata, recorded in the catalogue
    duration: u64,
    region: Option<String>,                      // Option: may fill from the ambient span
}

/// A payment was captured.
#[message(msg = "payment captured", level = warn, tags = ["analytics", "persist"])]
#[derive(Serialize)]
struct PaymentCaptured {
    amount_cents: u64,
    currency: &'static str,
    #[deprecated = "use amount_cents"]           // recorded in the catalogue; warns at construction
    amount: Option<u64>,
}

// Emit: required fields are checked here; an unset Option stays None
// (and may fill from the surrounding span — see Instrument (ambient) autocapture).
event!(RequestCompleted { route: "/users/:id", duration: 12, ..Default::default() });
```

See [`examples/`](tracing-wide/examples) for more examples.

## Emit — `event!`

`event!(RequestCompleted { .. })` builds the struct, fills any unset `Option`
fields from the ambient span, fans it out to registered subscribers, then records
it to `tracing` at the type's level. `#[message]` is the only supported way to make
a type emittable (the trait is pseudo-sealed); a field named `message` is rejected
(tracing reserves it for the event text) and generics aren't allowed (a message is
a concrete `'static` type).

For spans, use stock `tracing::instrument` — tracing-wide ships no span macro.

## Catalogue — every message a system can emit *(`catalogue`)*

A catalogue enables stakeholder engagement: a serialized catalogue lets non-technical
stakeholders see every message a system emits, so they can reason about it and
build downstream recipients — analytics, BI, alerts — against a stable contract.

`#[message]` auto-registers one descriptor per type; `catalogue::all()` walks
them. Each descriptor carries everything from the definitions above — `msg`,
`level`, `tags`, `origin`, doc comments, `#[field(unit = ...)]` and other
metadata, and deprecations (field- or struct-level). With the `serde` feature the
descriptors `Serialize`, so a build step can dump a manifest — the `catalogue`
example emits YAML keyed by `msg`.

- `msg` is the unique join key; `catalogue::duplicates()` flags collisions (run
  it in a test).
- Link-accurate: the catalogue holds exactly the messages of the crates linked
  into the binary. Registration survives dead-code elimination, so it may
  over-report what actually fires but never under-reports.

### Origin — where a message is defined

`origin()` is automatic provenance captured by `#[message]`: crate, module, file,
line, column — no input, can't drift. It's object-safe, so a subscriber can
attribute or route a `&dyn Message` by its originating crate without a downcast,
and it has a compact `Display`: `mycrate src/lib.rs:12:1`.

## Instrument (ambient) autocapture *(`instrument`)*

`RequestCompleted::region` is an `Option`, so when left unset at the `event!` site
it fills at emit time from the surrounding span scope — by field name, innermost
span wins, across crate boundaries. Required (bare) fields never do this, and a
field already `Some` is never overwritten.

Stock `tracing::instrument` is the contribution surface — it records function
arguments and `fields(..)` by default; install `instrument::layer()` on a
`tracing_subscriber::registry()` stack to capture them. With no layer or no
current span the lookup simply misses; `event!` never fails on ambient state.

> Name-based by design: an `Option` field fills from *any* same-named span field
> in scope, including spans the message author doesn't own — name fields
> deliberately (`token`, `id`, `user` collide easily). The layer also retains
> every span field for the span's lifetime; keep that in mind for sensitive data.

## Subscribe to wide events *(`subscriber`)*

With the `subscriber` feature, `event!` hands each message to every registered
subscriber as a typed `&dyn Message` *before* the tracing handoff — useful for
storing events in a database or forwarding to another subsystem (especially in
frontends). A sink can stay generic through the object-safe accessors, or
downcast to the concrete type via `m.as_any()`.

```rust,ignore
fn on_message(&self, m: &dyn Message) {
    if let Some(body) = m.as_serialize() {     // Some iff the type derives Serialize
        // body is a &dyn erased_serde::Serialize — serialize it with any format
    }
}
```

Serialization is opt-in per type (`#[derive(Serialize)]`, as on both messages
above) and never a bound on `Message`; a type that doesn't derive it yields
`None`. Register sinks into `Subscribers`, then `install()` once — set-once, like
tracing's global default.

> A panicking `on_message` panics the `event!` call site — same posture as
> tracing itself, but a logging sink can crash the app.

## Tags — route by intent

`RequestCompleted` is tagged `analytics` + `api`; `PaymentCaptured`, `analytics`
+ `persist`. Tags are *where to send*, not *where from*: a subscriber routes on
them with no downcast, and one message can fan out to several subsystems.

```rust,ignore
fn on_message(&self, m: &dyn Message) {
    if m.tags().contains(&"analytics") { /* forward */ }
}
```

Tags are sorted, deduped and lowercased at compile time, and lowercase is
enforced. Crate-prefix namespacing is unnecessary — `origin()` already carries
the crate.
