// Surrounding whitespace is invisible in a manifest and in a sink's event name,
// so an untrimmed `id` is a compile error rather than a normalization.
#[tracing_wide::message(msg = "x", id = " checkout.started ")]
struct M {
    a: usize,
}

fn main() {}
