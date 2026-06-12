//! Typed subscriber fan-out.
//!
//! Run: `just example-subscriber`
//!
//! With the `subscriber` feature, `event!` hands each message to every
//! registered [`Subscriber`] as a typed `&dyn Message` *before* the tracing
//! handoff. A sink can stay generic (the object-safe accessors) or downcast to
//! the concrete types it cares about — no string parsing, no `Visit`
//! round-trip. The registry is set-once at startup, mirroring
//! `tracing::subscriber::set_global_default`.

use tracing_wide::{
    Message, event, message,
    subscriber::{Subscriber, Subscribers},
};

struct Lines;

#[message(msg = "request completed")]
struct RequestCompleted {
    duration_ms: usize,
    route: &'static str,
}

#[message(msg = "service started", level = warn)]
struct Started {
    attempt: usize,
    service: &'static str,
}

/// A typed sink: downcasts to the one type it cares about and reads real fields.
struct StartupWatch;

impl Subscriber for Lines {
    fn on_message(&self, m: &dyn Message) {
        println!("[line] {:5} {}", m.level().to_string(), m.msg());
    }
}

impl Subscriber for StartupWatch {
    fn on_message(&self, m: &dyn Message) {
        if let Some(started) = m.as_any().downcast_ref::<Started>() {
            println!(
                "[watch] {} is up (attempt {})",
                started.service, started.attempt
            );
        }
    }
}

fn main() {
    let mut subscribers = Subscribers::default();
    subscribers.register(Box::new(Lines));
    subscribers.register(Box::new(StartupWatch));
    subscribers.install().ok();

    event!(Started {
        service: "billing",
        attempt: 1,
    });

    event!(RequestCompleted {
        route: "/users/:id",
        duration_ms: 12,
    });
}
