//! Behavioral coverage for the feature-free core: `#[message]` consts, the
//! `Message` trait surface, and the `event!` → tracing handoff. The per-feature
//! suites (subscriber, catalogue, instrument, serde, facet) live in sibling test
//! files, so each compiles to its own binary and process-global state can't leak
//! between feature areas.
//!
//! Every test carries both `#[test]` and (on wasm) `#[wasm_bindgen_test]`, so
//! the same suite runs natively and on `wasm32-unknown-unknown`.

mod common;

use std::sync::{Arc, Mutex};

use common::{Charged, KvGrab, Renamed, Started};
use seq_macro::seq;
use tracing::Level;
use tracing_wide::{Message, event, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// `msg` containing format braces: the macro escapes them for tracing's
/// format-string position, so the recorded text matches `MSG` verbatim.
#[message(msg = "rate {limit} hit")]
#[derive(Default)]
struct Braced {
    n: usize,
}

/// A tracing subscriber that records each event's level and static message text.
struct EventCollector(Arc<Mutex<Vec<(Level, String)>>>);

/// Collects every recorded field of each event as `(name, rendered)` pairs.
#[allow(clippy::type_complexity)]
struct FieldCollector(Arc<Mutex<Vec<Vec<(String, String)>>>>);

/// Pulls the static message text (the `message` field) out of an event.
struct MessageGrab(Option<String>);

/// A minimal event that takes the default level (INFO).
#[message(msg = "plain event")]
#[derive(Default)]
struct Plain {
    code: usize,
}

seq!(N in 0..40 {
    /// Just past the 32 fields tracing's docs still claim as the limit.
    #[message(msg = "narrow wide event")]
    struct Narrower {
        #( f~N: usize, )*
    }

    impl Narrower {
        fn seeded() -> Self {
            Self { #( f~N: N, )* }
        }
    }
});

seq!(N in 0..100 {
    /// A very wide event, with recursion headroom to spare.
    #[message(msg = "wide event")]
    struct Wider {
        #( f~N: usize, )*
    }

    impl Wider {
        fn seeded() -> Self {
            Self { #( f~N: N, )* }
        }
    }
});

seq!(N in 0..117 {
    /// One field below the widest `seq!`-defined message that compiles at
    /// rustc's default `recursion_limit`.
    #[message(msg = "ceiling wide event")]
    struct EvenWider {
        #( f~N: usize, )*
    }

    impl EvenWider {
        fn seeded() -> Self {
            Self { #( f~N: N, )* }
        }
    }
});

impl tracing::Subscriber for EventCollector {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut grab = MessageGrab(None);
        event.record(&mut grab);
        self.0
            .lock()
            .unwrap()
            .push((*event.metadata().level(), grab.0.unwrap_or_default()));
    }

    fn enter(&self, _: &tracing::span::Id) {}

    fn exit(&self, _: &tracing::span::Id) {}
}

impl tracing::Subscriber for FieldCollector {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut grab = KvGrab(Vec::new());
        event.record(&mut grab);
        self.0.lock().unwrap().push(grab.0);
    }

    fn enter(&self, _: &tracing::span::Id) {}

    fn exit(&self, _: &tracing::span::Id) {}
}

impl tracing::field::Visit for MessageGrab {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn consts_reflect_message_attributes() {
    assert_eq!(Started::MSG, "service started");
    assert_eq!(Started::LEVEL, Level::WARN);
    assert_eq!(Started::TAGS.to_vec(), ["platform", "startup"]);
    assert_eq!(Started::ORIGIN.krate, "tracing-wide");
    assert!(Started::ORIGIN.file.ends_with("common.rs"));

    assert_eq!(Renamed::MSG, "Renamed");
    assert_eq!(Plain::LEVEL, Level::INFO);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn dyn_message_exposes_accessors_and_downcast() {
    let started = Started {
        service: "billing",
        attempt: 3,
        ..Default::default()
    };

    let dynamic: &dyn Message = &started;

    assert_eq!(dynamic.msg(), "service started");
    assert_eq!(dynamic.id(), "service started");
    assert_eq!(dynamic.level(), Level::WARN);
    assert_eq!(dynamic.tags().to_vec(), ["platform", "startup"]);
    assert_eq!(dynamic.origin().krate, "tracing-wide");

    let back = dynamic
        .as_any()
        .downcast_ref::<Started>()
        .expect("downcast to the concrete type");

    assert_eq!(back.service, "billing");
    assert_eq!(back.attempt, 3);

    // The routing case: a sink reads the stable id off `&dyn Message`, with no
    // catalogue join and no knowledge of the concrete type.
    let charged: &dyn Message = &Charged { cents: 500 };

    assert_eq!(charged.id(), "billing.payment_charged");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[allow(deprecated)]
fn emit_omits_a_none_deprecated_field_but_records_some() {
    let events = Arc::new(Mutex::new(Vec::new()));

    tracing::subscriber::with_default(FieldCollector(events.clone()), || {
        event!(Started {
            service: "billing",
            attempt: 1,
            ..Default::default()
        });
        event!(Started {
            service: "billing",
            attempt: 1,
            legacy_id: Some(42),
        });
    });

    let events = events.lock().unwrap();

    let names = |i: usize| {
        events[i]
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
    };

    assert!(
        !names(0).contains(&"legacy_id"),
        "a None deprecated field must be omitted from the event"
    );
    assert!(events[1].contains(&("legacy_id".to_string(), "42".to_string())));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn emit_records_a_braced_msg_verbatim() {
    assert_eq!(Braced::MSG, "rate {limit} hit");

    let log = Arc::new(Mutex::new(Vec::new()));

    tracing::subscriber::with_default(EventCollector(log.clone()), || {
        event!(Braced::default());
    });

    let got = log.lock().unwrap();
    assert_eq!(*got, vec![(Level::INFO, "rate {limit} hit".to_string())]);
}

/// Each rung of the width ladder records every one of its fields. See the note
/// on the fixtures for why 117 is the ceiling and what breaks if it moves.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn emit_records_every_field_of_a_message_wider_than_the_documented_limit() {
    /// Asserts a single recorded event carries exactly `width` indexed fields plus
    /// `message`, each `fN` holding the value `N`.
    fn assert_round_trip(events: &[Vec<(String, String)>], width: usize, msg: &str) {
        assert_eq!(events.len(), 1);

        let fields = &events[0];

        for i in 0..width {
            let expected = (format!("f{i}"), i.to_string());
            assert!(
                fields.contains(&expected),
                "field {expected:?} is missing from a {width}-field event"
            );
        }

        // Every field plus the static message text, and nothing else: a truncating
        // expansion would drop fields, a duplicating one would add them.
        assert_eq!(fields.len(), width + 1);
        assert!(fields.contains(&("message".to_string(), msg.to_string())));
    }

    let collect = |emit: &dyn Fn()| {
        let events = Arc::new(Mutex::new(Vec::new()));
        tracing::subscriber::with_default(FieldCollector(events.clone()), emit);
        Arc::try_unwrap(events).unwrap().into_inner().unwrap()
    };

    assert_round_trip(
        &collect(&|| event!(Narrower::seeded())),
        40,
        "narrow wide event",
    );

    assert_round_trip(&collect(&|| event!(Wider::seeded())), 100, "wide event");

    assert_round_trip(
        &collect(&|| event!(EvenWider::seeded())),
        117,
        "ceiling wide event",
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn emit_records_to_tracing_at_the_typed_level() {
    let log = Arc::new(Mutex::new(Vec::new()));

    tracing::subscriber::with_default(EventCollector(log.clone()), || {
        event!(Started {
            service: "billing",
            attempt: 1,
            ..Default::default()
        });
    });

    let got = log.lock().unwrap();
    assert_eq!(*got, vec![(Level::WARN, "service started".to_string())]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn emit_records_typed_field_values() {
    let events = Arc::new(Mutex::new(Vec::new()));

    tracing::subscriber::with_default(FieldCollector(events.clone()), || {
        event!(Started {
            service: "billing",
            attempt: 3,
            ..Default::default()
        });
    });

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);

    let fields = &events[0];

    assert!(fields.contains(&("service".to_string(), "billing".to_string())));
    assert!(fields.contains(&("attempt".to_string(), "3".to_string())));
    assert!(fields.contains(&("message".to_string(), "service started".to_string())));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn id_const_resolves_the_explicit_msg_type_name_chain() {
    assert_eq!(Charged::ID, "billing.payment_charged");
    assert_eq!(Charged::MSG, "payment charged");

    assert_eq!(Started::ID, "service started");
    assert_eq!(Renamed::ID, "Renamed");
}
