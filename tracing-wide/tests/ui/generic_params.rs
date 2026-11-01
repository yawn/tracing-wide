// A message must be one concrete `'static` type, so `#[message]` rejects generics.
#[tracing_wide::message(msg = "x")]
struct M<T> {
    a: T,
}

fn main() {}
