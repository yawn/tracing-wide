// Each tag must be a string literal.
#[tracing_wide::message(msg = "x", tags = [1])]
struct M {
    a: usize,
}

fn main() {}
