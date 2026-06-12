// Tags must be lowercase — an uppercase tag is a compile error (provenance is
// automatic via `origin()`, so tags only carry a lowercase-normalized label).
#[tracing_wide::message(msg = "x", tags = ["Security"])]
struct M {
    a: usize,
}

fn main() {}
