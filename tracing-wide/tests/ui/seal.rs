// Only `#[message]` implements `Sealed`, so a hand-written `MessageBehaviour` impl does not compile.
struct Sneaky;

impl tracing_wide::__private::MessageBehaviour for Sneaky {
    fn record(&self) {}
}

fn main() {}
