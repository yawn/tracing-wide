// Generic parameters are rejected up front: the generated impls and the
// catalogue's `TypeId::of` need one concrete `'static` type.
#[tracing_wide::message(msg = "x")]
struct M<T> {
    a: T,
}

fn main() {}
