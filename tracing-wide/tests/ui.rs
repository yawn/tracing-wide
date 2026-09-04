//! Compile-fail tests: the seals, the `Field` bound, `event!` input, and `#[message]` arguments. Native only.
//!
//! `tests/ui/` indexes the validation rules: one fixture per rule, named `<subject>_<violation>`, with its
//! diagnostic pinned in the `.stderr`.
#![cfg(not(target_arch = "wasm32"))]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
