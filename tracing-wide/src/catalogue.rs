//! The auto-collected catalogue: per-type descriptors registered by `#[message]`
//! and read back at runtime. Gated as a whole by the `catalogue` feature.

use core::any::TypeId;
use std::collections::BTreeMap;

use tracing::Level;

use crate::Origin;

/// One field's catalogue metadata.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize))]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
pub struct FieldDescriptor {
    /// `Some` iff the field carries `#[deprecated]` — the reason, or `"true"`
    /// when none is given. Serialized only when present.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub deprecated: Option<&'static str>,
    /// The field's `///` doc comments, harvested at compile time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub doc: Option<&'static str>,
    /// Arbitrary k/v from `#[field(key = ...)]`. Rendered as a map under both
    /// frameworks — serde via `serde::pairs`, facet via the `MetaMap` proxy.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "pairs_empty", serialize_with = "serde::pairs")
    )]
    #[cfg_attr(feature = "facet", facet(proxy = MetaMap, skip_serializing_if = pairs_empty))]
    pub meta: &'static [(&'static str, &'static str)],
    pub name: &'static str,
    /// Field type as written (e.g. `"& 'static str"`, `"Option < usize >"`).
    #[cfg_attr(feature = "facet", facet(rename = "type"))]
    pub r#type: &'static str,
}

/// A message's severity — a mirror of `tracing::Level`'s five levels.
///
/// `tracing::Level` is neither `Serialize` nor `Facet` (and foreign, so the
/// catalogue can't implement either) and its variants are sealed, so the level is
/// recorded as this enum. It renders as the canonical name (`"WARN"`) under both
/// frameworks and converts back with `Level::from`. The variant names are
/// SCREAMING so both serializers emit the exact tracing names with no rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize))]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum LevelName {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

/// A message type's catalogue entry, auto-registered by `#[message]`.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize))]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
pub struct MessageDescriptor {
    /// `Some` iff the message type carries `#[deprecated]` — the reason, or
    /// `"true"` when none is given (`since` is not captured). Producers get
    /// rustc's native warning; the catalogue records the intent. Serialized
    /// only when present.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub deprecated: Option<&'static str>,
    /// The struct's `///` doc comments, harvested at compile time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub doc: Option<&'static str>,
    pub fields: &'static [FieldDescriptor],
    /// The *raw* explicit id: `Some` if the type carried `#[message(id = "...")]`.
    /// Not serialized: the entry is keyed by [`resolved_id`](Self::resolved_id), and
    /// re-stating the raw id inside it would duplicate that key for every message
    /// that declares one. A consumer that needs "was this id explicit?" reads the
    /// live descriptor, not the manifest.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub id: Option<&'static str>,
    /// Severity, recorded as its canonical name — see [`LevelName`].
    pub level: LevelName,
    /// Arbitrary k/v from `#[message(key = ...)]` (everything but `msg`/`level`),
    /// rendered as a map — serde via `serde::pairs`, facet via the `MetaMap` proxy.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "pairs_empty", serialize_with = "serde::pairs")
    )]
    #[cfg_attr(feature = "facet", facet(proxy = MetaMap, skip_serializing_if = pairs_empty))]
    pub meta: &'static [(&'static str, &'static str)],
    /// The human-readable event text — semantically the unique name of this event
    /// type, though uniqueness is enforced on
    /// [`resolved_id`](Self::resolved_id), not here. Excluded from serialization:
    /// the manifest keys its entries by the resolved id (see the
    /// `catalogue-serde` example), which equals `msg` unless an explicit id
    /// overrides it.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub msg: &'static str,
    /// Where this message type is defined; serialized via its compact
    /// [`Display`](crate::Origin) — a single string like `mycrate src/lib.rs:12:1`,
    /// not a nested map (serde via `serde::origin`, facet via the proxy on [`Origin`]).
    #[cfg_attr(feature = "serde", serde(serialize_with = "serde::origin"))]
    pub origin: Origin,
    /// Routing tags: sorted, deduped, lowercased. The common (untagged) case is
    /// empty and stays quiet in the manifest.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "tags_empty"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = tags_empty))]
    pub tags: &'static [&'static str],
    /// Runtime identity — the join key from a live `&dyn Message` via
    /// `m.as_any().type_id()`. Opaque and build-local, so excluded from
    /// serialization.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub type_id: TypeId,
}

impl MessageDescriptor {
    /// The resolved join key: the explicit [`id`](Self::id) if present, else
    /// [`msg`](Self::msg). This is the value [`Message::id`](crate::Message::id)
    /// returns for a live instance and the value [`duplicates`] walks.
    pub fn resolved_id(&self) -> &'static str {
        self.id.unwrap_or(self.msg)
    }
}

impl LevelName {
    /// The canonical level name, e.g. `"WARN"` — a `&'static str` (which
    /// `AsRef<str>` could not preserve).
    pub const fn as_str(&self) -> &'static str {
        match self {
            LevelName::TRACE => "TRACE",
            LevelName::DEBUG => "DEBUG",
            LevelName::INFO => "INFO",
            LevelName::WARN => "WARN",
            LevelName::ERROR => "ERROR",
        }
    }
}

