//! Subsystem selection by tag.
//!
//! Run: `just example-tags`

use tracing_wide::{
    Message, event, message,
    sink::{Sink, Sinks},
};

/// Forwards anything tagged `analytics` (e.g. to an analytics pipeline).
struct Analytics;

/// Persists anything tagged `persist` (e.g. to a database).
struct Database;

/// Persistence only (and a higher level): never reaches analytics.
#[message(msg = "config changed", level = warn, tags = ["audit", "persist"])]
struct ConfigChanged {
    key: &'static str,
}

/// Analytics only.
#[message(msg = "page viewed", tags = ["analytics"])]
struct PageViewed {
    path: &'static str,
}

/// Tagged for both analytics and persistence: fans out to *both* subsystems.
#[message(msg = "payment captured", tags = ["analytics", "persist"])]
struct PaymentCaptured {
    amount_cents: u64,
    currency: &'static str,
}

impl Sink for Analytics {
    fn on_message(&self, m: &dyn Message) {
        if m.tags().contains("analytics") {
            println!("[analytics] forward {:?}  ({})", m.msg(), m.origin());
        }
    }
}

impl Sink for Database {
    fn on_message(&self, m: &dyn Message) {
        if m.tags().contains("persist") {
            println!("[db]        store   {:?}  ({})", m.msg(), m.origin());
        }
    }
}

fn main() {
    let mut sinks = Sinks::default();
    sinks.register(Box::new(Database));
    sinks.register(Box::new(Analytics));
    sinks.install().ok();

    event!(PaymentCaptured {
        amount_cents: 4200,
        currency: "EUR"
    });

    event!(PageViewed { path: "/pricing" });

    event!(ConfigChanged {
        key: "feature.flags"
    });
}
