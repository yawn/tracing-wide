// tracing merges a field named `message` into the event text, so `#[message]` rejects it.
#[tracing_wide::message(msg = "x")]
struct M {
    message: usize,
}

fn main() {}
