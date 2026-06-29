//! Coverage for ambient autocapture (`instrument` feature): a message's still-
//! `None` `Option` fields fill from the current span scope through the capture
//! layer, with the lossless/innermost/already-set/missing-layer edge cases.

#![cfg(feature = "instrument")]

mod common;

use std::sync::{Arc, Mutex};

use common::KvGrab;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_wide::{event, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Collects each tracing event's fields. A `Layer` (not a flat
/// `tracing::Subscriber`) because the ambient stack must be registry-based
/// for the join to reach span data.
#[allow(clippy::type_complexity)]
struct EventFields(Arc<Mutex<Vec<Vec<(String, String)>>>>);

/// `ratio` narrows losslessly from the span's widened `f64`; `skew` is a
/// genuine `f64` that `f32` can't represent, so it must stay `None`.
#[message(msg = "float event")]
#[derive(Default)]
struct Floats {
    ratio: Option<f32>,
    skew: Option<f32>,
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

impl<S: tracing::Subscriber> Layer<S> for EventFields {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        let mut grab = KvGrab(Vec::new());
        event.record(&mut grab);
        self.0.lock().unwrap().push(grab.0);
    }
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
fn join_fills_a_none_option_from_the_span_scope() {
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
fn join_keeps_an_existing_some_field() {
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

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn join_narrows_f32_only_losslessly() {
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
fn join_picks_the_innermost_span_on_a_collision() {
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
fn join_without_a_capture_layer_is_a_quiet_miss() {
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
