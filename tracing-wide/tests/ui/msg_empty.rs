// `msg` must not be empty: it is the fallback `id`.
#[tracing_wide::message(msg = "")]
struct M {
    a: usize,
}

fn main() {}
