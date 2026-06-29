```console
$ cargo run -q --example instrument --features instrument,subscriber
 INFO handle{attempt=3 component="billing"}:work{attempt=3}: instrument: work finished attempt=3 component="billing" payload=42
[parked] all values present: payload=42 component=Some("billing") attempt=Some(3)

```
