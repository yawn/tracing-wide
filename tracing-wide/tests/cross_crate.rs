//! Cross-crate provenance & routing: a subscriber sees messages — with their
//! automatic `origin().krate` and their `tags()` — from BOTH this test crate
//! and the foreign `integration_test_crate`, without ever naming the foreign
//! types. Dual native/wasm, like `coverage.rs`; the wasm run also exercises
//! cross-crate catalogue registration via module-init ctors.
//!
//! This file also holds the *emitting* half of the very-wide-message proof.
//! `integration_test_crate` defines a 500-field message under
//! `#![recursion_limit = "1024"]`; this crate deliberately keeps rustc's default
//! limit of 128 and emits it anyway. Do not add a `recursion_limit` attribute
//! here — its absence is the assertion.

use tracing_wide::{event, message};

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use wasm_bindgen_test::wasm_bindgen_test;

/// A message defined in *this* crate (so `origin().krate == "tracing-wide"`),
/// tagged for persistence — a distinct origin from the foreign crate's messages.
#[message(msg = "order placed", tags = ["persist"])]
struct OrderPlaced {
    id: usize,
}

#[allow(dead_code)] // only the `subscriber`-gated test calls this
fn place_order(id: usize) {
    event!(OrderPlaced { id });
}

#[cfg(feature = "subscriber")]
mod routing {
    use std::sync::{Arc, Mutex};

    use tracing_wide::Message;
    use tracing_wide::subscriber::{Subscriber, Subscribers};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::place_order;

    /// Type-agnostic sink: records `(origin crate, msg, tags)` for *every*
    /// message through the object-safe accessors — it never downcasts, so it
    /// observes the foreign crate's messages without naming their types.
    #[allow(clippy::type_complexity)]
    struct Recorder(Arc<Mutex<Vec<(&'static str, &'static str, Vec<&'static str>)>>>);

    impl Subscriber for Recorder {
        fn on_message(&self, m: &dyn Message) {
            self.0
                .lock()
                .unwrap()
                .push((m.origin().krate, m.msg(), m.tags().to_vec()));
        }
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn subscriber_sees_origin_and_tags_from_both_crates() {
        let log = Arc::new(Mutex::new(Vec::new()));

        let mut subs = Subscribers::default();
        subs.register(Box::new(Recorder(log.clone())));
        subs.install().ok();

        integration_test_crate::grant("alice");
        integration_test_crate::write("orders/42");

        place_order(42);

        let seen = log.lock().unwrap().clone();

        assert!(seen.contains(&("integration-test-crate", "access granted", vec!["security"])));
        assert!(seen.contains(&("integration-test-crate", "record written", vec!["persist"])));
        assert!(seen.contains(&("tracing-wide", "order placed", vec!["persist"])));

        let to_db: Vec<&str> = seen
            .iter()
            .filter(|(_, _, tags)| tags.contains(&"security"))
            .map(|(_, msg, _)| *msg)
            .collect();

        let to_store: Vec<&str> = seen
            .iter()
            .filter(|(_, _, tags)| tags.contains(&"persist"))
            .map(|(_, msg, _)| *msg)
            .collect();

        assert_eq!(to_db, ["access granted"]);
        assert!(to_store.contains(&"record written"));
        assert!(to_store.contains(&"order placed"));
    }
}

/// The emitting half of the very-wide-message proof. `integration_test_crate`
/// raised its own `recursion_limit` to define a 500-field message; this crate
/// did not, and still records every field — because `#[message]` expands
/// `tracing::info!` at the definition site, so only the defining crate spends
/// recursion budget. `tests/core.rs` pins the other side of this: what the
/// ceiling is when nobody raises the limit.
///
/// The collector is local rather than shared with `core.rs` via `common`: this
/// binary's catalogue registrations are asserted below, and pulling in
/// `common`'s message fixtures would change what it registers.
mod very_wide {
    use std::sync::{Arc, Mutex};

    use integration_test_crate::{VERY_WIDE_FIELDS, emit_very_wide};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Hands each event's fields to a fresh [`Grab`].
    #[allow(clippy::type_complexity)]
    struct Collector(Arc<Mutex<Vec<Vec<(String, String)>>>>);

    /// Collects one event's fields as `(name, rendered value)` pairs.
    struct Grab(Vec<(String, String)>);

    impl tracing::field::Visit for Grab {
        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.0.push((field.name().to_string(), value.to_string()));
        }

        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }

    impl tracing::Subscriber for Collector {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }

        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut grab = Grab(Vec::new());
            event.record(&mut grab);
            self.0.lock().unwrap().push(grab.0);
        }

        fn enter(&self, _: &tracing::span::Id) {}

        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn a_message_defined_under_a_raised_recursion_limit_round_trips_whole() {
        let events = Arc::new(Mutex::new(Vec::new()));

        tracing::subscriber::with_default(Collector(events.clone()), emit_very_wide);

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);

        let fields = &events[0];

        for i in 0..VERY_WIDE_FIELDS {
            let expected = (format!("f{i}"), i.to_string());
            assert!(
                fields.contains(&expected),
                "field {expected:?} is missing from a {VERY_WIDE_FIELDS}-field event"
            );
        }

        // Every field plus the static message text, and nothing else.
        assert_eq!(fields.len(), VERY_WIDE_FIELDS + 1);
        assert!(fields.contains(&("message".to_string(), "very wide event".to_string())));
    }
}

#[cfg(feature = "catalogue")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_registers_descriptors_from_both_crates() {
    use tracing_wide::catalogue;

    // Force-link the foreign crate: taking its fn pointers pulls in its
    // inventory registrations even in feature combos where no other (gated)
    // test references it.
    let _link: [fn(&'static str); 2] =
        [integration_test_crate::grant, integration_test_crate::write];

    let find = |msg: &str| {
        catalogue::all()
            .find(|d| d.msg == msg)
            .unwrap_or_else(|| panic!("no catalogue entry for {msg:?}"))
    };

    let granted = find("access granted");

    assert_eq!(granted.origin.krate, "integration-test-crate");
    assert_eq!(granted.tags.to_vec(), ["security"]);

    assert!(
        granted.origin.file.ends_with("lib.rs"),
        "file: {}",
        granted.origin.file
    );

    assert!(
        granted.origin.module.starts_with("integration_test_crate"),
        "module: {}",
        granted.origin.module
    );

    let order = find("order placed");

    assert_eq!(order.origin.krate, "tracing-wide");
    assert_eq!(order.tags.to_vec(), ["persist"]);
}
