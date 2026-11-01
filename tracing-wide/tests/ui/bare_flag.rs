// `#[message]` takes no bare flags; `#[derive(Serialize)]` opts into serialization.
#[tracing_wide::message(serialize)]
struct M {
    a: usize,
}

fn main() {}
