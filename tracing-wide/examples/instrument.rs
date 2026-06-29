//! Ambient autocapture: `tracing::instrument` as the contribution surface.
//!
//! Run: `just example-instrument`
//!
//! Stock `tracing::instrument` opens the spans (tracing-wide ships no span
//! macro); whatever it records becomes ambient context via
//! [`CaptureLayer`](tracing_wide::instrument::CaptureLayer). At `event!` time
//! the still-`None` `Option` fields fill from the span scope by name, innermost
//! first — required fields stay compile-checked at the literal, and a field
//! already set is never overwritten. The fmt layer shows the tracing handoff;
//! a typed subscriber parks the *same* message to verify every ambient value
//! arrived, proving the join happens before dispatch.

use std::sync::{Arc, Mutex};

use tracing::instrument;
use tracing_subscriber::layer::SubscriberExt;
use tracing_wide::{
    Message, event, message,
    subscriber::{Subscriber, Subscribers},
};

struct Park(Arc<Mutex<Option<WorkFinished>>>);

/// `payload` is required (supplied at the `event!` literal). `component` and
/// `attempt` are ambient: nothing in `work` knows them — they join from the
/// spans `handle` and `work` opened above the emission point.
#[message(msg = "work finished")]
#[derive(Clone, Default)]
struct WorkFinished {
    attempt: Option<usize>,
    component: Option<String>,
    payload: usize,
}

impl WorkFinished {
    fn payload(payload: usize) -> Self {
        WorkFinished {
            payload,
            ..Default::default()
        }
    }
}

impl Subscriber for Park {
    fn on_message(&self, m: &dyn Message) {
        if let Some(work) = m.as_any().downcast_ref::<WorkFinished>() {
            *self.0.lock().unwrap() = Some(work.clone());
        }
    }
}

#[instrument(fields(component = "billing"))]
fn handle(attempt: usize) {
    work(attempt);
}

// `tracing::instrument` records arguments by default: `attempt` lands on this
// span without being named.
#[instrument]
fn work(attempt: usize) {
    event!(WorkFinished::payload(42));
}

fn main() {
    let parked = Arc::new(Mutex::new(None));

    let mut subscribers = Subscribers::default();
    subscribers.register(Box::new(Park(parked.clone())));
    subscribers.install().ok();

    let subscriber = tracing_subscriber::registry()
        .with(tracing_wide::instrument::layer())
        // `.without_time()`: the wall-clock timestamp is the one non-deterministic
        // bit of this example's output, and it's documented verbatim (README +
        // rustdoc), so drop it to keep the snapshot stable.
        .with(tracing_subscriber::fmt::layer().without_time());

    tracing::subscriber::set_global_default(subscriber).expect("first subscriber");

    handle(3);

    let work = parked
        .lock()
        .unwrap()
        .take()
        .expect("the typed subscriber saw the event");

    assert_eq!(work.payload, 42, "required field, from the literal");

    assert_eq!(
        work.component.as_deref(),
        Some("billing"),
        "joined from handle's explicit fields(...)"
    );

    assert_eq!(
        work.attempt,
        Some(3),
        "joined from work's auto-recorded argument"
    );

    println!(
        "[parked] all values present: payload={} component={:?} attempt={:?}",
        work.payload, work.component, work.attempt
    );
}
