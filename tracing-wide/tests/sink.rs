//! The sink registry: `install`, thread-local scopes, fan-out, order, meta routing, and the `facet` hook in a sink.
//! Each test scopes its sinks to its own thread to run isolated; only one test installs the global set.
#![cfg(feature = "sink")]

use core::marker::PhantomData;
use std::sync::{Arc, Mutex};

use tracing::Level;
use tracing_wide::sink::{Sink, Sinks};
use tracing_wide::{Message, event, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Carries `owner = "billing"`, the pair that `MetaRouter` selects.
#[message(msg = "invoice settled", owner = "billing")]
#[derive(Default)]
struct BillingEvent {}

/// Emitted only inside thread-local scopes, so it never reaches the sinks of another test.
#[message(msg = "captured event", level = error)]
#[derive(Default)]
#[cfg_attr(feature = "facet", derive(tracing_wide::facet::Facet))]
struct CapturedEvent {
    n: usize,
}

/// Reads `n` of a [`CapturedEvent`] by name through the `facet` hook.
#[cfg(feature = "facet")]
struct FacetProbe(Arc<Mutex<Vec<usize>>>);

/// Emitted only by the test that installs the global set.
#[message(msg = "global event")]
#[derive(Default)]
struct GlobalEvent {}

/// Keeps the `msg` of each message whose meta has `owner = "billing"`.
struct MetaRouter(Arc<Mutex<Vec<&'static str>>>);

/// Emitted only under `with_default(OrderTracing)`: tracing caches callsite interest globally, so an emit without a
/// tracing subscriber disables the callsite for the test that runs second.
#[message(msg = "ordered event", level = error)]
#[derive(Default)]
struct OrderedEvent {}

/// A second sink that only logs that dispatch reached it.
struct OrderMarker(Arc<Mutex<Vec<&'static str>>>);

/// A tracing subscriber that logs `"record"` for each event.
struct OrderTracing(Arc<Mutex<Vec<&'static str>>>);

/// The message of the scope tests and the meta routing test; [`ReentrantEmitter`] answers it with a [`Pong`].
#[message(msg = "ping")]
#[derive(Default)]
struct Ping {}

/// Carries `owner = "platform"`, which `MetaRouter` must skip.
#[message(msg = "node drained", owner = "platform")]
#[derive(Default)]
struct PlatformEvent {}

#[message(msg = "pong")]
#[derive(Default)]
struct Pong {}

/// Records only [`CapturedEvent`], so the events of concurrent tests do not leak in.
#[allow(clippy::type_complexity)]
struct Recorder(Arc<Mutex<Vec<(&'static str, Level, usize)>>>);

/// On a [`Ping`], emits a [`Pong`] from `on_message`; the lock guard is a temporary, so the nested emit can lock.
struct ReentrantEmitter(Arc<Mutex<Vec<&'static str>>>);

/// Logs its label for each message of type `M`.
struct Tagger<M>(
    &'static str,
    Arc<Mutex<Vec<&'static str>>>,
    PhantomData<fn() -> M>,
);

impl<M> Tagger<M> {
    fn new(label: &'static str, log: Arc<Mutex<Vec<&'static str>>>) -> Self {
        Tagger(label, log, PhantomData)
    }
}

#[cfg(feature = "facet")]
impl Sink for FacetProbe {
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

impl Sink for MetaRouter {
    fn on_message(&self, m: &dyn Message) {
        if m.meta().get("owner") == Some("billing") {
            self.0.lock().unwrap().push(m.msg());
        }
    }
}

impl Sink for OrderMarker {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<CapturedEvent>().is_some() {
            self.0.lock().unwrap().push("dispatch");
        }
    }
}

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

impl Sink for Recorder {
    fn on_message(&self, m: &dyn Message) {
        if let Some(e) = m.as_any().downcast_ref::<CapturedEvent>() {
            self.0.lock().unwrap().push((m.msg(), m.level(), e.n));
        }
    }
}

impl Sink for ReentrantEmitter {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<Ping>().is_some() {
            self.0.lock().unwrap().push("ping");
            event!(Pong::default());
        } else if m.as_any().downcast_ref::<Pong>().is_some() {
            self.0.lock().unwrap().push("pong");
        }
    }
}

impl<M: 'static> Sink for Tagger<M> {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<M>().is_some() {
            self.1.lock().unwrap().push(self.0);
        }
    }
}

/// Dispatch reaches every registered sink.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn dispatch_fans_out_to_every_sink() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let order = Arc::new(Mutex::new(Vec::new()));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(Recorder(captured.clone())));
    sinks.register(Box::new(OrderMarker(order.clone())));

    sinks.with_default(|| {
        event!(CapturedEvent { n: 7 });
    });

    assert_eq!(
        *captured.lock().unwrap(),
        vec![("captured event", Level::ERROR, 7)]
    );
    assert_eq!(*order.lock().unwrap(), vec!["dispatch"]);
}

/// The fan-out runs before the tracing handoff.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn dispatch_precedes_the_tracing_handoff() {
    let order = Arc::new(Mutex::new(Vec::new()));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(Tagger::<OrderedEvent>::new(
        "dispatch",
        order.clone(),
    )));

    sinks.with_default(|| {
        tracing::subscriber::with_default(OrderTracing(order.clone()), || {
            event!(OrderedEvent::default());
        });
    });

    assert_eq!(*order.lock().unwrap(), vec!["dispatch", "record"]);
}

