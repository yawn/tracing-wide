// `#[message]` takes no bare flags. The old `#[message(serialize)]` flag is gone —
// serialization is keyed on `#[derive(Serialize)]` — so a bare flag is an error.
#[tracing_wide::message(serialize)]
struct M {
    a: usize,
}

fn main() {}
