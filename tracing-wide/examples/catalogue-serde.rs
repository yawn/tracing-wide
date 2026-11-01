//! Generate a YAML manifest from the message catalogue.
//!
//! Run: `just example-catalogue-serde` (see `catalogue-facet` for the facet equivalent)

use std::collections::{BTreeMap, BTreeSet};

use tracing_wide::{catalogue::MessageDescriptor, message};

/// Recorded when a customer is charged; the explicit `id` keeps the key stable when the text changes.
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

    // A keyed manifest needs unique ids; an app checks this in a test or a catalogue binary.
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
        .flat_map(|d| d.tags.iter())
        .collect();

    let manifest = serde_norway::to_string(&Manifest { messages, tags })
        .expect("catalogue serializes to YAML");

    print!("{manifest}");
}
