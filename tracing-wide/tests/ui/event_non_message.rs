// `event!` only accepts a `Message`; a bare struct is rejected.
struct NotAMessage;

fn main() {
    tracing_wide::event!(NotAMessage);
}
