//! Generate a YAML manifest from the message catalogue.
//!
//! Run: `just example-catalogue-serde` (see `catalogue-facet` for the facet equivalent)
//!
//! Walks `tracing_wide::catalogue::all()` and serializes the descriptors
//! directly — the `serde` feature implements `Serialize` on them, so no mirror
//! structs are needed. The `message` leaf is keyed by the resolved id (the same
//! unique key `Message::id()` reports on an emitted event, which is `msg` unless
//! `#[message(id = "...")]` overrides it); a sibling `tags` leaf lists the sorted union of
//! every routing tag. A build step could redirect this into a checked-in
//! `catalogue.yaml` — the catalogue is the schema.

use std::collections::{BTreeMap, BTreeSet};

use tracing_wide::{catalogue::MessageDescriptor, message};

/// Recorded when a customer is charged.
///
/// Carries an explicit `id`: the manifest key is the stable join key, not the
/// human sentence, so editing this message's copy does not rename the event for
/// every downstream consumer.
#[message(msg = "payment charged", id = "billing.payment_charged", owner = "billing", tags = ["billing"])]
#[derive(Default)]
struct PaymentCharged {
    #[field(unit = "cents")]
    amount: usize,
}

/// Recorded when a request finishes handling.
#[message(msg = "request completed", level = info, owner = "api", tags = ["api"])]
#[derive(Default)]
struct RequestCompleted {
    #[field(unit = "ms")]
    duration: usize,
    /// Route template, e.g. `/users/:id`.
    route: &'static str,
}

/// Emitted when a service finishes starting up.
///
/// If services start up at all - we had lots of quality issues lately.
#[message(msg = "service started", level = warn, owner = "platform", tags = ["platform", "startup"])]
#[derive(Default)]
struct Started {
    #[field(unit = "count")]
    attempt: usize,
    #[deprecated = "fold the id into `service`"]
    legacy_id: Option<usize>,
    /// Logical name of the service.
    service: &'static str,
}

fn main() {
    #[derive(serde::Serialize)]
    struct Manifest {
        messages: BTreeMap<&'static str, &'static MessageDescriptor>,
        tags: BTreeSet<&'static str>,
    }

    // A keyed manifest requires unique keys — a duplicate id would silently drop
    // an entry from the map below, so the guard and the key must agree on what
    // "unique" means. An app would put this line in a test or a catalogue generating binary.
    let dupes = tracing_wide::catalogue::duplicates();

    assert!(
        dupes.is_empty(),
        "duplicate ids in the catalogue: {dupes:#?}"
    );

    let messages: BTreeMap<&'static str, &'static MessageDescriptor> =
        tracing_wide::catalogue::all()
            .map(|d| (d.resolved_id(), d))
            .collect();

    let tags: BTreeSet<&'static str> = tracing_wide::catalogue::all()
        .flat_map(|d| d.tags.iter().copied())
        .collect();

    let manifest = serde_norway::to_string(&Manifest { messages, tags })
        .expect("catalogue serializes to YAML");

    print!("{manifest}");
}
