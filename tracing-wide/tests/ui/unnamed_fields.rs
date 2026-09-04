// A message needs named fields — each field name becomes a recorded key.
#[tracing_wide::message(msg = "x")]
struct M(usize);

fn main() {}
