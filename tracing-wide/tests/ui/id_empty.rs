// `id` must not be empty.
#[tracing_wide::message(msg = "x", id = "")]
struct M {
    a: usize,
}

fn main() {}
