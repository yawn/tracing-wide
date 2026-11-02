//! The catalogue: one descriptor per message type, registered by `#[message]` (feature `catalogue`).

use core::any::TypeId;
use core::fmt;
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
    /// Read through [`type_name`](Self::type_name).
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    #[cfg_attr(feature = "facet", facet(rename = "type"))]
    type_name: TypeName,
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

/// A field type's [`core::any::type_name`].
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
#[cfg_attr(feature = "facet", facet(proxy = &'static str))]
struct TypeName(#[cfg_attr(feature = "facet", facet(opaque))] fn() -> &'static str);

impl FieldDescriptor {
    /// `#[message]`-internal: `type_name` is `|| core::any::type_name::<T>()` for the field type `T`.
    #[doc(hidden)]
    pub const fn __new(
        deprecated: Option<&'static str>,
        doc: Option<&'static str>,
        meta: Meta,
        name: &'static str,
        type_name: fn() -> &'static str,
    ) -> Self {
        Self {
            deprecated,
            doc,
            meta,
            name,
            type_name: TypeName(type_name),
        }
    }

    /// The field type, resolved by [`core::any::type_name`] (e.g. `"&str"`, `"core::option::Option<usize>"`).
    /// The text can change between compiler versions.
    pub fn type_name(&self) -> &'static str {
        // This can be removed when type_name becomes const
        self.type_name.get()
    }
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

impl TypeName {
    /// The name.
    fn get(&self) -> &'static str {
        (self.0)()
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

/// The name, not the fn pointer's address.
impl fmt::Debug for TypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get(), f)
    }
}

/// Serializes as the name.
#[cfg(feature = "serde")]
impl ::serde::Serialize for TypeName {
    fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.get())
    }
}

/// Facet serializes a `TypeName` as the name (serialize-only).
// Infallible, but facet's `proxy` mechanism is defined in terms of `TryFrom`.
#[cfg(feature = "facet")]
#[allow(clippy::infallible_try_from)]
impl TryFrom<&TypeName> for &'static str {
    type Error = core::convert::Infallible;
    fn try_from(type_name: &TypeName) -> Result<Self, Self::Error> {
        Ok(type_name.get())
    }
}

#[cfg(feature = "facet")]
impl TryFrom<&'static str> for TypeName {
    type Error = &'static str;
    fn try_from(_: &'static str) -> Result<Self, Self::Error> {
        Err("TypeName is serialize-only in the catalogue")
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
/// ```
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
