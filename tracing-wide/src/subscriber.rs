//! App-level subscriber mechanics: a global registry whose handlers see each
//! emitted message as a typed `&dyn Message` *before* the tracing handoff.
//! Gated by the `subscriber` feature — without it, `event!` just records to
//! tracing and the core stays free of global state.

use std::sync::OnceLock;

use crate::Message;

static SUBSCRIBERS: OnceLock<Subscribers> = OnceLock::new();

/// A runtime-installed sink. Downcast via [`Message::as_any`] to the concrete
/// type; serialization is the subscriber's concern, never a bound here.
pub trait Subscriber: Send + Sync {
    fn on_message(&self, msg: &dyn Message);
}

/// The set of subscribers to install: register into it, then publish it once
/// with [`install`](Self::install). Lock-free by construction — a single owner
/// while being built, immutable once installed.
#[derive(Default)]
pub struct Subscribers(Vec<Box<dyn Subscriber>>);

impl Subscribers {
    /// Fan a message out to every subscriber in this set.
    fn dispatch(&self, msg: &dyn Message) {
        for sub in &self.0 {
            sub.on_message(msg);
        }
    }

    /// Install this set as the global subscribers — the typed analogue of
    /// tracing's `set_global_default`. Returns `self` back as `Err` if a set
    /// was already installed (mirrors [`OnceLock::set`]).
    pub fn install(self) -> Result<(), Subscribers> {
        SUBSCRIBERS.set(self)
    }

    /// Add a subscriber to the set. Call before [`install`](Self::install).
    pub fn register(&mut self, subscriber: Box<dyn Subscriber>) {
        self.0.push(subscriber);
    }
}

/// Fan a message out to the installed subscribers; a no-op until `install`.
/// Called by the `emit` path — reading the `OnceLock` never blocks or poisons.
pub(crate) fn dispatch(msg: &dyn Message) {
    if let Some(subscribers) = SUBSCRIBERS.get() {
        subscribers.dispatch(msg);
    }
}
