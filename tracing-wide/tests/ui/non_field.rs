// A type without a `tracing::field::Value` impl is not a `Field`.
struct NotAValue;

#[tracing_wide::message(msg = "bad")]
struct Bad {
    thing: NotAValue,
}

fn main() {}
