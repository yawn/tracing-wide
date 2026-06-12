//! Serializing messages from a subscriber.
//!
//! Run: `just example-serializable`
//!
//! tracing-wide puts **no `Serialize` bound** on messages: a type opts in with
//! `#[derive(Serialize)]` alone, and [`Message::as_serialize`] hands back an
//! object-safe `&dyn erased_serde::Serialize` a subscriber can serialize with
//! any format — no downcast, no per-type match. A type that doesn't derive
//! `Serialize` simply yields `None`.

use serde::Serialize;
use tracing_wide::{
    Message, event, message,
    subscriber::{Subscriber, Subscribers},
};

struct JsonForwarder;

#[message(msg = "request completed")]
#[derive(Serialize)]
struct RequestCompleted {
    duration_ms: usize,
    route: &'static str,
}

#[message(msg = "service started", level = warn)]
#[derive(Serialize)]
struct Started {
    attempt: usize,
    service: &'static str,
}

impl Subscriber for JsonForwarder {
    fn on_message(&self, m: &dyn Message) {
        let Some(fields) = m.as_serialize() else {
            return;
        };

        let payload = serde_json::json!({
            "msg": m.msg(),
            "level": m.level().to_string(),
            "fields": serde_json::to_value(fields).unwrap(),
        });

        println!("[forward] {payload}");
    }
}

fn main() {
    let mut subscribers = Subscribers::default();
    subscribers.register(Box::new(JsonForwarder));
    subscribers.install().ok();

    event!(Started {
        service: "billing",
        attempt: 1
    });

    event!(RequestCompleted {
        route: "/users/:id",
        duration_ms: 12
    });
}
