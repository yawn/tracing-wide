//! Fixtures shared by the test binaries; each binary uses a subset, so dead code is allowed.
#![allow(dead_code)]

use tracing::field::Visit;
use tracing_wide::message;

/// Collects event fields as `(name, rendered value)` pairs.
pub struct KvGrab(pub Vec<(String, String)>);

/// Explicit id that is deliberately not its `msg`.
#[message(msg = "payment charged", id = "billing.payment_charged")]
#[derive(Default)]
pub struct Charged {
    pub cents: usize,
}

/// No explicit `msg`: the catalogue entry is identified by the struct name.
#[message]
#[derive(Default)]
pub struct Renamed {
    pub x: usize,
}

/// Emitted when a service finishes starting up.
#[message(msg = "service started", level = warn, owner = "platform", tags = ["platform", "startup"])]
#[derive(Default)]
pub struct Started {
    #[field(unit = "count")]
    pub attempt: usize,
    #[deprecated = "fold the id into `service`"]
    pub legacy_id: Option<usize>,
    /// Logical name of the service.
    pub service: &'static str,
}

impl Visit for KvGrab {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.push((field.name().to_string(), value.to_string()));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.push((field.name().to_string(), value.to_string()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .push((field.name().to_string(), format!("{value:?}")));
    }
}
