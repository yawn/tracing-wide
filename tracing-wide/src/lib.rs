#![doc = include_str!("../../README.md")]
//!
//! # Feature flags
//!
#![cfg_attr(
    feature = "docs-features",
    cfg_attr(doc, doc = ::document_features::document_features!())
)]
// The bare core (define + emit) uses only `core`; `std` — and every feature
// that enables it — links std.
#![cfg_attr(not(feature = "std"), no_std)]

use core::any::Any;
use core::fmt;

pub use tracing_wide_macros::{event, message};

#[cfg(feature = "catalogue")]
pub mod catalogue;
#[cfg(feature = "instrument")]
pub mod instrument;
#[cfg(feature = "subscriber")]
pub mod subscriber;

/// Marker trait: a type eligible to be a field of a message, blanket-implemented
/// for every `tracing::field::Value` — the supertrait bound guarantees every
/// field can be handed to tracing, and `Option<T: Value>` is covered for free.
/// The trait carries no constraint of its own; it exists for the name and as the
/// single seam to tighten the field contract later. Serialization is never part
/// of this bound — it is opted into case-by-case at the subscriber.
pub trait Field: tracing::field::Value {}

/// A struct that may be emitted as a wide event. Apply `#[message]`, which
/// implements this trait, enforces that every field is a [`Field`], and fills
/// the inherent consts (`MSG`, `LEVEL`, `ORIGIN`, `TAGS`) the methods below
/// mirror (associated consts aren't object-safe).
///
/// Object-safe, so subscribers can receive `&dyn Message`. Pseudo-sealed
/// against accidental hand-written impls via the hidden
/// [`__private::MessageBehaviour`] supertrait, which only `#[message]` emits —
/// see [`__private::Sealed`] for why it's only a *pseudo*-seal.
pub trait Message: __private::MessageBehaviour {
    /// Escape hatch for subscribers that want the concrete type back, via
    /// `as_any().downcast_ref::<T>()`.
    fn as_any(&self) -> &dyn Any;

    /// Erased serialization hook, keyed on one knob: `#[derive(Serialize)]`.
    /// Via [`__private::serde`] autoref specialization the generated body
    /// returns `Some(self)` when the type derives `Serialize` and `None`
    /// otherwise — no `Serialize` bound ever lands on a message that didn't
    /// derive it. The only serde surface in the core; `dyn
    /// erased_serde::Serialize` implements `serde::Serialize`, so a subscriber
    /// serializes the result with any format.
    #[cfg(feature = "serde")]
    fn as_serialize(&self) -> Option<&dyn ::erased_serde::Serialize> {
        None
    }

    /// Severity of this event type; `#[message(level = ...)]`, default `INFO`.
    fn level(&self) -> tracing::Level;

    /// The constant, static message text.
    fn msg(&self) -> &'static str;

    /// Where this message type is defined — automatic provenance, never set by
    /// hand. See [`Origin`].
    fn origin(&self) -> &'static Origin;

    /// Static routing tags: the sorted, deduped, lowercased set from
    /// `#[message(tags = [...])]`, default empty. Where [`origin`](Self::origin)
    /// is provenance (where defined), tags are intent (where to send) — routing
    /// is the subscriber's call, e.g. `m.tags().contains(&"analytics")`.
    fn tags(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Where a message *type* is defined — automatic provenance captured by
/// `#[message]`, which emits the `core` location builtins
/// (`env!("CARGO_PKG_NAME")`, `module_path!`, `file!`, `line!`, `column!`) for
/// rustc to fill while compiling the *defining* crate. Reachable on every
/// message through [`Message::origin`], so a subscriber can attribute or route
/// a `&dyn Message` without naming the concrete type. All fields are
/// `&'static`/`u32`, so `Origin` is `Copy` and stays `no_std`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Origin {
    /// Column of the definition (`column!()`).
    pub column: u32,
    /// Source file of the definition (`file!()`).
    pub file: &'static str,
    /// The defining crate (`CARGO_PKG_NAME`); named `krate` because `crate`
    /// cannot be written as a raw identifier.
    pub krate: &'static str,
    /// Line of the definition (`line!()`).
    pub line: u32,
    /// The defining module path (`module_path!()`), e.g. `mycrate::sub`.
    pub module: &'static str,
}

impl fmt::Display for Origin {
    /// Compact one-line form: `crate file:line:column` (the `module` is
    /// omitted — it's prefixed by the crate and rarely needed at a glance).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}:{}:{}",
            self.krate, self.file, self.line, self.column
        )
    }
}

impl<T: tracing::field::Value> Field for T {}

