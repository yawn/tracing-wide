//! The text of `catalogue::snapshot`; the fixtures here are the whole catalogue of this binary.

#![cfg(feature = "catalogue")]

use tracing_wide::{catalogue, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

type Cents = u64;

/// Recorded when a customer is charged.
/// The second line has a "quote" and a \ backslash.
#[message(msg = "payment charged", id = "billing.payment_charged", owner = "billing", tags = ["billing", "audit"])]
#[allow(dead_code)]
struct Charged {
    #[field(unit = "cents")]
    amount: Cents,
    #[deprecated = "use `amount`"]
    legacy: Option<usize>,
    /// The route template.
    route: &'static str,
}

#[message(msg = "service started", level = warn)]
#[deprecated]
#[allow(dead_code)]
struct Started {
    attempt: usize,
}

/// A blank line before each message, one fact per line, all tags on one line: aliases resolved, values escaped.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn snapshot_has_one_sorted_fact_per_line() {
    let snapshot = catalogue::snapshot().unwrap();
    let (header, body) = snapshot.split_once('\n').unwrap();

    let version: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').take(2).collect();

    assert_eq!(
        body,
        r#"
"billing.payment_charged" field "amount" meta "unit" "cents"
"billing.payment_charged" field "amount" type "u64"
"billing.payment_charged" field "legacy" deprecated "use `amount`"
"billing.payment_charged" field "legacy" type "core::option::Option<usize>"
"billing.payment_charged" field "route" doc "The route template."
"billing.payment_charged" field "route" type "&str"
"billing.payment_charged" message doc "Recorded when a customer is charged.\nThe second line has a \"quote\" and a \\ backslash."
"billing.payment_charged" message level INFO
"billing.payment_charged" message meta "owner" "billing"
"billing.payment_charged" message msg "payment charged"
"billing.payment_charged" message tags "audit" "billing"

"service started" field "attempt" type "usize"
"service started" message deprecated "true"
"service started" message level WARN
"service started" message msg "service started"
"#
    );
    assert_eq!(
        header,
        format!("tracing-wide catalogue {}", version.join("."))
    );
}
