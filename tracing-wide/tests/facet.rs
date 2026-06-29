//! Coverage for the facet reflection hook (`facet` feature): `Message::as_facet`
//! yields a `Peek` only for a `#[derive(Facet)]` message, walkable field-by-name
//! and in declaration order without naming the concrete type.

#![cfg(feature = "facet")]

use tracing_wide::{
    Message,
    facet::{Facet, HasFields},
    message,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// Does *not* opt in — and isn't `Facet`. Proves the hook never forces a
/// `Facet` bound on a message that didn't ask for it.
#[message(msg = "opaque reflect event")]
#[allow(dead_code)]
struct Opaque {
    n: usize,
}

/// Enables the reflection hook with nothing but `#[derive(Facet)]`.
#[message(msg = "reflectable event")]
#[derive(Facet)]
struct Reflect {
    n: usize,
    name: &'static str,
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn opted_in_message_reflects_through_dyn_message() {
    let m = Reflect {
        n: 5,
        name: "billing",
    };

    let dynamic: &dyn Message = &m;

    let body = dynamic
        .as_facet()
        .expect("an opted-in message yields Some")
        .into_struct()
        .expect("a message is a struct");

    // Read individual fields by name, with no knowledge of the concrete type.
    assert_eq!(
        body.field_by_name("name").unwrap().as_str(),
        Some("billing")
    );
    assert_eq!(*body.field_by_name("n").unwrap().get::<usize>().unwrap(), 5);

    // Full reflection: every field, in declaration order.
    let names: Vec<&str> = body.fields().map(|(f, _)| f.name).collect();
    assert_eq!(names, ["n", "name"]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn opted_out_message_returns_none() {
    let m = Opaque { n: 1 };
    let dynamic: &dyn Message = &m;

    assert!(dynamic.as_facet().is_none());
}
