//! Ambient autocapture: `tracing::instrument` as the contribution surface.
//!
//! Run: `just example-instrument`

use std::sync::{Arc, Mutex};

use tracing::instrument;
use tracing_subscriber::layer::SubscriberExt;
use tracing_wide::{
    Message, event, message,
    sink::{Sink, Sinks},
};

struct Park(Arc<Mutex<Option<WorkFinished>>>);

/// `payload` is required; `component` and `attempt` join from the spans of `handle` and `work`.
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

impl Sink for Park {
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

// `tracing::instrument` records arguments by default, so `attempt` lands on this span.
#[instrument]
fn work(attempt: usize) {
    event!(WorkFinished::payload(42));
}

fn main() {
    let parked = Arc::new(Mutex::new(None));

    let mut sinks = Sinks::default();
    sinks.register(Box::new(Park(parked.clone())));
    sinks.install().ok();

    let subscriber = tracing_subscriber::registry()
        .with(tracing_wide::instrument::layer())
        // No timestamp: the output is a checked snapshot.
        .with(tracing_subscriber::fmt::layer().without_time());

    tracing::subscriber::set_global_default(subscriber).expect("first subscriber");

    handle(3);

    let work = parked
        .lock()
        .unwrap()
        .take()
        .expect("the typed sink saw the event");

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
