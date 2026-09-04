// `id` must be a string literal.
#[tracing_wide::message(id = 5)]
struct M {
    a: usize,
}

fn main() {}
