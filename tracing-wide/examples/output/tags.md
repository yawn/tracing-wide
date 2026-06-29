```console
$ cargo run -q --example tags --features subscriber
[db]        store   "payment captured"  (tracing-wide tracing-wide/examples/tags.rs:35:8)
[analytics] forward "payment captured"  (tracing-wide tracing-wide/examples/tags.rs:35:8)
[analytics] forward "page viewed"  (tracing-wide tracing-wide/examples/tags.rs:29:8)
[db]        store   "config changed"  (tracing-wide tracing-wide/examples/tags.rs:23:8)

```
