// `#[message]` takes `key = value`, never a list.
#[tracing_wide::message(msg = "x", owner(platform))]
struct M {
    a: usize,
}

fn main() {}
