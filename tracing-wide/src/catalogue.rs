//! The auto-collected catalogue: per-type descriptors registered by `#[message]`
//! and read back at runtime. Gated as a whole by the `catalogue` feature.

use core::any::TypeId;
use std::collections::HashMap;

use tracing::Level;

use crate::Origin;

/// One field's catalogue metadata.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize))]
pub struct FieldDescriptor {
    /// `Some` iff the field carries `#[deprecated]` — the reason, or `"true"`
    /// when none is given. Serialized only when present.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub deprecated: Option<&'static str>,
    /// The field's `///` doc comments, harvested at compile time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub doc: Option<&'static str>,
    /// Arbitrary k/v from `#[field(key = ...)]`.
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "serde::pairs_empty",
            serialize_with = "serde::pairs"
        )
    )]
    pub meta: &'static [(&'static str, &'static str)],
    pub name: &'static str,
    /// Field type as written (e.g. `"& 'static str"`, `"Option < usize >"`).
    pub r#type: &'static str,
}

/// A message type's catalogue entry, auto-registered by `#[message]`.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize))]
pub struct MessageDescriptor {
    /// `Some` iff the message type carries `#[deprecated]` — the reason, or
    /// `"true"` when none is given (`since` is not captured). Producers get
    /// rustc's native warning; the catalogue records the intent. Serialized
    /// only when present.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub deprecated: Option<&'static str>,
    /// The struct's `///` doc comments, harvested at compile time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub doc: Option<&'static str>,
    pub fields: &'static [FieldDescriptor],
    /// `tracing::Level` isn't `Serialize`; rendered as its name ("WARN", …).
    #[cfg_attr(feature = "serde", serde(serialize_with = "serde::level"))]
    pub level: Level,
    /// Arbitrary k/v from `#[message(key = ...)]` (everything but `msg`/`level`).
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "serde::pairs_empty",
            serialize_with = "serde::pairs"
        )
    )]
    pub meta: &'static [(&'static str, &'static str)],
    /// The unique join key. Excluded from serialization: a serialized catalogue
    /// keys its entries by `msg` (see the `catalogue` example), so rendering it
    /// inside the entry too would duplicate the key.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub msg: &'static str,
    /// Where this message type is defined; serialized via its compact
    /// [`Display`](crate::Origin) — a single string like
    /// `mycrate src/lib.rs:12:1`, not a nested map.
    #[cfg_attr(feature = "serde", serde(serialize_with = "serde::origin"))]
    pub origin: Origin,
    /// Routing tags: sorted, deduped, lowercased. The common (untagged) case is
    /// empty and stays quiet in the manifest.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "serde::tags_empty"))]
    pub tags: &'static [&'static str],
    /// Runtime identity — the join key from a live `&dyn Message` via
    /// `m.as_any().type_id()`. Opaque and build-local, so excluded from
    /// serialization.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub type_id: TypeId,
}

/// The descriptor serialization shims, grouped behind one gate. A *local*
/// module deliberately named `serde` so the attribute paths above read as
/// `serde::pairs` etc.; it shadows the crate inside this file, which is why
/// the derives above name the crate absolutely (`::serde`).
#[cfg(feature = "serde")]
mod serde {
    use ::serde::{Serializer, ser::SerializeMap};
    use tracing::Level;

    use crate::Origin;

    /// Render a `tracing::Level` as its uppercase name.
    pub(super) fn level<S: Serializer>(level: &Level, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&level.to_string())
    }

    /// Render an [`Origin`] as its compact `Display` string.
    pub(super) fn origin<S: Serializer>(origin: &Origin, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(origin)
    }

    /// Render `meta` k/v slices as a map (a YAML/JSON mapping), not an array
    /// of pairs.
    pub(super) fn pairs<S: Serializer>(
        pairs: &&'static [(&'static str, &'static str)],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(pairs.len()))?;
        for (k, v) in pairs.iter() {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }

    pub(super) fn pairs_empty(pairs: &&'static [(&'static str, &'static str)]) -> bool {
        pairs.is_empty()
    }

    pub(super) fn tags_empty(tags: &&'static [&'static str]) -> bool {
        tags.is_empty()
    }
}

/// Iterate every registered message type. Populated at load time (before
/// `main` natively; via `__wasm_call_ctors` on wasm). Joinable to emitted
/// events by `MSG`.
pub fn all() -> impl Iterator<Item = &'static MessageDescriptor> {
    inventory::iter::<MessageDescriptor>.into_iter()
}

/// The `msg` values registered by more than one message type — `msg` is the
/// catalogue's unique join key, so an empty result is the invariant. A proc
/// macro cannot enforce this (each expansion is blind to its siblings), so the
/// check is a runtime walk; its natural home is a test in the *application*
/// crate, where every linked-in message is registered:
///
/// ```ignore
/// #[test]
/// fn catalogue_msgs_are_unique() {
///     assert_eq!(tracing_wide::catalogue::duplicates(), Vec::<&str>::new());
/// }
/// ```
pub fn duplicates() -> Vec<&'static str> {
    let mut counts = HashMap::new();
    for descriptor in all() {
        *counts.entry(descriptor.msg).or_insert(0_usize) += 1;
    }

    let mut keys: Vec<&'static str> = counts
        .into_iter()
        .filter_map(|(msg, count)| (count > 1).then_some(msg))
        .collect();

    keys.sort_unstable();

    keys
}

inventory::collect!(MessageDescriptor);
