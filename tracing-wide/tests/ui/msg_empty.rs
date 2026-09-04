// `msg` must not be empty — it is the fallback `id`, so an empty `msg` would
// resolve to an empty join key.
#[tracing_wide::message(msg = "")]
struct M {
    a: usize,
}

fn main() {}
