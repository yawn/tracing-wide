// The package-local README symlink, so the path also resolves in the published archive.
#![doc = include_str!("../README.md")]
//!
//! # Feature flags
//!
#![cfg_attr(
    feature = "docs",
    cfg_attr(doc, doc = ::document_features::document_features!())
)]
#![cfg_attr(not(feature = "std"), no_std)]

use core::any::Any;
use core::fmt;

pub use tracing_wide_macros::{event, message};

#[cfg(doc)]
pub mod examples;

/// Re-export of [`facet`](https://docs.rs/facet): specify `Peek` and `Facet` through it, so they match the version that
/// tracing-wide compiles against. To derive without a direct `facet` dependency, write
/// `#[derive(tracing_wide::facet::Facet)]` with `#[facet(crate = tracing_wide::facet)]`.
#[cfg(feature = "facet")]
pub use ::facet;

#[cfg(feature = "catalogue")]
pub mod catalogue;
#[cfg(feature = "instrument")]
pub mod instrument;
#[cfg(feature = "sink")]
pub mod sink;

/// Marker for a type that can be a message field; implemented for every `tracing::field::Value`.
pub trait Field: tracing::field::Value {}

/// A struct that `event!` emits; sinks receive it as `&dyn Message`. Only `#[message]` implements it.
pub trait Message: __private::MessageBehaviour {
    /// The concrete message, for `as_any().downcast_ref::<T>()`.
    fn as_any(&self) -> &dyn Any;

    /// A [`Peek`](::facet::Peek) over the message if its type derives `Facet`, else `None`.
    #[cfg(feature = "facet")]
    fn as_facet(&self) -> Option<::facet::Peek<'_, 'static>> {
        None
    }

    /// The message as `erased_serde::Serialize` if its type derives `Serialize`, else `None`.
    #[cfg(feature = "serde")]
    fn as_serialize(&self) -> Option<&dyn ::erased_serde::Serialize> {
        None
    }

    /// Severity of this event type; `#[message(level = ...)]`, default `INFO`.
    fn level(&self) -> tracing::Level;

    /// The constant, static message text.
    fn msg(&self) -> &'static str;

    /// Where the message type is defined; see [`Origin`].
    fn origin(&self) -> &'static Origin;

    /// The routing tags of `#[message(tags = [...])]`: sorted, deduped, lowercase; empty by default.
    fn tags(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Where a message type is defined, captured by `#[message]` at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
#[cfg_attr(feature = "facet", facet(proxy = String))]
pub struct Origin {
    /// Column of the definition (`column!()`).
    pub column: u32,
    /// Source file of the definition (`file!()`).
    pub file: &'static str,
    /// The defining crate (`CARGO_PKG_NAME`).
    pub krate: &'static str,
    /// Line of the definition (`line!()`).
    pub line: u32,
    /// The defining module path (`module_path!()`), e.g. `mycrate::sub`.
    pub module: &'static str,
}

impl fmt::Display for Origin {
    /// `crate file:line:column`, without the module.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}:{}:{}",
            self.krate, self.file, self.line, self.column
        )
    }
}

/// Facet serializes an `Origin` as its `Display` string (serialize-only).
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

/// `#[message]`-internal: registers a catalogue descriptor if the `catalogue` feature is on.
#[cfg(feature = "catalogue")]
#[doc(hidden)]
#[macro_export]
macro_rules! __register_message {
    ($desc:expr) => {
        // A const item, so the allow covers the whole expansion.
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

/// `#[message]`-internal: emits `Message::as_facet` if the `facet` feature is on.
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

/// `#[message]`-internal: emits `MessageBehaviour::join_ambient` if the `instrument` feature is on.
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

/// `#[message]`-internal: emits `Message::as_serialize` if the `serde` feature is on.
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

/// Plumbing for `#[message]` and `event!` expansions; not supported API. The feature cfgs live here and in the
/// `__*!` macros, because a cfg in proc-macro output compiles in the caller's crate.
#[doc(hidden)]
pub mod __private {
    /// For `__register_message!`, so a user crate needs no `inventory` dependency.
    #[cfg(feature = "catalogue")]
    pub use ::inventory;
    /// For expansions, so a crate that only defines messages needs no `tracing` dependency.
    pub use ::tracing;

    /// The recording side of [`Message`](crate::Message); only `#[message]` implements it.
    pub trait MessageBehaviour: Sealed {
        /// The entry point of `event!`: fans out to sinks, then records to tracing.
        fn emit(&self)
        where
            Self: crate::Message + Sized,
        {
            #[cfg(feature = "sink")]
            crate::sink::dispatch(self);
            self.record();
        }

        /// Fills still-`None` `Option` fields from the span scope; `event!` calls it before `emit`.
        fn join_ambient(&mut self) {}

        /// The tracing handoff for this concrete type. Generated by `#[message]`.
        fn record(&self);
    }

    /// The pseudo-seal: only `#[message]` implements it, which stops an accidental hand-written impl.
    pub trait Sealed {}

    /// Autoref specialization for `__message_facet_method!`.
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

    /// Autoref specialization for `__message_ambient_method!`, and the span-scope lookup.
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

        /// Looks up `name` in the current span scope, innermost span first; `None` on any failure.
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

    /// Autoref specialization for `__message_serialize_method!`.
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
