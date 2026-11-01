//! Compile-fail tests: the seals, the `Field` bound, `event!` input, and `#[message]` arguments. Native only.
#![cfg(not(target_arch = "wasm32"))]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
