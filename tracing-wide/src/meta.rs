//! The static metadata of a message type or a field.

use core::iter::Copied;
use core::slice;
#[cfg(feature = "facet")]
use std::collections::BTreeMap;

use crate::precedes;

/// Static `key = <literal>` metadata of a message type or a field: values stringified, sorted by key, each key once.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "facet", derive(::facet::Facet))]
#[cfg_attr(feature = "facet", facet(proxy = BTreeMap<&'static str, &'static str>))]
pub struct Meta(&'static [(&'static str, &'static str)]);

impl Meta {
    /// `#[message]`-internal: wraps pairs whose keys are sorted and unique, else panics (a compile error in a `const`).
    #[doc(hidden)]
    pub const fn __new(pairs: &'static [(&'static str, &'static str)]) -> Self {
        let mut i = 1;

        while i < pairs.len() {
            assert!(
                precedes(pairs[i - 1].0, pairs[i].0),
                "`Meta` keys must be sorted and unique"
            );
            i += 1;
        }

        Self(pairs)
    }

    /// The value for `key`, if present.
    pub fn get(&self, key: &str) -> Option<&'static str> {
        self.iter().find_map(|(k, v)| (k == key).then_some(v))
    }

    /// `true` when there are no pairs.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The pairs, in key order.
    pub fn iter(&self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl IntoIterator for Meta {
    type IntoIter = Copied<slice::Iter<'static, (&'static str, &'static str)>>;
    type Item = (&'static str, &'static str);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().copied()
    }
}

/// Serializes as a map.
#[cfg(feature = "serde")]
impl ::serde::Serialize for Meta {
    fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_map(*self)
    }
}

/// Facet serializes a `Meta` as a map (serialize-only).
// Infallible, but facet's `proxy` mechanism is defined in terms of `TryFrom`.
#[cfg(feature = "facet")]
#[allow(clippy::infallible_try_from)]
impl TryFrom<&Meta> for BTreeMap<&'static str, &'static str> {
    type Error = core::convert::Infallible;
    fn try_from(meta: &Meta) -> Result<Self, Self::Error> {
        Ok(meta.iter().collect())
    }
}

#[cfg(feature = "facet")]
impl TryFrom<BTreeMap<&'static str, &'static str>> for Meta {
    type Error = &'static str;
    fn try_from(_: BTreeMap<&'static str, &'static str>) -> Result<Self, Self::Error> {
        Err("Meta is serialize-only in the catalogue")
    }
}
