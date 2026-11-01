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
  sinks, then records to `tracing` at the type's level.
- Flexible serialization: none, `serde`, or `facet`; opt-in per type, never a
  bound on `Message`.
- Catalogue: every linked-in message type, auto-registered and walkable as plain
  data; serialize it into a manifest keyed by the resolved `id` and test it for duplicates.
- Automatic origin: crate / module / file / line / column captured per
  message, object-safe and drift-free.
- Ambient autocapture: `Option` fields fill at emit time from same-named
  fields on the surrounding `tracing::instrument` span, across crate boundaries.
- Sinks: each event reaches registered sinks as a typed `&dyn Message`
  *before* the `tracing` handoff; stay generic via accessors or downcast.
- Routing: on static `tags()` or `meta()` with no downcast, or (with `facet`) on
  live field values read by name.

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

- [`catalogue-facet`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/catalogue_facet/index.html): dump the catalogue as YAML via `facet-yaml`
- [`catalogue-serde`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/catalogue_serde/index.html): dump the catalogue as YAML via `serde`
- [`instrument`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/instrument/index.html): fill `Option` fields from surrounding spans
- [`sink-facet`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/sink_facet/index.html): filter on a live field value via `as_facet`
- [`sink-serde`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/sink_serde/index.html): forward each event as JSON via `as_serialize`
- [`sink`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/sink/index.html): a generic line printer plus a typed downcasting sink
- [`tags`](https://docs.rs/tracing-wide/latest/tracing_wide/examples/tags/index.html): route messages by tag with no downcast
