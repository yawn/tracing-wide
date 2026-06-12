// `level` must be one of trace/debug/info/warn/error.
#[tracing_wide::message(msg = "x", level = bogus)]
struct M {
    a: usize,
}

fn main() {}
