// A type with no `tracing::field::Value` impl is not a `tracing_wide::Field`, so
// `#[message]`'s per-field assertion rejects this at compile time.
struct NotAValue;

#[tracing_wide::message(msg = "bad")]
struct Bad {
    thing: NotAValue,
}

fn main() {}
