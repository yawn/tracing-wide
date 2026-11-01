```console
$ cargo run -q --example tags --features sink
[db]        store   "payment captured"  (tracing-wide tracing-wide/examples/tags.rs:30:8)
[analytics] forward "payment captured"  (tracing-wide tracing-wide/examples/tags.rs:30:8)
[analytics] forward "page viewed"  (tracing-wide tracing-wide/examples/tags.rs:24:8)
[db]        store   "config changed"  (tracing-wide tracing-wide/examples/tags.rs:18:8)

```
