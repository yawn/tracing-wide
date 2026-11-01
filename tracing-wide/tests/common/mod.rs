//! Fixtures shared by the test binaries; each binary uses a subset, so dead code is allowed.
#![allow(dead_code)]

use std::fmt::Debug;
use std::mem;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber, subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry;
#[cfg(feature = "catalogue")]
use tracing_wide::catalogue::{self, MessageDescriptor};
use tracing_wide::message;

/// Explicit id that is deliberately not its `msg`.
#[message(msg = "payment charged", id = "billing.payment_charged")]
#[derive(Default)]
pub struct Charged {
    pub cents: usize,
}

/// A tracing layer that pushes `map(event)` into the shared log for each event.
pub struct Collector<T>(pub Arc<Mutex<Vec<T>>>, pub fn(&Event<'_>) -> T);

/// Collects event fields as `(name, rendered value)` pairs.
pub struct KvGrab(Vec<(String, String)>);

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

impl<T: Send + 'static> Collector<T> {
    /// Runs `f` under a registry with a collector of `map`, and returns what it collected.
    pub fn collect<F: FnOnce()>(map: fn(&Event<'_>) -> T, f: F) -> Vec<T> {
        let log = Arc::new(Mutex::new(Vec::new()));

        subscriber::with_default(registry().with(Self(log.clone(), map)), f);
        mem::take(&mut *log.lock().unwrap())
    }
}

impl KvGrab {
    /// The fields of `event`.
    pub fn fields(event: &Event<'_>) -> Vec<(String, String)> {
        let mut grab = Self(Vec::new());

        event.record(&mut grab);
        grab.0
    }
}

impl<S: Subscriber, T: Send + 'static> Layer<S> for Collector<T> {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        self.0.lock().unwrap().push((self.1)(event));
    }
}

impl Visit for KvGrab {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.push((field.name().to_string(), value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.push((field.name().to_string(), value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0
            .push((field.name().to_string(), format!("{value:?}")));
    }
}

/// The catalogue entry whose `msg` is `msg`; panics when there is none.
#[cfg(feature = "catalogue")]
pub fn descriptor(msg: &str) -> &'static MessageDescriptor {
    catalogue::all()
        .find(|d| d.msg == msg)
        .unwrap_or_else(|| panic!("no catalogue entry for {msg:?}"))
}
