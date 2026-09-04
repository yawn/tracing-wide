// Metadata keys must be identifiers — a path is not one.
#[tracing_wide::message(msg = "x", a::b = "c")]
struct M {
    a: usize,
}

fn main() {}
