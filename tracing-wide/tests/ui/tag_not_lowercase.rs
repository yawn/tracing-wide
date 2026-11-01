// Tags must be lowercase.
#[tracing_wide::message(msg = "x", tags = ["Security"])]
struct M {
    a: usize,
}

fn main() {}
