//! Coverage for the app-level subscriber registry (`subscriber` feature): the
//! global set-once `install`, thread-local `set_default`/`with_default` scoping,
//! fan-out, the dispatch-before-tracing ordering, and the `facet` reflection
//! hook used from inside a subscriber.
//!
//! Each test that needs only thread-local scoping stands up its own subscribers
//! via `with_default`, so they run isolated and in parallel; the single global
//! `install` lives in one test, which also shows a scope overriding it.
#![cfg(feature = "subscriber")]

use core::marker::PhantomData;
use std::sync::{Arc, Mutex};

use tracing::Level;
use tracing_wide::subscriber::{Subscriber, Subscribers};
use tracing_wide::{Message, event, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Emitted (only inside thread-local scopes) by the fan-out and facet tests; the
/// scope keeps each test's emit off the global registry and out of the other
/// tests' subscribers.
#[message(msg = "captured event", level = error)]
#[derive(Default)]
#[cfg_attr(feature = "facet", derive(tracing_wide::facet::Facet))]
struct CapturedEvent {
    n: usize,
}

/// Exercises the `facet` hook *inside* a subscriber — the path the
/// `subscriber-facet` example shows, here under test on native and wasm. Filters
/// to its dedicated type for the same isolation reason as [`Recorder`], then
/// reads a field by name through reflection rather than the concrete type.
#[cfg(feature = "facet")]
struct FacetProbe(Arc<Mutex<Vec<usize>>>);

/// Emitted only by the global-registry test, always against the installed global
/// (never inside another test's scope), so its dispatches stay isolated from the
/// thread-local tests.
#[message(msg = "global event")]
#[derive(Default)]
struct GlobalEvent {}

/// Emitted only by the ordering test, inside `with_default(OrderTracing)`, so
/// its tracing callsite's interest is first computed under that subscriber.
/// tracing caches callsite interest process-globally, so a type also emitted
/// with no tracing subscriber active would cache the callsite as disabled and
/// silently drop the handoff in whichever test ran second.
#[message(msg = "ordered event", level = error)]
#[derive(Default)]
struct OrderedEvent {}

/// A second subscriber that only notes *that* it was reached, so the fan-out
/// test can prove dispatch hits more than one sink.
struct OrderMarker(Arc<Mutex<Vec<&'static str>>>);

/// A tracing subscriber that notes the handoff into the *same* shared log as
/// the tracing-wide dispatch marker, so their relative order is observable.
struct OrderTracing(Arc<Mutex<Vec<&'static str>>>);

/// Emitted only inside a thread-local scope by the `set_default`/`with_default`
/// tests, so it never reaches another test's subscribers. `Pong` is re-emitted
/// from within a subscriber to exercise re-entrant dispatch.
#[message(msg = "ping")]
#[derive(Default)]
struct Ping {}

#[message(msg = "pong")]
#[derive(Default)]
struct Pong {}

/// Filters to its dedicated type so concurrently-emitted events from other
/// tests can't leak into this recorder.
#[allow(clippy::type_complexity)]
struct Recorder(Arc<Mutex<Vec<(&'static str, Level, usize)>>>);

/// On a [`Ping`], re-emits a [`Pong`] from inside `on_message` — the re-entrant
/// dispatch path. The lock is released (the guard is a statement temporary)
/// before the nested emit, so the second dispatch can take it again.
struct ReentrantEmitter(Arc<Mutex<Vec<&'static str>>>);

/// Records its label for every message of type `M` it sees — a test proves
/// which set was active by which labels land in the shared log. Generic over `M`
/// so each test keys its taggers to a type only it emits.
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

impl Subscriber for OrderMarker {
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

impl Subscriber for Recorder {
    fn on_message(&self, m: &dyn Message) {
        if let Some(e) = m.as_any().downcast_ref::<CapturedEvent>() {
            self.0.lock().unwrap().push((m.msg(), m.level(), e.n));
        }
    }
}

impl Subscriber for ReentrantEmitter {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<Ping>().is_some() {
            self.0.lock().unwrap().push("ping");
            event!(Pong::default());
        } else if m.as_any().downcast_ref::<Pong>().is_some() {
            self.0.lock().unwrap().push("pong");
        }
    }
}

impl<M: 'static> Subscriber for Tagger<M> {
    fn on_message(&self, m: &dyn Message) {
        if m.as_any().downcast_ref::<M>().is_some() {
            self.1.lock().unwrap().push(self.0);
        }
    }
}

/// Dispatch fans a message out to *every* registered subscriber, not just the
/// first — two sinks both record the single emit. Scoped via `with_default` so
/// it runs isolated from the global registry and the other subscriber tests.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn dispatch_fans_out_to_every_subscriber() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let order = Arc::new(Mutex::new(Vec::new()));

    let mut subs = Subscribers::default();
    subs.register(Box::new(Recorder(captured.clone())));
    subs.register(Box::new(OrderMarker(order.clone())));

    subs.with_default(|| {
        event!(CapturedEvent { n: 7 });
    });

    assert_eq!(
        *captured.lock().unwrap(),
        vec![("captured event", Level::ERROR, 7)]
    );
    assert_eq!(*order.lock().unwrap(), vec!["dispatch"]);
}

/// The subscriber fan-out runs *before* the tracing handoff — the typed-primary
/// promise — observed by funnelling a tracing-wide subscriber and a tracing
/// subscriber into one ordered log.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn dispatch_precedes_the_tracing_handoff() {
    let order = Arc::new(Mutex::new(Vec::new()));

    let mut subs = Subscribers::default();
    subs.register(Box::new(Tagger::<OrderedEvent>::new(
        "dispatch",
        order.clone(),
    )));

    subs.with_default(|| {
        tracing::subscriber::with_default(OrderTracing(order.clone()), || {
            event!(OrderedEvent::default());
        });
    });

    assert_eq!(*order.lock().unwrap(), vec!["dispatch", "record"]);
}

/// A subscriber reads a field by name through the `facet` hook on `&dyn Message`
/// — reflection rather than the concrete type.
#[cfg(feature = "facet")]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn facet_hook_reads_a_field_through_dyn_message() {
    let reflected = Arc::new(Mutex::new(Vec::new()));

    let mut subs = Subscribers::default();
    subs.register(Box::new(FacetProbe(reflected.clone())));

    subs.with_default(|| {
        event!(CapturedEvent { n: 7 });
    });

    assert_eq!(*reflected.lock().unwrap(), vec![7]);
}

/// The global registry is a process-global `OnceLock`: `install` succeeds once,
/// a second attempt is rejected, and while a thread-local scope (`with_default`)
/// is held it *replaces* the global, which resumes once the scope ends. The
/// `["global", "scoped", "global"]` shape mirrors
/// [`set_default_guards_restore_the_previous_scope`], rooted at the global
/// instead of an outer scope — and this is the one test that owns the global, so
/// the set-once assertion is deterministic regardless of run order.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn global_install_is_set_once_and_a_scope_overrides_it() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut global = Subscribers::default();
    global.register(Box::new(Tagger::<GlobalEvent>::new("global", log.clone())));

    assert!(global.install().is_ok(), "first install wins");
    assert!(
        Subscribers::default().install().is_err(),
        "the registry installs exactly once"
    );

    event!(GlobalEvent::default()); // -> global

    let mut scoped = Subscribers::default();
    scoped.register(Box::new(Tagger::<GlobalEvent>::new("scoped", log.clone())));
    scoped.with_default(|| {
        event!(GlobalEvent::default()); // -> scoped, overriding the global
    });

    event!(GlobalEvent::default()); // -> global again, scope ended

    assert_eq!(*log.lock().unwrap(), vec!["global", "scoped", "global"]);
}

/// A subscriber that emits from inside `on_message` re-enters dispatch. The
/// scoped set is cloned out of the cell before fan-out, so the nested emit is
/// safe and reaches the same set.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn scoped_dispatch_is_reentrant() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut subs = Subscribers::default();
    subs.register(Box::new(ReentrantEmitter(log.clone())));

    subs.with_default(|| {
        event!(Ping::default());
    });

    assert_eq!(*log.lock().unwrap(), vec!["ping", "pong"]);
}

