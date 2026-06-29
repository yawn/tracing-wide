//! Shared fixtures for the per-feature test binaries. Message types asserted by
//! more than one suite (so they're defined once — and, with `catalogue`,
//! registered once per binary rather than duplicated) live here, alongside the
//! field-grabbing visitor reused across suites. Each `tests/*.rs` pulls this in
//! with `mod common;`; every binary uses a different subset, so some items are
//! unused per binary — hence the module-level allow.
#![allow(dead_code)]

use tracing::field::Visit;
use tracing_wide::message;

/// Collects event fields as `(name, rendered value)` pairs.
pub struct KvGrab(pub Vec<(String, String)>);

/// No explicit `msg`: the catalogue entry is keyed by the struct name.
#[message]
#[derive(Default)]
pub struct Renamed {
    pub x: usize,
}

/// Emitted when a service finishes starting up.
//
// The richly-attributed fixture: the core suite emits it and reads its consts,
// the catalogue suite asserts the descriptor those attributes produce — so the
// struct/field docs and attributes below are load-bearing for both.
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