/// `#[message]`-internal: register a descriptor in the catalogue, or not.
/// Cfg-selected by *tracing-wide's* `catalogue` feature; with it off, the call
/// expands to nothing and never names the (absent) descriptor types.
#[cfg(feature = "catalogue")]
#[doc(hidden)]
#[macro_export]
macro_rules! __register_message {
    ($desc:expr) => {
        // In a const item so the allow reliably covers the whole expansion — a
        // deprecated message type is still named by its own registration.
        #[allow(deprecated)]
        const _: () = {
            $crate::__private::inventory::submit! { $desc }
        };
    };
}

#[cfg(not(feature = "catalogue"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __register_message {
    ($desc:expr) => {};
}

/// `#[message]`-internal: emit the `MessageBehaviour::join_ambient` override,
/// or not. Cfg-selected by *tracing-wide's* `instrument` feature (a `#[cfg]`
/// the proc macro emitted would be evaluated in the wrong crate). Receives
/// every `Option` field as `(ident, InnerType)`; the override fills each
/// still-`None` field from the span scope by name, with
/// [`__private::instrument`] autoref specialization deciding per inner type
/// whether a [`FromCaptured`](instrument::FromCaptured) conversion exists.
#[cfg(feature = "instrument")]
#[doc(hidden)]
#[macro_export]
macro_rules! __message_ambient_method {
    ( $( ($field:ident, $ty:ty) ),* $(,)? ) => {
        // Deprecated fields stay joinable; only producer construction should warn.
        #[allow(deprecated)]
        fn join_ambient(&mut self) {
            $(
                if self.$field.is_none() {
                    if let ::core::option::Option::Some(__tracing_wide_value) =
                        $crate::__private::instrument::get(::core::stringify!($field))
                    {
                        #[allow(unused_imports)]
                        use $crate::__private::instrument::{
                            ViaFromCaptured as _, ViaNotCapturable as _,
                        };
                        self.$field = (&$crate::__private::instrument::Probe::<$ty>::new())
                            .__tracing_wide_from_captured(&__tracing_wide_value);
                    }
                }
            )*
        }
    };
}

#[cfg(not(feature = "instrument"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __message_ambient_method {
    ( $($tt:tt)* ) => {};
}

/// `#[message]`-internal: emit the `Message::as_serialize` override, or not.
/// Cfg-selected by *tracing-wide's* `serde` feature. Every message gets the
/// same body — `Some(self)` iff `Self: Serialize`, decided at the call site by
/// [`__private::serde`] autoref specialization; `#[derive(Serialize)]` is the
/// only knob.
#[cfg(feature = "serde")]
#[doc(hidden)]
#[macro_export]
macro_rules! __message_serialize_method {
    () => {
        fn as_serialize(&self) -> ::core::option::Option<&dyn $crate::__private::serde::Serialize> {
            #[allow(unused_imports)]
            use $crate::__private::serde::{ViaNotSerializable as _, ViaSerialize as _};
            (&$crate::__private::serde::Probe::new(self)).__tracing_wide_as_serialize()
        }
    };
}

#[cfg(not(feature = "serde"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __message_serialize_method {
    () => {};
}

/// Macro-internal plumbing — everything `#[message]`/`event!` expansions name
/// in *downstream* crates. All of it is `pub` only because generated code must
/// reach it across crate boundaries; none of it is supported API. (The `__*!`
/// macros can't join it — `#[macro_export]` forces them to the crate root.)
#[doc(hidden)]
pub mod __private {
    /// Re-export so `__register_message!` can name inventory without the user
    /// crate depending on it directly. Gated by `catalogue`, like its caller.
    #[cfg(feature = "catalogue")]
    pub use ::inventory;
    /// Re-export so `#[message]` expansions can name `Level` and the level
    /// macros through *this* crate — a downstream crate that only defines
    /// messages needs no direct `tracing` dependency.
    pub use ::tracing;

    /// Recording behavior for [`Message`](crate::Message), generated by
    /// `#[message]` and itself pseudo-sealed via [`Sealed`] — combined with
    /// `Message: MessageBehaviour`, `#[message]` is the only supported way to
    /// obtain a `Message`.
    pub trait MessageBehaviour: Sealed {
        /// The single entry point `event!` expands to: fan out to registered
        /// subscribers (with the `subscriber` feature), then hand off to
        /// tracing. The body lives here so the cfg keys on *tracing-wide's*
        /// feature and can reach the crate-private registry; the `Self: Sized`
        /// bound lets `self` coerce to `&dyn Message` while keeping the trait
        /// object-safe.
        fn emit(&self)
        where
            Self: crate::Message + Sized,
        {
            #[cfg(feature = "subscriber")]
            crate::subscriber::dispatch(self);
            self.record();
        }

