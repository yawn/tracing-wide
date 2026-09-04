// An empty `id` would resolve to an empty join key — rejected rather than
// silently falling back to `msg`.
#[tracing_wide::message(msg = "x", id = "")]
struct M {
    a: usize,
}

fn main() {}
