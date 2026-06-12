// `msg` must be a string literal.
#[tracing_wide::message(msg = 5)]
struct M {
    a: usize,
}

fn main() {}