/// A sink reads a field by name through the `facet` hook on `&dyn Message`.
#[cfg(feature = "facet")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn facet_hook_reads_a_field_through_dyn_message() {
    let reflected = Arc::new(Mutex::new(Vec::new()));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(FacetProbe(reflected.clone())));

    sinks.with_default(|| {
        event!(CapturedEvent { n: 7 });
    });

    assert_eq!(*reflected.lock().unwrap(), vec![7]);
}

/// `install` succeeds once, and a `with_default` scope replaces the global set until it ends. Only this test installs
/// the global set, so the set-once assertion does not depend on the test order.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn global_install_is_set_once_and_a_scope_overrides_it() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut global = Sinks::default();
    global.register(Box::new(Tagger::<GlobalEvent>::new("global", log.clone())));

    assert!(global.install().is_ok(), "first install wins");
    assert!(
        Sinks::default().install().is_err(),
        "the registry installs exactly once"
    );

    event!(GlobalEvent::default()); // -> global

    let mut scoped = Sinks::default();
    scoped.register(Box::new(Tagger::<GlobalEvent>::new("scoped", log.clone())));
    scoped.with_default(|| {
        event!(GlobalEvent::default()); // -> scoped, overriding the global
    });

    event!(GlobalEvent::default()); // -> global again, scope ended

    assert_eq!(*log.lock().unwrap(), vec!["global", "scoped", "global"]);
}

/// A sink routes on `#[message(owner = ...)]` read off a live `&dyn Message`, with no `catalogue` join.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn meta_routes_a_live_message_with_no_catalogue_join() {
    let routed = Arc::new(Mutex::new(Vec::new()));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(MetaRouter(routed.clone())));

    sinks.with_default(|| {
        event!(BillingEvent {});
        event!(PlatformEvent {});
        event!(Ping {});
    });

    assert_eq!(*routed.lock().unwrap(), vec!["invoice settled"]);
}

/// A sink that emits from `on_message` re-enters dispatch and reaches the same set.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn scoped_dispatch_is_reentrant() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(ReentrantEmitter(log.clone())));

    sinks.with_default(|| {
        event!(Ping::default());
    });

    assert_eq!(*log.lock().unwrap(), vec!["ping", "pong"]);
}

/// Nested `set_default` guards restore the displaced set on drop.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn set_default_guards_restore_the_previous_scope() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut outer = Sinks::default();
    outer.register(Box::new(Tagger::<Ping>::new("outer", log.clone())));
    let outer_guard = outer.set_default();

    event!(Ping::default()); // -> outer

    {
        let mut inner = Sinks::default();
        inner.register(Box::new(Tagger::<Ping>::new("inner", log.clone())));
        let _inner_guard = inner.set_default();
        event!(Ping::default()); // -> inner
    } // inner guard dropped here, restoring outer

    event!(Ping::default()); // -> outer again

    drop(outer_guard);
    event!(Ping::default()); // -> no scope; not recorded by either tagger

    assert_eq!(*log.lock().unwrap(), vec!["outer", "inner", "outer"]);
}

/// `with_default` scopes the set to the closure and returns its value.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn with_default_scopes_to_the_closure_and_returns_its_value() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(Tagger::<Ping>::new("scoped", log.clone())));

    let out = sinks.with_default(|| {
        event!(Ping::default());
        "result"
    });

    assert_eq!(out, "result");
    assert_eq!(*log.lock().unwrap(), vec!["scoped"]);

    // The scope ended, so this emit is not logged.
    event!(Ping::default());
    assert_eq!(*log.lock().unwrap(), vec!["scoped"]);
}
