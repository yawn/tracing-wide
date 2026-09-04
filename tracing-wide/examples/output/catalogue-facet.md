```console
$ cargo run -q --example catalogue-facet --features catalogue,facet
---
messages: 
  billing.payment_charged: 
    doc: Recorded when a customer is charged; the explicit `id` keeps the key stable when the text changes.
    fields: 
      - meta: 
          unit: cents
        name: amount
        type: usize
    level: INFO
    meta: 
      owner: billing
    origin: "tracing-wide tracing-wide/examples/catalogue-facet.rs:12:8"
    tags: 
      - billing
  request completed: 
    doc: Recorded when a request finishes handling.
    fields: 
      - meta: 
          unit: ms
        name: duration
        type: usize
      - doc: "Route template, e.g. `/users/:id`."
        name: route
        type: "& 'static str"
    level: INFO
    meta: 
      owner: api
    origin: "tracing-wide tracing-wide/examples/catalogue-facet.rs:20:8"
    tags: 
      - api
  service started: 
    doc: Emitted when a service finishes starting up.
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
        type: "& 'static str"
    level: WARN
    meta: 
      owner: platform
    origin: "tracing-wide tracing-wide/examples/catalogue-facet.rs:30:8"
    tags: 
      - platform
      - startup
tags: 
  - api
  - billing
  - platform
  - startup

```
