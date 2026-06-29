# `tracing-wide`

[![CI](https://github.com/yawn/tracing-wide/actions/workflows/ci.yml/badge.svg)](https://github.com/yawn/tracing-wide/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/tracing-wide/badge.svg)](https://docs.rs/tracing-wide)

> [!CAUTION]
> This is *NOT* an official [tokio](https://tokio.rs) / [tokio-tracing](https://github.com/tokio-rs/tracing) product or associated crate.

This crate enables _wide_ events for tokio [`tracing`](https://docs.rs/tracing). It enables additional functionality for tracing but does not implement / replace it.

Wide events are built as one struct per event, carrying every observability-relevant field for that event. The message text of such an event stays static; all variance lives in typed fields.

The core of the crate is `no_std` and runs in WASM.

Highlights include:

- Typed wide events: one struct per event via `#[message]`; `event!` checks
  required fields, fills unset `Option`s from the ambient span, fans out to
  subscribers, then records to `tracing` at the type's level.
- Flexible serialization: none, `serde`, or `facet`; opt-in per type, never a
  bound on `Message`.
- Catalogue: every message a binary can emit, auto-registered and walkable,
  including messages defined in libraries or member crates as long as they're
  linked in. The catalogue is representation-agnostic - it's plain data you can
  serialize to any format (or none): a manifest (`msg` is the unique join key)
  that non-technical stakeholders can reason about, with duplicate keys detectable
  in a test.
- Automatic origin: crate / module / file / line / column captured per
  message, object-safe and drift-free.
- Ambient autocapture: `Option` fields fill at emit time from same-named
  fields on the surrounding `tracing::instrument` span, across crate boundaries.
- Subscribers: each event reaches registered sinks as a typed `&dyn Message`
  *before* the `tracing` handoff; stay generic via accessors or downcast.
- Routing: on static `tags()` with no downcast, or (with `facet`) on live
  field values read by name.

## Getting started

```rust
use tracing_wide::{event, message};

// One struct per event: the message text is static, the fields carry the variance.
#[message(msg = "hello world")]
struct Hello {
    who: &'static str,
}

// Emit it: builds the struct and records it to `tracing` at the type's level.
event!(Hello { who: "world" });
```

## Examples

- [`catalogue-facet`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#catalogue-facet): dump the catalogue as YAML via `facet-yaml`
- [`catalogue-serde`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#catalogue-serde): dump the catalogue as YAML via `serde`
- [`instrument`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#instrument): fill `Option` fields from surrounding spans
- [`subscriber-facet`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#subscriber-facet): filter on a live field value via `as_facet`
- [`subscriber-serde`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#subscriber-serde): forward each event as JSON via `as_serialize`
- [`subscriber`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#subscriber): a generic line printer plus a typed downcasting sink
- [`tags`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/index.html#tags): route messages by tag with no downcast
