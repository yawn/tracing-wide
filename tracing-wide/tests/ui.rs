//! Compile-fail coverage: the *negative* guarantees the behavioral suite can't
//! express — the pseudo-seals reject accidental hand-written impls, the `Field`
//! bound rejects bad field types, `event!` rejects non-messages, and
//! `#[message]` validates its arguments. trybuild shells out to cargo, so this
//! is native-only.
#![cfg(not(target_arch = "wasm32"))]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
