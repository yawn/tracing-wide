//! Generate a YAML manifest from the message catalogue.
//!
//! Run: `just example-catalogue`
//!
//! Walks `tracing_wide::catalogue::all()` and serializes the descriptors
//! directly — the `serde` feature implements `Serialize` on them, so no mirror
//! structs are needed. The `message` leaf is keyed by `msg` (the same unique
//! key emitted events join on); a sibling `tags` leaf lists the sorted union of
//! every routing tag. A build step could redirect this into a checked-in
//! `catalogue.yaml` — the catalogue is the schema.

use std::collections::{BTreeMap, BTreeSet};

// Needed only because we are not actually using any code from
// integration_test_crate. No usage, no linkage, no messages.
use integration_test_crate as _;
use tracing_wide::{catalogue::MessageDescriptor, message};

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

    // A keyed manifest requires unique keys — a duplicate `msg` would silently
    // drop an entry from the map below. An app would put this line in a test or a catalogue generating binary.
    assert_eq!(
        tracing_wide::catalogue::duplicates(),
        Vec::<&str>::new(),
        "duplicate `msg` keys in the catalogue"
    );

    let messages: BTreeMap<&'static str, &'static MessageDescriptor> =
        tracing_wide::catalogue::all().map(|d| (d.msg, d)).collect();

    let tags: BTreeSet<&'static str> = tracing_wide::catalogue::all()
        .flat_map(|d| d.tags.iter().copied())
        .collect();

    let manifest = serde_norway::to_string(&Manifest { messages, tags })
        .expect("catalogue serializes to YAML");

    print!("{manifest}");
}
