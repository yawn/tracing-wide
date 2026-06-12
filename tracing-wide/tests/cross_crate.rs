//! Cross-crate provenance & routing: a subscriber sees messages — with their
//! automatic `origin().krate` and their `tags()` — from BOTH this test crate
//! and the foreign `integration_test_crate`, without ever naming the foreign
//! types. Dual native/wasm, like `coverage.rs`; the wasm run also exercises
//! cross-crate catalogue registration via module-init ctors.

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
