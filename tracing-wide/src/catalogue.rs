//! The catalogue: one descriptor per message type, registered by `#[message]` (feature `catalogue`).

use core::any::TypeId;
use std::collections::BTreeMap;

use tracing::Level;

use crate::{Meta, Origin, Tags};

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
    /// The pairs of `#[field(key = ...)]`: a map, not serialized when empty.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Meta::is_empty"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Meta::is_empty))]
    pub meta: Meta,
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
    /// The explicit `#[message(id = "...")]`, if any; not serialized, because the manifest keys its entries by
    /// [`resolved_id`](Self::resolved_id).
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub id: Option<&'static str>,
    /// Severity, recorded as its canonical name: see [`LevelName`].
    pub level: LevelName,
    /// The pairs of `#[message(key = ...)]` other than `msg`, `id`, `level`, `tags`: a map, not serialized when empty.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Meta::is_empty"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Meta::is_empty))]
    pub meta: Meta,
    /// The event text and the fallback id; not serialized, because the manifest keys its entries by the resolved id.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub msg: &'static str,
    /// Where the type is defined; serialized as its `Display` string.
    #[cfg_attr(feature = "serde", serde(serialize_with = "serde::origin"))]
    pub origin: Origin,
    /// Routing tags; not serialized when empty.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Tags::is_empty"))]
    #[cfg_attr(feature = "facet", facet(skip_serializing_if = Tags::is_empty))]
    pub tags: Tags,
    /// Joins a live message by `m.as_any().type_id()`; build-local, so not serialized.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "facet", facet(skip))]
    pub type_id: TypeId,
}

impl MessageDescriptor {
    /// The explicit [`id`](Self::id), else [`msg`](Self::msg): the value of [`Message::id`](crate::Message::id).
    pub fn resolved_id(&self) -> &'static str {
        self.id.unwrap_or(self.msg)
    }
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

/// Serde shims; the name shadows the `serde` crate, so the derives write `::serde`.
#[cfg(feature = "serde")]
mod serde {
    use ::serde::Serializer;

    use crate::Origin;

    /// Render an [`Origin`] as its compact `Display` string.
    pub(super) fn origin<S: Serializer>(origin: &Origin, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(origin)
    }
}

/// Every registered message type; registered before `main` (on wasm, by `__wasm_call_ctors`).
pub fn all() -> impl Iterator<Item = &'static MessageDescriptor> {
    inventory::iter::<MessageDescriptor>.into_iter()
}

/// The resolved ids that more than one message type claims, each with its descriptors; must be empty. Check it in an
/// application test:
///
/// ```ignore
/// let dupes = tracing_wide::catalogue::duplicates();
/// assert!(dupes.is_empty(), "{dupes:#?}");
/// ```
///
/// The map is ordered by id, and each list by definition site.
pub fn duplicates() -> BTreeMap<&'static str, Vec<&'static MessageDescriptor>> {
    let mut by_id: BTreeMap<&'static str, Vec<&'static MessageDescriptor>> = BTreeMap::new();
    for descriptor in all() {
        by_id
            .entry(descriptor.resolved_id())
            .or_default()
            .push(descriptor);
    }

    by_id.retain(|_, descriptors| descriptors.len() > 1);

    // Link order varies between builds; `msg` breaks ties within one `macro_rules!` expansion.
    for descriptors in by_id.values_mut() {
        descriptors
            .sort_unstable_by_key(|d| (d.origin.file, d.origin.line, d.origin.column, d.msg));
    }

    by_id
}

inventory::collect!(MessageDescriptor);
