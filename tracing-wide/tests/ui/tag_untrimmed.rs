// A tag must not have leading or trailing whitespace.
#[tracing_wide::message(msg = "x", tags = [" security "])]
struct M {
    a: usize,
}

fn main() {}
