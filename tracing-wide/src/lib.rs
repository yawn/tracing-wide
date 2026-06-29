// Via the package-local symlink, so the path also resolves inside the
// published archive (where `../../README.md` would not exist).
#![doc = include_str!("../README.md")]
//!
//! # Feature flags
//!
#![cfg_attr(
    feature = "docs",
    cfg_attr(doc, doc = ::document_features::document_features!())
)]
// The bare core (define + emit) uses only `core`; `std` — and every feature
// that enables it — links std.
#![cfg_attr(not(feature = "std"), no_std)]

use core::any::Any;
use core::fmt;

pub use tracing_wide_macros::{event, message};

#[cfg(doc)]
pub mod examples;

/// Re-export of the [`facet`](https://docs.rs/facet) crate, so a subscriber names
/// `Peek` and the `Facet` trait through the exact version tracing-wide compiled
/// against. facet is pre-1.0 — every minor is a breaking change — so naming it via a
/// separate direct dependency risks a type mismatch across the `as_facet` boundary;
/// going through `tracing_wide::facet` guarantees one version.
///
/// To `#[derive(Facet)]` on a message, either depend on `facet` directly (pinned to
/// the same version), or derive through this re-export with
/// `#[derive(tracing_wide::facet::Facet)]` + `#[facet(crate = tracing_wide::facet)]`,
/// which needs no direct facet dependency.
#[cfg(feature = "facet")]
pub use ::facet;

#[cfg(feature = "catalogue")]
pub mod catalogue;
#[cfg(feature = "instrument")]
pub mod instrument;
#[cfg(feature = "subscriber")]
pub mod subscriber;

/// Marker trait: a type eligible to be a field of a message.
///
/// Blanket-implemented for every `tracing::field::Value` — the supertrait bound
/// guarantees every field can be handed to tracing, and `Option<T: Value>` is
/// covered for free.
///
/// The trait carries no constraint of its own; it exists for the name and as the
/// single seam to tighten the field contract later. Serialization is never part
/// of this bound — it is opted into case-by-case at the subscriber.
pub trait Field: tracing::field::Value {}

/// A struct that may be emitted as a wide event.
///
/// Apply `#[message]`, which implements this trait, enforces that every field is
/// a [`Field`], and fills the inherent consts (`MSG`, `LEVEL`, `ORIGIN`, `TAGS`)
/// the methods below mirror (associated consts aren't object-safe).
///
/// Object-safe, so subscribers can receive `&dyn Message`. Pseudo-sealed
/// against accidental hand-written impls via the hidden
/// [`__private::MessageBehaviour`] supertrait, which only `#[message]` emits —
/// see [`__private::Sealed`] for why it's only a *pseudo*-seal.
pub trait Message: __private::MessageBehaviour {
    /// Escape hatch for subscribers that want the concrete type back, via
    /// `as_any().downcast_ref::<T>()`.
    fn as_any(&self) -> &dyn Any;

    /// Erased reflection hook, keyed on one knob: `#[derive(Facet)]`.
    ///
    /// Via [`__private::facet`] autoref specialization the generated body returns
    /// `Some` (a [`Peek`](::facet::Peek) over `self`) when the type derives
    /// `Facet` and `None` otherwise — no `Facet` bound ever lands on a message
    /// that didn't derive it. The introspection parallel to
    /// [`as_serialize`](Self::as_serialize): a subscriber walks the `Peek` to
    /// read fields by name and filter on a field's value, which static
    /// [`tags`](Self::tags) cannot.
    #[cfg(feature = "facet")]
    fn as_facet(&self) -> Option<::facet::Peek<'_, 'static>> {
        None
    }

    /// Erased serialization hook, keyed on one knob: `#[derive(Serialize)]`.
    ///
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
/// rustc to fill while compiling the *defining* crate.
///
/// Reachable on every message through [`Message::origin`], so a subscriber can
/// attribute or route a `&dyn Message` without naming the concrete type. All
/// fields are `&'static`/`u32`, so `Origin` is `Copy` and stays `no_std`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
#[cfg_attr(feature = "facet", facet(proxy = String))]
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

/// Facet serializes `Origin` through its compact `Display` string (the `proxy`),
/// matching the serde manifest. Serialize-only — provenance is never parsed back,
/// so the reverse conversion is a stub.
// Infallible, but facet's `proxy` mechanism is defined in terms of `TryFrom`.
#[cfg(feature = "facet")]
#[allow(clippy::infallible_try_from)]
impl TryFrom<&Origin> for String {
    type Error = core::convert::Infallible;
    fn try_from(origin: &Origin) -> Result<Self, Self::Error> {
        Ok(origin.to_string())
    }
}

#[cfg(feature = "facet")]
impl TryFrom<String> for Origin {
    type Error = &'static str;
    fn try_from(_: String) -> Result<Self, Self::Error> {
        Err("Origin is serialize-only in the catalogue")
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

/// `#[message]`-internal: emit the `Message::as_facet` override, or not.
/// Cfg-selected by *tracing-wide's* `facet` feature. Every message gets the
/// same body — `Some(Peek::new(self))` iff `Self: Facet`, decided at the call
/// site by [`__private::facet`] autoref specialization; `#[derive(Facet)]` is
/// the only knob.
#[cfg(feature = "facet")]
#[doc(hidden)]
#[macro_export]
macro_rules! __message_facet_method {
    () => {
        fn as_facet(&self) -> ::core::option::Option<$crate::__private::facet::Peek<'_, 'static>> {
            #[allow(unused_imports)]
            use $crate::__private::facet::{ViaFacet as _, ViaNotFacet as _};
            (&$crate::__private::facet::Probe::new(self)).__tracing_wide_as_facet()
        }
    };
}

#[cfg(not(feature = "facet"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __message_facet_method {
    () => {};
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

    /// Autoref specialization for the `__message_facet_method!` shim —
    /// `Some(Peek::new(self))` iff `Self: Facet<'static>` — plus the `Peek`
    /// re-export the shim names. The module is deliberately named `facet`; the
    /// `::facet` bound below names the crate absolutely.
    #[cfg(feature = "facet")]
    pub mod facet {
        use ::facet::Facet;
        pub use ::facet::Peek;

        pub struct Probe<'a, T>(&'a T);

        pub trait ViaFacet<'a> {
            fn __tracing_wide_as_facet(&self) -> Option<Peek<'a, 'static>>;
        }

        pub trait ViaNotFacet<'a> {
            fn __tracing_wide_as_facet(&self) -> Option<Peek<'a, 'static>>;
        }

        impl<'a, T> Probe<'a, T> {
            pub fn new(value: &'a T) -> Self {
                Probe(value)
            }
        }

        impl<'a, T: Facet<'static>> ViaFacet<'a> for Probe<'a, T> {
            fn __tracing_wide_as_facet(&self) -> Option<Peek<'a, 'static>> {
                Some(Peek::new(self.0))
            }
        }

        impl<'a, T> ViaNotFacet<'a> for &Probe<'a, T> {
            fn __tracing_wide_as_facet(&self) -> Option<Peek<'a, 'static>> {
                None
            }
        }
    }

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
