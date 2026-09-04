// A metadata key may appear once — the catalogue stores pairs, not a multimap.
#[tracing_wide::message(msg = "x", owner = "a", owner = "b")]
struct M {
    a: usize,
}

fn main() {}
