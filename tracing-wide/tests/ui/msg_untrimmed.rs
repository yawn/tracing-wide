// `msg` must be trimmed, for the same reason as `id`: it is the fallback `id`.
#[tracing_wide::message(msg = " checkout started ")]
struct M {
    a: usize,
}

fn main() {}
