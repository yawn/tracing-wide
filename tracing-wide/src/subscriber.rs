//! App-level subscriber mechanics: a global registry whose handlers see each
//! emitted message as a typed `&dyn Message` *before* the tracing handoff.
//! Gated by the `subscriber` feature — without it, `event!` just records to
//! tracing and the core stays free of global state.

use core::cell::RefCell;
use std::sync::{Arc, OnceLock};

use crate::Message;

static SUBSCRIBERS: OnceLock<Subscribers> = OnceLock::new();

thread_local! {
    /// The thread-local scoped set, mirroring tracing's `set_default`.
    ///
    /// When set, it *replaces* the global [`SUBSCRIBERS`] for this thread
    /// for as long as a [`DefaultGuard`] is held; cleared (restored to
    /// the previous scope) on the guard's drop.
    ///
    /// `Arc` so [`dispatch`] can lift the set out of the cell and
    /// release the borrow *before* fanning out — a subscriber that itself emits
    /// would otherwise re-enter and double-borrow.
    static SCOPED: RefCell<Option<Arc<Subscribers>>> = const { RefCell::new(None) };
}

/// Restores the previous thread-local scope on drop.
#[must_use = "dropping the guard immediately ends the scope"]
pub struct DefaultGuard(Option<Arc<Subscribers>>);

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

    /// Add a subscriber to the set. Call before [`install`](Self::install) or
    /// before scoping the set with [`set_default`](Self::set_default) /
    /// [`with_default`](Self::with_default).
    pub fn register(&mut self, subscriber: Box<dyn Subscriber>) {
        self.0.push(subscriber);
    }

    /// Set this set as the calling thread's subscribers for as long as the
    /// returned guard lives.
    pub fn set_default(self) -> DefaultGuard {
        let previous = SCOPED.with(|cell| cell.borrow_mut().replace(Arc::new(self)));
        DefaultGuard(previous)
    }

    /// Run `f` with this set as the calling thread's scoped set, restoring the
    /// previous scope afterward — the typed analogue of tracing's
    /// [`with_default`](tracing::subscriber::with_default).
    pub fn with_default<T>(self, f: impl FnOnce() -> T) -> T {
        let _guard = self.set_default();
        f()
    }
}

impl Drop for DefaultGuard {
    fn drop(&mut self) {
        SCOPED.with(|cell| {
            *cell.borrow_mut() = self.0.take();
        });
    }
}

/// Fan a message out to the active subscribers: the calling thread's scoped set
/// if one is held (see [`set_default`](Subscribers::set_default)), else the
/// global [`install`](Subscribers::install)ed set, else a no-op.
pub(crate) fn dispatch(msg: &dyn Message) {
    if let Some(subscribers) = SCOPED
        .with(|cell| cell.borrow().clone())
        .as_deref()
        .or_else(|| SUBSCRIBERS.get())
    {
        subscribers.dispatch(msg);
    }
}
