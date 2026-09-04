// `tags` must be an array of string literals.
#[tracing_wide::message(msg = "x", tags = "security")]
struct M {
    a: usize,
}

fn main() {}
