//! Typed fan-out: registered sinks see each message as `&dyn Message` before the tracing handoff (feature `sink`).

use core::cell::RefCell;
use std::sync::{Arc, OnceLock};

use crate::Message;

static SINKS: OnceLock<Sinks> = OnceLock::new();

thread_local! {
    /// The scoped set of this thread; replaces the global set while a [`DefaultGuard`] lives. `Arc`, so [`dispatch`]
    /// releases the borrow before the fan-out: a sink can emit.
    static SCOPED: RefCell<Option<Arc<Sinks>>> = const { RefCell::new(None) };
}

/// Restores the previous thread-local scope on drop.
#[must_use = "dropping the guard immediately ends the scope"]
pub struct DefaultGuard(Option<Arc<Sinks>>);

/// A sink for each emitted message; downcast with [`Message::as_any`].
pub trait Sink: Send + Sync {
    fn on_message(&self, msg: &dyn Message);
}

/// A set of sinks: register into it, then [`install`](Self::install) it or scope it to a thread.
#[derive(Default)]
pub struct Sinks(Vec<Box<dyn Sink>>);

impl Sinks {
    /// Fan a message out to every sink in this set.
    fn dispatch(&self, msg: &dyn Message) {
        for sink in &self.0 {
            sink.on_message(msg);
        }
    }

    /// Installs the set globally, as tracing's `set_global_default` does; `Err(self)` if a set is installed.
    pub fn install(self) -> Result<(), Sinks> {
        SINKS.set(self)
    }

    /// Adds a sink.
    pub fn register(&mut self, sink: Box<dyn Sink>) {
        self.0.push(sink);
    }

    /// Scopes the set to this thread while the guard lives.
    pub fn set_default(self) -> DefaultGuard {
        let previous = SCOPED.with(|cell| cell.borrow_mut().replace(Arc::new(self)));
        DefaultGuard(previous)
    }

    /// Runs `f` with the set scoped to this thread, as tracing's [`with_default`](tracing::subscriber::with_default).
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

/// Fans out to the scoped set of this thread, else to the installed set.
pub(crate) fn dispatch(msg: &dyn Message) {
    if let Some(sinks) = SCOPED
        .with(|cell| cell.borrow().clone())
        .as_deref()
        .or_else(|| SINKS.get())
    {
        sinks.dispatch(msg);
    }
}
