//! Coverage for the serde hook (`serde` feature): `Message::as_serialize` yields
//! `Some` only for a `#[derive(Serialize)]` message, and serializes the body
//! whole without naming the concrete type.

#![cfg(feature = "serde")]

use serde::Serialize;
use tracing_wide::{Message, message};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Does *not* opt in — and isn't `Serialize`. Proves the hook never forces
/// a `Serialize` bound on messages that didn't ask for it.
#[message(msg = "opaque event")]
#[allow(dead_code)]
struct Opaque {
    n: usize,
}

/// Enables the erased hook with nothing but `#[derive(Serialize)]`.
#[message(msg = "serializable event")]
#[derive(Serialize)]
struct Ser {
    n: usize,
    name: &'static str,
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn opted_in_message_serializes_through_dyn_message() {
    let m = Ser {
        name: "billing",
        n: 5,
    };

    let dynamic: &dyn Message = &m;

    let erased = dynamic
        .as_serialize()
        .expect("an opted-in message yields Some");

    let value = serde_json::to_value(erased).unwrap();

    assert_eq!(value, serde_json::json!({ "name": "billing", "n": 5 }));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn opted_out_message_returns_none() {
    let m = Opaque { n: 1 };
    let dynamic: &dyn Message = &m;

    assert!(dynamic.as_serialize().is_none());
}