/// Nested `set_default` scopes stack: each guard restores the set it displaced
/// on drop, so the active set tracks guard lifetime.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn set_default_guards_restore_the_previous_scope() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut outer = Subscribers::default();
    outer.register(Box::new(Tagger::<Ping>::new("outer", log.clone())));
    let outer_guard = outer.set_default();

    event!(Ping::default()); // -> outer

    {
        let mut inner = Subscribers::default();
        inner.register(Box::new(Tagger::<Ping>::new("inner", log.clone())));
        let _inner_guard = inner.set_default();
        event!(Ping::default()); // -> inner
    } // inner guard dropped here, restoring outer

    event!(Ping::default()); // -> outer again

    drop(outer_guard);
    event!(Ping::default()); // -> no scope; not recorded by either tagger

    assert_eq!(*log.lock().unwrap(), vec!["outer", "inner", "outer"]);
}

/// `with_default` scopes the set to the closure, hands back the closure's value,
/// and restores the previous (here empty) scope after — an emit past the closure
/// no longer reaches the set.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn with_default_scopes_to_the_closure_and_returns_its_value() {
    let log = Arc::new(Mutex::new(Vec::new()));

    let mut subs = Subscribers::default();
    subs.register(Box::new(Tagger::<Ping>::new("scoped", log.clone())));

    let out = subs.with_default(|| {
        event!(Ping::default());
        "result"
    });

    assert_eq!(out, "result");
    assert_eq!(*log.lock().unwrap(), vec!["scoped"]);

    // Scope ended: this emit reaches no scoped set, so the log is unchanged.
    event!(Ping::default());
    assert_eq!(*log.lock().unwrap(), vec!["scoped"]);
}
