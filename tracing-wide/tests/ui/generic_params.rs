// A message must be one concrete `'static` type, so `#[message]` rejects generics of every kind.
#[tracing_wide::message(msg = "x")]
struct M<T> {
    a: T,
}

#[tracing_wide::message(msg = "y")]
struct L<'a> {
    a: &'a str,
}

#[tracing_wide::message(msg = "z")]
struct C<const N: usize> {
    a: usize,
}

fn main() {}
