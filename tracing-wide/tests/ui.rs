//! Compile-fail coverage: the *negative* guarantees the behavioral suite can't
//! express — the pseudo-seals reject accidental hand-written impls, the `Field`
//! bound rejects bad field types, `event!` rejects non-messages, and
//! `#[message]` validates its arguments. trybuild shells out to cargo, so this
//! is native-only.
//!
//! **`tests/ui/` is the index of validation rules.** One fixture per rule, named
//! `<subject>_<violation>`, with the rule restated as the file's first comment
//! and its exact diagnostic pinned in the neighbouring `.stderr`. So
//! `ls tests/ui/` answers "what does `#[message]` reject?", and a new rule is
//! not finished until it has a fixture here. The parsers in
//! `tracing-wide-macros` (`parse_msg`, `parse_id`, `parse_level`, `Tags::parse`,
//! `MetaPairs::push`) are the other half: each states its rules in one place,
//! and every one of them is covered below.
#![cfg(not(target_arch = "wasm32"))]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
