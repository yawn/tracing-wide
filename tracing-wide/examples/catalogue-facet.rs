//! Generate a YAML manifest from the message catalogue — through facet.
//!
//! Run: `just example-catalogue-facet` (the facet twin of `catalogue-serde`)
//!
//! Identical to `catalogue-serde`, but the descriptors are dumped through
//! `facet` (here `facet-yaml`) instead of serde. `level`, `origin`, and the
//! `meta` maps render the same — the catalogue manifest is serializer-agnostic.

use std::collections::{BTreeMap, BTreeSet};

use tracing_wide::{catalogue::MessageDescriptor, facet::Facet, message};

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
    #[derive(Facet)]
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

    let manifest =
        facet_yaml::to_string(&Manifest { messages, tags }).expect("catalogue serializes to YAML");

    print!("{manifest}");
}
