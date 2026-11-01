// A metadata key may appear once, on the message and on a field — the catalogue stores pairs, not a multimap.
#[tracing_wide::message(msg = "x", owner = "a", owner = "b")]
struct M {
    a: usize,
}

#[tracing_wide::message(msg = "y")]
struct F {
    #[field(unit = "ms", unit = "s")]
    a: usize,
}

fn main() {}
