//! Behavioral coverage for the tracing-wide public surface: `#[message]`,
//! `event!`, subscriber fan-out, the tracing handoff, and the catalogue.
//!
//! Every test carries both `#[test]` and (on wasm) `#[wasm_bindgen_test]`, so
//! the same suite runs natively and on `wasm32-unknown-unknown` — where the
//! catalogue descriptors are registered by ctor calls during module init
//! rather than before `main`.

use std::sync::{Arc, Mutex};

use tracing::Level;
#[cfg(feature = "subscriber")]
use tracing_wide::subscriber::{Subscriber, Subscribers};
use tracing_wide::{Message, event, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Used only by the subscriber fan-out test, so its assertions can't observe
/// events emitted by other tests sharing the global subscriber list.
#[cfg(feature = "subscriber")]
#[message(msg = "captured event", level = error)]
#[derive(Default)]
#[cfg_attr(feature = "facet", derive(tracing_wide::facet::Facet))]
struct CapturedEvent {
    n: usize,
}

/// Predecessor of [`Started`], kept to exercise message-level deprecation:
/// the note must land in the descriptor, and the generated impls must not
/// warn (only producer construction should).
#[cfg(feature = "catalogue")]
#[message(msg = "legacy started")]
#[deprecated = "use `service started` instead"]
#[allow(dead_code)]
struct LegacyStarted {
    n: usize,
}

/// A minimal event that takes the default level (INFO).
#[message(msg = "plain event")]
#[derive(Default)]
struct Plain {
    code: usize,
}

/// No explicit `msg`: the catalogue entry is keyed by the struct name.
#[message]
#[derive(Default)]
struct Renamed {
    x: usize,
}

/// `msg` containing format braces: the macro escapes them for tracing's
/// format-string position, so the recorded text matches `MSG` verbatim.
#[message(msg = "rate {limit} hit")]
#[derive(Default)]
struct Braced {
    n: usize,
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

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn message_consts_reflect_attributes() {
    assert_eq!(Started::MSG, "service started");
    assert_eq!(Started::LEVEL, Level::WARN);
    assert_eq!(Started::TAGS.to_vec(), ["platform", "startup"]);
    assert_eq!(Started::ORIGIN.krate, "tracing-wide");
    assert!(Started::ORIGIN.file.ends_with("coverage.rs"));

    assert_eq!(Renamed::MSG, "Renamed");
    assert_eq!(Plain::LEVEL, Level::INFO);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn message_trait_accessors_and_downcast() {
    let started = Started {
        service: "billing",
        attempt: 3,
        ..Default::default()
    };

    let dynamic: &dyn Message = &started;

    assert_eq!(dynamic.msg(), "service started");
    assert_eq!(dynamic.level(), Level::WARN);
    assert_eq!(dynamic.tags().to_vec(), ["platform", "startup"]);
    assert_eq!(dynamic.origin().krate, "tracing-wide");

    let back = dynamic
        .as_any()
        .downcast_ref::<Started>()
        .expect("downcast to the concrete type");

    assert_eq!(back.service, "billing");
    assert_eq!(back.attempt, 3);
}

/// Filters to its dedicated type so concurrently-emitted events from other
/// tests can't leak into this recorder.
#[cfg(feature = "subscriber")]
#[allow(clippy::type_complexity)]
struct Recorder(Arc<Mutex<Vec<(&'static str, Level, usize)>>>);

#[cfg(feature = "subscriber")]
impl Subscriber for Recorder {
    fn on_message(&self, m: &dyn Message) {
        if let Some(e) = m.as_any().downcast_ref::<CapturedEvent>() {
            self.0.lock().unwrap().push((m.msg(), m.level(), e.n));
        }
    }
}

/// A second subscriber that only notes *that* it was reached, into a shared
/// ordered log — so the test can prove fan-out hits more than one sink and can
/// observe dispatch-vs-tracing ordering.
#[cfg(feature = "subscriber")]
struct OrderMarker(Arc<Mutex<Vec<&'static str>>>);

#[cfg(feature = "subscriber")]
impl Subscriber for OrderMarker {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<CapturedEvent>().is_some() {
            self.0.lock().unwrap().push("dispatch");
        }
    }
}

/// A tracing subscriber that notes the handoff into the *same* shared log as
/// [`OrderMarker`], so their relative order is observable.
#[cfg(feature = "subscriber")]
struct OrderTracing(Arc<Mutex<Vec<&'static str>>>);

#[cfg(feature = "subscriber")]
impl tracing::Subscriber for OrderTracing {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}

    fn event(&self, _: &tracing::Event<'_>) {
        self.0.lock().unwrap().push("record");
    }

    fn enter(&self, _: &tracing::span::Id) {}

    fn exit(&self, _: &tracing::span::Id) {}
}

/// Exercises the `facet` hook *inside* a subscriber — the path the
/// `subscriber-facet` example shows, here under test on native and wasm. Filters to its dedicated
/// type for the same isolation reason as [`Recorder`], then reads a field by
/// name through reflection rather than the concrete type.
#[cfg(all(feature = "subscriber", feature = "facet"))]
struct FacetProbe(Arc<Mutex<Vec<usize>>>);

#[cfg(all(feature = "subscriber", feature = "facet"))]
impl Subscriber for FacetProbe {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<CapturedEvent>().is_none() {
            return;
        }

        if let Some(peek) = m.as_facet()
            && let Ok(body) = peek.into_struct()
            && let Ok(field) = body.field_by_name("n")
            && let Ok(n) = field.get::<usize>()
        {
            self.0.lock().unwrap().push(*n);
        }
    }
}

/// The subscriber registry is a process-global `OnceLock`, installable exactly
/// once — so every assertion that needs the global lives in this single test:
/// fan-out to more than one subscriber, the set-once `install` contract, and
/// the dispatch-before-tracing ordering the typed-primary design promises.
#[cfg(feature = "subscriber")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn subscriber_fanout_is_set_once_and_precedes_tracing() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let order = Arc::new(Mutex::new(Vec::new()));
    #[cfg(feature = "facet")]
    let reflected = Arc::new(Mutex::new(Vec::new()));

    let mut subscribers = Subscribers::default();
    subscribers.register(Box::new(Recorder(captured.clone())));
    subscribers.register(Box::new(OrderMarker(order.clone())));
    #[cfg(feature = "facet")]
    subscribers.register(Box::new(FacetProbe(reflected.clone())));

    assert!(subscribers.install().is_ok(), "first install wins");
    assert!(
        Subscribers::default().install().is_err(),
        "the registry installs exactly once"
    );

    tracing::subscriber::with_default(OrderTracing(order.clone()), || {
        event!(CapturedEvent { n: 7 });
    });

    assert_eq!(
        *captured.lock().unwrap(),
        vec![("captured event", Level::ERROR, 7)]
    );

    assert_eq!(*order.lock().unwrap(), vec!["dispatch", "record"]);

    // The facet hook resolved the message's `n` field through `&dyn Message`.
    #[cfg(feature = "facet")]
    assert_eq!(*reflected.lock().unwrap(), vec![7]);
}

struct EventCollector(Arc<Mutex<Vec<(Level, String)>>>);

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

/// Pulls the static message text (the `message` field) out of an event.
struct MessageGrab(Option<String>);

impl tracing::field::Visit for MessageGrab {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = Some(format!("{value:?}"));
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn event_records_to_tracing_at_the_typed_level() {
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
fn braces_in_msg_record_verbatim() {
    assert_eq!(Braced::MSG, "rate {limit} hit");

    let log = Arc::new(Mutex::new(Vec::new()));

    tracing::subscriber::with_default(EventCollector(log.clone()), || {
        event!(Braced::default());
    });

    let got = log.lock().unwrap();
    assert_eq!(*got, vec![(Level::INFO, "rate {limit} hit".to_string())]);
}

/// Collects every recorded field of each event as `(name, rendered)` pairs.
#[allow(clippy::type_complexity)]
struct FieldCollector(Arc<Mutex<Vec<Vec<(String, String)>>>>);

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

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn event_records_typed_field_values_to_tracing() {
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
#[allow(deprecated)]
fn deprecated_none_field_is_omitted_but_some_is_recorded() {
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

/// Collects event fields as `(name, rendered value)` pairs.
struct KvGrab(Vec<(String, String)>);

impl tracing::field::Visit for KvGrab {
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

#[cfg(feature = "catalogue")]
fn descriptor(msg: &str) -> &'static tracing_wide::catalogue::MessageDescriptor {
    tracing_wide::catalogue::all()
        .find(|d| d.msg == msg)
        .unwrap_or_else(|| panic!("no catalogue entry for {msg:?}"))
}

#[cfg(feature = "catalogue")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_registers_message_descriptors() {
    let d = descriptor("service started");

    assert_eq!(d.level.as_str(), "WARN");
    assert_eq!(Level::from(d.level), Level::WARN);
    assert_eq!(d.doc, Some("Emitted when a service finishes starting up."));
    assert_eq!(d.deprecated, None);
    assert!(d.meta.contains(&("owner", "platform")));
    assert_eq!(d.tags.to_vec(), ["platform", "startup"]);
    assert_eq!(d.origin.krate, "tracing-wide");
    assert!(d.origin.file.ends_with("coverage.rs"));

    let names: Vec<&str> = d.fields.iter().map(|f| f.name).collect();

    assert_eq!(names, ["attempt", "legacy_id", "service"]);

    let service = d.fields.iter().find(|f| f.name == "service").unwrap();

    assert_eq!(service.r#type, "& 'static str");
    assert_eq!(service.doc, Some("Logical name of the service."));
    assert_eq!(service.deprecated, None);

    let attempt = d.fields.iter().find(|f| f.name == "attempt").unwrap();

    assert!(attempt.meta.contains(&("unit", "count")));

    let legacy = d.fields.iter().find(|f| f.name == "legacy_id").unwrap();

    assert_eq!(legacy.deprecated, Some("fold the id into `service`"));
    assert_eq!(legacy.r#type, "Option < usize >");

    assert_eq!(Level::from(descriptor("Renamed").level), Level::INFO);

    assert_eq!(
        descriptor("legacy started").deprecated,
        Some("use `service started` instead")
    );
}

/// Message types deliberately colliding on `msg` — only ever registered, so
/// they poison nothing but the duplicate check they exist to exercise. The
/// `Dup*` triple shares one msg (dedup: one report, not one per extra
/// registration); the `Alt*` pair shares another (multiple distinct collisions
/// are all reported).
#[cfg(feature = "catalogue")]
#[message(msg = "also duplicated")]
#[allow(dead_code)]
struct AltA {
    a: usize,
}

#[cfg(feature = "catalogue")]
#[message(msg = "also duplicated")]
#[allow(dead_code)]
struct AltB {
    b: usize,
}

#[cfg(feature = "catalogue")]
#[message(msg = "duplicated msg")]
#[allow(dead_code)]
struct DupA {
    a: usize,
}

#[cfg(feature = "catalogue")]
#[message(msg = "duplicated msg")]
#[allow(dead_code)]
struct DupB {
    b: usize,
}

#[cfg(feature = "catalogue")]
#[message(msg = "duplicated msg")]
#[allow(dead_code)]
struct DupC {
    c: usize,
}

/// Exactly the planted collisions, sorted, one entry per colliding msg; every
/// other fixture's msg is unique, so this also asserts unique msgs aren't flagged.
#[cfg(feature = "catalogue")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn catalogue_reports_duplicate_msgs() {
    assert_eq!(
        tracing_wide::catalogue::duplicates(),
        vec!["also duplicated", "duplicated msg"]
    );
}

#[cfg(feature = "instrument")]
mod instrument {
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
    use tracing_wide::{event, message};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::KvGrab;

    /// Collects each tracing event's fields. A `Layer` (not a flat
    /// `tracing::Subscriber`) because the ambient stack must be registry-based
    /// for the join to reach span data.
    #[allow(clippy::type_complexity)]
    struct EventFields(Arc<Mutex<Vec<Vec<(String, String)>>>>);

    impl<S: tracing::Subscriber> Layer<S> for EventFields {
        fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
            let mut grab = KvGrab(Vec::new());
            event.record(&mut grab);
            self.0.lock().unwrap().push(grab.0);
        }
    }

    /// Required `payload` from the literal; ambient `component` (string) and
    /// `attempt` (lossless span `u64` → field `usize`) joined from the span scope.
    #[message(msg = "ambient event")]
    #[derive(Default)]
    struct Joined {
        attempt: Option<usize>,
        component: Option<String>,
        payload: usize,
    }

    /// Run `f` under registry + capture layer + event collector; return the
    /// fields of every tracing event it emitted.
    fn collect(f: impl FnOnce()) -> Vec<Vec<(String, String)>> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry()
            .with(tracing_wide::instrument::layer())
            .with(EventFields(events.clone()));

        tracing::subscriber::with_default(subscriber, f);

        events.lock().unwrap().clone()
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn option_fields_fill_from_the_span_scope() {
        let events = collect(|| {
            let span = tracing::info_span!("req", component = "billing", attempt = 3usize);
            let _enter = span.enter();
            event!(Joined {
                payload: 1,
                ..Default::default()
            });
        });

        assert_eq!(events.len(), 1);
        assert!(events[0].contains(&("component".to_string(), "billing".to_string())));
        assert!(events[0].contains(&("attempt".to_string(), "3".to_string())));
        assert!(events[0].contains(&("payload".to_string(), "1".to_string())));
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn innermost_span_wins_a_name_collision() {
        let events = collect(|| {
            let outer = tracing::info_span!("outer", component = "outer");
            let _outer = outer.enter();

            let inner = tracing::info_span!("inner", component = "inner");
            let _inner = inner.enter();

            event!(Joined {
                payload: 1,
                ..Default::default()
            });
        });

        assert!(events[0].contains(&("component".to_string(), "inner".to_string())));
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn a_some_field_is_never_overwritten() {
        let events = collect(|| {
            let span = tracing::info_span!("req", component = "ambient");
            let _enter = span.enter();

            event!(Joined {
                payload: 1,
                component: Some("local".to_string()),
                ..Default::default()
            });
        });

        assert!(events[0].contains(&("component".to_string(), "local".to_string())));
    }

    /// `ratio` narrows losslessly from the span's widened `f64`; `skew` is a
    /// genuine `f64` that `f32` can't represent, so it must stay `None`.
    #[message(msg = "float event")]
    #[derive(Default)]
    struct Floats {
        ratio: Option<f32>,
        skew: Option<f32>,
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn f32_fields_fill_only_losslessly() {
        let events = collect(|| {
            let span = tracing::info_span!("req", ratio = 0.5_f32, skew = 0.1_f64);
            let _enter = span.enter();

            event!(Floats::default());
        });

        assert_eq!(events.len(), 1);
        assert!(events[0].contains(&("ratio".to_string(), "0.5".to_string())));

        let names: Vec<&str> = events[0].iter().map(|(n, _)| n.as_str()).collect();

        assert!(!names.contains(&"skew"), "a lossy f64→f32 must miss");
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn missing_capture_layer_is_a_quiet_miss() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(EventFields(events.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("req", component = "billing");
            let _enter = span.enter();

            event!(Joined {
                payload: 1,
                ..Default::default()
            });
        });

        let events = events.lock().unwrap();
        let names: Vec<&str> = events[0].iter().map(|(n, _)| n.as_str()).collect();

        assert!(
            !names.contains(&"component"),
            "a missed join must leave the field None/omitted"
        );
        assert!(names.contains(&"payload"));
    }
}

#[cfg(feature = "serde")]
mod serialize {
    use serde::Serialize;
    use tracing_wide::{Message, message};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Does *not* opt in — and isn't `Serialize`. Proves the hook never forces
    /// a `Serialize` bound on messages that didn't ask for it.
    #[message(msg = "opaque event")]
    #[allow(dead_code)]
    struct Opaque {
        n: usize,
    }

    /// Enables the erased hook with nothing but `#[derive(Serialize)]`.
    #[message(msg = "serializable event")]
    #[derive(Serialize)]
    struct Ser {
        n: usize,
        name: &'static str,
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn opted_in_message_serializes_through_dyn_message() {
        let m = Ser {
            name: "billing",
            n: 5,
        };

        let dynamic: &dyn Message = &m;

        let erased = dynamic
            .as_serialize()
            .expect("an opted-in message yields Some");

        let value = serde_json::to_value(erased).unwrap();

        assert_eq!(value, serde_json::json!({ "name": "billing", "n": 5 }));
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn opted_out_message_returns_none() {
        let m = Opaque { n: 1 };
        let dynamic: &dyn Message = &m;

        assert!(dynamic.as_serialize().is_none());
    }
}

#[cfg(feature = "facet")]
mod reflect {
    use tracing_wide::{
        Message,
        facet::{Facet, HasFields},
        message,
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Does *not* opt in — and isn't `Facet`. Proves the hook never forces a
    /// `Facet` bound on a message that didn't ask for it.
    #[message(msg = "opaque reflect event")]
    #[allow(dead_code)]
    struct Opaque {
        n: usize,
    }

    /// Enables the reflection hook with nothing but `#[derive(Facet)]`.
    #[message(msg = "reflectable event")]
    #[derive(Facet)]
    struct Reflect {
        n: usize,
        name: &'static str,
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn opted_in_message_reflects_through_dyn_message() {
        let m = Reflect {
            n: 5,
            name: "billing",
        };

        let dynamic: &dyn Message = &m;

        let body = dynamic
            .as_facet()
            .expect("an opted-in message yields Some")
            .into_struct()
            .expect("a message is a struct");

        // Read individual fields by name, with no knowledge of the concrete type.
        assert_eq!(
            body.field_by_name("name").unwrap().as_str(),
            Some("billing")
        );
        assert_eq!(*body.field_by_name("n").unwrap().get::<usize>().unwrap(), 5);

        // Full reflection: every field, in declaration order.
        let names: Vec<&str> = body.fields().map(|(f, _)| f.name).collect();
        assert_eq!(names, ["n", "name"]);
    }

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn opted_out_message_returns_none() {
        let m = Opaque { n: 1 };
        let dynamic: &dyn Message = &m;

        assert!(dynamic.as_facet().is_none());
    }
}

// Native-only, like the serde catalogue manifest (serde_norway / facet-json are
// both non-wasm dev-deps): the descriptors serialize the same under both frameworks.
#[cfg(all(
    feature = "catalogue",
    feature = "facet",
    feature = "serde",
    not(target_arch = "wasm32")
))]
mod catalogue_facet {
    use tracing_wide::message;

    /// Exercises every customized descriptor field: struct deprecation, field
    /// docs, meta (map), level (name), tags, and origin (compact string).
    #[message(msg = "facet catalogue probe", level = warn, owner = "platform", tags = ["b", "a"])]
    #[deprecated = "demo"]
    #[allow(dead_code)]
    struct Probe {
        /// A documented field.
        #[field(unit = "ms")]
        duration: usize,
    }

    /// The facet manifest must match the serde manifest field-for-field — the
    /// whole point of the proxies (compact origin, map meta, level name).
    #[test]
    fn descriptor_serializes_identically_under_serde_and_facet() {
        let descriptor = tracing_wide::catalogue::all()
            .find(|d| d.msg == "facet catalogue probe")
            .expect("Probe is registered");

        let via_serde = serde_json::to_value(descriptor).unwrap();
        let via_facet: serde_json::Value =
            serde_json::from_str(&facet_json::to_string(descriptor).unwrap()).unwrap();

        assert_eq!(via_serde, via_facet);
    }
}
