//! A private fixture crate that contributes messages to the catalogue from
//! *outside* `tracing-wide`, so the integration tests can prove a subscriber
//! sees foreign messages — `origin().krate`, `tags()` — without naming their
//! types. It makes no feature demands on `tracing-wide` (`default-features =
//! false`), inheriting whatever the test build's feature powerset selects.

use tracing_wide::{event, message};

#[message(msg = "access granted", tags = ["security"])]
pub struct AccessGranted {
    pub user: &'static str,
}

#[message(msg = "record written", tags = ["persist"])]
pub struct RecordWritten {
    pub key: &'static str,
}

pub fn grant(user: &'static str) {
    event!(AccessGranted { user });
}

pub fn write(key: &'static str) {
    event!(RecordWritten { key });
}