        /// Fill still-`None` `Option` fields from the ambient span scope.
        /// `event!` calls this *before* `emit`. A no-op unless the `instrument`
        /// feature lets `#[message]` override it; unconditional (not cfg-gated)
        /// so `event!` expansions compile regardless of tracing-wide's features.
        fn join_ambient(&mut self) {}

        /// The tracing handoff for this concrete type. Generated by `#[message]`.
        fn record(&self);
    }

    /// The pseudo-seal: only `#[message]` emits `impl Sealed`, so neither
    /// [`MessageBehaviour`] nor (transitively) [`Message`](crate::Message) can
    /// be implemented by hand *by accident*. Only a *pseudo*-seal because
    /// `#[message]` emits this impl in the *downstream* crate, so the trait must
    /// stay `pub`/reachable — and anything the macro can write, a hand can write
    /// too. It catches accidents, not deliberate reach-through, which stays
    /// unsupported.
    pub trait Sealed {}

    /// Autoref specialization for the `__message_ambient_method!` shim —
    /// deciding per inner type whether a
    /// [`FromCaptured`](crate::instrument::FromCaptured) conversion exists —
    /// plus [`get`](instrument::get), the read half of ambient autocapture.
    #[cfg(feature = "instrument")]
    pub mod instrument {
        use core::marker::PhantomData;

        use tracing::{Span, dispatcher};
        use tracing_subscriber::registry::{LookupSpan, Registry};

        use crate::instrument::{CapturedFields, CapturedValue, FromCaptured};

        pub struct Probe<T>(PhantomData<T>);

        pub trait ViaFromCaptured<T> {
            fn __tracing_wide_from_captured(&self, value: &CapturedValue) -> Option<T>;
        }

        pub trait ViaNotCapturable<T> {
            fn __tracing_wide_from_captured(&self, value: &CapturedValue) -> Option<T>;
        }

        impl<T> Probe<T> {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Probe(PhantomData)
            }
        }

        impl<T: FromCaptured> ViaFromCaptured<T> for Probe<T> {
            fn __tracing_wide_from_captured(&self, value: &CapturedValue) -> Option<T> {
                T::from_captured(value)
            }
        }

        impl<T> ViaNotCapturable<T> for &Probe<T> {
            fn __tracing_wide_from_captured(&self, _: &CapturedValue) -> Option<T> {
                None
            }
        }

        /// Look `name` up in the current span scope, innermost span first.
        /// Requires a stack built on the concrete `Registry`
        /// (`tracing_subscriber::registry()`); every failure mode — no
        /// dispatcher, no current span, a non-`Registry` subscriber, no
        /// [`CaptureLayer`](crate::instrument::CaptureLayer), name not
        /// captured — is a quiet `None`. Emission never fails on ambient state.
        pub fn get(name: &str) -> Option<CapturedValue> {
            let current = Span::current();
            let id = current.id()?;

            dispatcher::get_default(|dispatch| {
                let registry = dispatch.downcast_ref::<Registry>()?;

                let span = registry.span(&id)?;

                for span in span.scope() {
                    if let Some(captured) = span.extensions().get::<CapturedFields>()
                        && let Some((_, value)) = captured.0.iter().find(|(n, _)| *n == name)
                    {
                        return Some(value.clone());
                    }
                }
                None
            })
        }
    }

    /// Autoref specialization for the `__message_serialize_method!` shim —
    /// `Some(self)` iff `Self: Serialize` — plus the erased-serde re-export the
    /// shim names. The module is deliberately named `serde`; the `::serde`
    /// bound below names the crate absolutely.
    #[cfg(feature = "serde")]
    pub mod serde {
        pub use ::erased_serde::Serialize;

        pub struct Probe<'a, T>(&'a T);

        pub trait ViaNotSerializable<'a> {
            fn __tracing_wide_as_serialize(&self) -> Option<&'a dyn Serialize>;
        }

        pub trait ViaSerialize<'a> {
            fn __tracing_wide_as_serialize(&self) -> Option<&'a dyn Serialize>;
        }

        impl<'a, T> Probe<'a, T> {
            pub fn new(value: &'a T) -> Self {
                Probe(value)
            }
        }

        impl<'a, T> ViaNotSerializable<'a> for &Probe<'a, T> {
            fn __tracing_wide_as_serialize(&self) -> Option<&'a dyn Serialize> {
                None
            }
        }

        impl<'a, T: ::serde::Serialize> ViaSerialize<'a> for Probe<'a, T> {
            fn __tracing_wide_as_serialize(&self) -> Option<&'a dyn Serialize> {
                Some(self.0)
            }
        }
    }
}
