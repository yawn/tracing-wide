//! Print the catalogue snapshot: the text an application commits, so that a CI diff shows every semantic change.
//!
//! Run: `just example-catalogue-snapshot` (the YAML twins are `catalogue-serde` and `catalogue-facet`)

use tracing_wide::message;

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
    let snapshot = tracing_wide::catalogue::snapshot()
        .unwrap_or_else(|dupes| panic!("duplicate ids in the catalogue: {dupes:#?}"));

    print!("{snapshot}");
}
