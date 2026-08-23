//! A private fixture crate that contributes messages to the catalogue from
//! *outside* `tracing-wide`, so the integration tests can prove a subscriber
//! sees foreign messages — `origin().krate`, `tags()` — without naming their
//! types. It makes no feature demands on `tracing-wide` (`default-features =
//! false`), inheriting whatever the test build's feature powerset selects.

// Sized for the `define_very_wide!` width below, with room to spare. Raise the
// two together; `tests/core.rs` in `tracing-wide` pins the *unraised* ceiling.
#![recursion_limit = "1024"]

use seq_macro::seq;
use tracing_wide::{event, message};

#[message(msg = "access granted", tags = ["security"])]
pub struct AccessGranted {
    pub user: &'static str,
}

#[message(msg = "record written", tags = ["persist"])]
pub struct RecordWritten {
    pub key: &'static str,
}

/// Defines the very-wide fixture and its field count from one width literal.
macro_rules! define_very_wide {
    ($count:literal) => {
        seq!(N in 0..$count {
            /// A message far wider than rustc's default `recursion_limit`
            /// permits.
            #[message(msg = "very wide event", tags = ["persist"])]
            pub struct VeryWide {
                #( pub f~N: usize, )*
            }

            impl VeryWide {
                fn seeded() -> Self {
                    Self { #( f~N: N, )* }
                }
            }
        });

        /// Field count of [`VeryWide`]. Public so `cross_crate.rs` asserts
        /// against one definition of the width instead of repeating the literal.
        pub const VERY_WIDE_FIELDS: usize = $count;
    };
}

define_very_wide!(500);

/// Emits [`VeryWide`] with field `fN` seeded to `N`, so a field the macro maps
/// to the wrong slot fails on the value and not only on a missing name.
pub fn emit_very_wide() {
    event!(VeryWide::seeded());
}

pub fn grant(user: &'static str) {
    event!(AccessGranted { user });
}

pub fn write(key: &'static str) {
    event!(RecordWritten { key });
}
