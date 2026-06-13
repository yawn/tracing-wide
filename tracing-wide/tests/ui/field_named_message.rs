// A field named `message` would merge into the event's message text at the
// tracing handoff (the key is silently lost), so `#[message]` rejects it.
#[tracing_wide::message(msg = "x")]
struct M {
    message: usize,
}

fn main() {}
