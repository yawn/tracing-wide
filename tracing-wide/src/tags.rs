//! The static routing tags of a message type.

use core::iter::Copied;
use core::slice;

use crate::precedes;

/// Static routing tags of a message type: lowercase, sorted, each tag once.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "facet", derive(::facet::Facet), facet(transparent))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize), serde(transparent))]
pub struct Tags(&'static [&'static str]);

impl Tags {
    /// `#[message]`-internal: wraps tags that are sorted and unique, else panics (a compile error in a `const`).
    #[doc(hidden)]
    pub const fn __new(tags: &'static [&'static str]) -> Self {
        let mut i = 1;

        while i < tags.len() {
            assert!(
                precedes(tags[i - 1], tags[i]),
                "`Tags` must be sorted and unique"
            );
            i += 1;
        }

        Self(tags)
    }

    /// Whether `tag` is one of the tags.
    pub fn contains(&self, tag: &str) -> bool {
        self.iter().any(|t| t == tag)
    }

    /// `true` when there are no tags.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The tags, in sorted order.
    pub fn iter(&self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl IntoIterator for Tags {
    type IntoIter = Copied<slice::Iter<'static, &'static str>>;
    type Item = &'static str;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().copied()
    }
}
