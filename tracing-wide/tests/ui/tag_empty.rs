// A tag must not be empty.
#[tracing_wide::message(msg = "x", tags = [""])]
struct M {
    a: usize,
}

fn main() {}
