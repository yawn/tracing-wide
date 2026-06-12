// `__private::MessageBehaviour` is pseudo-sealed via the `__private::Sealed`
// supertrait: only `#[message]` emits `impl Sealed`, so an accidental
// hand-written impl (lacking the matching `impl Sealed`) can't compile.
struct Sneaky;

impl tracing_wide::__private::MessageBehaviour for Sneaky {
    fn record(&self) {}
}

fn main() {}
