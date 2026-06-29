```console
$ cargo run -q --example catalogue-serde --features catalogue,serde
messages:
  request completed:
    doc: Recorded when a request finishes handling.
    fields:
    - meta:
        unit: ms
      name: duration
      type: usize
    - doc: Route template, e.g. `/users/:id`.
      name: route
      type: '& ''static str'
    level: INFO
    meta:
      owner: api
    origin: tracing-wide tracing-wide/examples/catalogue-serde.rs:19:8
    tags:
    - api
  service started:
    doc: |-
      Emitted when a service finishes starting up.

      If services start up at all - we had lots of quality issues lately.
    fields:
    - meta:
        unit: count
      name: attempt
      type: usize
    - deprecated: fold the id into `service`
      name: legacy_id
      type: Option < usize >
    - doc: Logical name of the service.
      name: service
      type: '& ''static str'
    level: WARN
    meta:
      owner: platform
    origin: tracing-wide tracing-wide/examples/catalogue-serde.rs:31:8
    tags:
    - platform
    - startup
tags:
- api
- platform
- startup

```
