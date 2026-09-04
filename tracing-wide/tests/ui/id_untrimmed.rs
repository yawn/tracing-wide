// `id` must not have leading or trailing whitespace.
#[tracing_wide::message(msg = "x", id = " checkout.started ")]
struct M {
    a: usize,
}

fn main() {}
