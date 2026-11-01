//! The catalogue: one descriptor per message type, registered by `#[message]` (feature `catalogue`).

use core::any::TypeId;
use std::collections::HashMap;

use tracing::Level;

use crate::Origin;

/// One field's catalogue metadata.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize))]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
pub struct FieldDescriptor {
    /// The `#[deprecated]` reason, `"true"` without one; not serialized when `None`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub deprecated: Option<&'static str>,
    /// The field's `///` doc comments, harvested at compile time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub doc: Option<&'static str>,
    /// The pairs of `#[field(key = ...)]`, serialized as a map.
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

/// A serializable mirror of `tracing::Level`, with its exact names; `Level::from` converts back.
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
    /// The `#[deprecated]` reason, `"true"` without one; not serialized when `None`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub deprecated: Option<&'static str>,
    /// The struct's `///` doc comments, harvested at compile time.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Option::is_none))]
    pub doc: Option<&'static str>,
    pub fields: &'static [FieldDescriptor],
    /// Severity, recorded as its canonical name: see [`LevelName`].
    pub level: LevelName,
    /// The pairs of `#[message(key = ...)]` other than `msg`, `level`, and `tags`, serialized as a map.
    #[cfg_attr(
        feature = "serde",
        serde(skip_serializing_if = "pairs_empty", serialize_with = "serde::pairs")
    )]
    #[cfg_attr(feature = "facet", facet(proxy = MetaMap, skip_serializing_if = pairs_empty))]
    pub meta: &'static [(&'static str, &'static str)],
    /// The unique join key; not serialized, because a serialized catalogue keys its entries by it.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub msg: &'static str,
    /// Where the type is defined; serialized as its `Display` string.
    #[cfg_attr(feature = "serde", serde(serialize_with = "serde::origin"))]
    pub origin: Origin,
    /// Routing tags: sorted, deduped, lowercase; not serialized when empty.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "tags_empty"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = tags_empty))]
    pub tags: &'static [&'static str],
    /// Joins a live message by `m.as_any().type_id()`; build-local, so not serialized.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub type_id: TypeId,
}

impl LevelName {
    /// The canonical name, e.g. `"WARN"`.
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

/// Facet proxy that renders the `meta` pairs as a map; serialize-only.
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

/// Omit-when-empty predicates for serde and facet.
#[cfg(any(feature = "serde", feature = "facet"))]
fn pairs_empty(pairs: &&'static [(&'static str, &'static str)]) -> bool {
    pairs.is_empty()
}

#[cfg(any(feature = "serde", feature = "facet"))]
fn tags_empty(tags: &&'static [&'static str]) -> bool {
    tags.is_empty()
}

/// Serde shims; the name shadows the `serde` crate, so the derives write `::serde`.
#[cfg(feature = "serde")]
mod serde {
    use ::serde::{Serializer, ser::SerializeMap};

    use crate::Origin;

    /// Render an [`Origin`] as its compact `Display` string.
    pub(super) fn origin<S: Serializer>(origin: &Origin, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(origin)
    }

    /// Render the `meta` pairs as a map.
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

/// Every registered message type; registered before `main` (on wasm, by `__wasm_call_ctors`).
pub fn all() -> impl Iterator<Item = &'static MessageDescriptor> {
    inventory::iter::<MessageDescriptor>.into_iter()
}

/// The `msg` values that more than one message type registers; must be empty. Check it in an application test:
///
/// ```ignore
/// assert_eq!(tracing_wide::catalogue::duplicates(), Vec::<&str>::new());
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
