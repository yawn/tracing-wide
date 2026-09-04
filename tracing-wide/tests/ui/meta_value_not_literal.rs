// Metadata values must be literals (str/int/bool/float).
#[tracing_wide::message(msg = "x", owner = SOME_CONST)]
struct M {
    a: usize,
}

fn main() {}
