```console
$ cargo run -q --example sink-serde --features serde,sink
[forward] {"fields":{"attempt":1,"service":"billing"},"level":"WARN","msg":"service started"}
[forward] {"fields":{"duration_ms":12,"route":"/users/:id"},"level":"INFO","msg":"request completed"}

```