impl From<LevelName> for Level {
    fn from(name: LevelName) -> Self {
        match name {
            LevelName::TRACE => Level::TRACE,
            LevelName::DEBUG => Level::DEBUG,
            LevelName::INFO => Level::INFO,
            LevelName::WARN => Level::WARN,
            LevelName::ERROR => Level::ERROR,
        }
    }
}

/// Facet proxy for the `meta` k/v slices: reflecting a `&[(k, v)]` directly
/// yields an array of pairs, but the manifest wants a map — so the `meta` fields
/// proxy through this `BTreeMap` newtype, which facet renders as an object (serde
/// gets the same shape from `serde::pairs`). Serialize-only; the catalogue is
/// never deserialized, so the reverse conversion is a stub.
#[cfg(feature = "facet")]
#[derive(::facet::Facet)]
#[facet(transparent)]
struct MetaMap(std::collections::BTreeMap<&'static str, &'static str>);

// Infallible, but facet's `proxy` mechanism is defined in terms of `TryFrom`.
#[cfg(feature = "facet")]
#[allow(clippy::infallible_try_from)]
impl TryFrom<&&'static [(&'static str, &'static str)]> for MetaMap {
    type Error = core::convert::Infallible;
    fn try_from(pairs: &&'static [(&'static str, &'static str)]) -> Result<Self, Self::Error> {
        Ok(MetaMap(pairs.iter().copied().collect()))
    }
}

#[cfg(feature = "facet")]
impl TryFrom<MetaMap> for &'static [(&'static str, &'static str)] {
    type Error = &'static str;
    fn try_from(_: MetaMap) -> Result<Self, Self::Error> {
        Err("the catalogue is serialize-only")
    }
}

/// Omit-when-empty predicates shared by serde and facet (both keep the common
/// unannotated/untagged case quiet in the manifest).
#[cfg(any(feature = "serde", feature = "facet"))]
fn pairs_empty(pairs: &&'static [(&'static str, &'static str)]) -> bool {
    pairs.is_empty()
}

#[cfg(any(feature = "serde", feature = "facet"))]
fn tags_empty(tags: &&'static [&'static str]) -> bool {
    tags.is_empty()
}

/// The serde-specific serialization shims, grouped behind one gate. A *local*
/// module deliberately named `serde` so the attribute paths above read as
/// `serde::pairs` etc.; it shadows the crate inside this file, which is why the
/// derives above name the crate absolutely (`::serde`).
#[cfg(feature = "serde")]
mod serde {
    use ::serde::{Serializer, ser::SerializeMap};

    use crate::Origin;

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
}

/// Iterate every registered message type. Populated at load time (before
/// `main` natively; via `__wasm_call_ctors` on wasm). Joinable to emitted
/// events by `MSG`.
pub fn all() -> impl Iterator<Item = &'static MessageDescriptor> {
    inventory::iter::<MessageDescriptor>.into_iter()
}

/// Every resolved [`id`](MessageDescriptor::resolved_id) claimed by more than
/// one message type, mapped to the descriptors that claim it. The resolved id is
/// the catalogue's unique join key, so an empty map is the invariant (an explicit
/// id colliding with another message's fallback text counts as a duplicate). A
/// proc macro cannot enforce this (each expansion is blind to its siblings), so
/// the check is a runtime walk; its natural home is a test in the *application*
/// crate, where every linked-in message is registered:
///
/// ```ignore
/// #[test]
/// fn catalogue_ids_are_unique() {
///     let dupes = tracing_wide::catalogue::duplicates();
///     assert!(dupes.is_empty(), "{dupes:#?}");
/// }
/// ```
///
/// The descriptors carry [`origin`](MessageDescriptor::origin), so that failure
/// names the crate, file and line of each colliding definition — the detail a
/// bare id cannot give when the collision spans two crates. Both the map and each
/// entry's descriptors are ordered (by id, then by definition site), so the report
/// is stable across runs.
pub fn duplicates() -> BTreeMap<&'static str, Vec<&'static MessageDescriptor>> {
    let mut by_id: BTreeMap<&'static str, Vec<&'static MessageDescriptor>> = BTreeMap::new();
    for descriptor in all() {
        by_id
            .entry(descriptor.resolved_id())
            .or_default()
            .push(descriptor);
    }

    by_id.retain(|_, descriptors| descriptors.len() > 1);

    // Link order is not stable across builds; the definition site is. Two messages
    // expanded from one `macro_rules!` share a call-site span, so `msg` breaks the
    // remaining ties.
    for descriptors in by_id.values_mut() {
        descriptors
            .sort_unstable_by_key(|d| (d.origin.file, d.origin.line, d.origin.column, d.msg));
    }

    by_id
}

inventory::collect!(MessageDescriptor);
