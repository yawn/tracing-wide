// `level` takes a bare ident or a string literal, never another literal kind.
#[tracing_wide::message(msg = "x", level = 5)]
struct M {
    a: usize,
}

fn main() {}
