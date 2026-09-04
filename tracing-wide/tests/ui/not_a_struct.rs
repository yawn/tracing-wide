// `#[message]` applies to structs only.
#[tracing_wide::message(msg = "x")]
enum M {
    A,
}

fn main() {}
