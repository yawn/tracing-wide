```console
$ cargo run -q --example catalogue-snapshot --features catalogue
tracing-wide catalogue 0.3

"billing.payment_charged" field "amount" meta "unit" "cents"
"billing.payment_charged" field "amount" type "usize"
"billing.payment_charged" message doc "Recorded when a customer is charged; the explicit `id` keeps the key stable when the text changes."
"billing.payment_charged" message level INFO
"billing.payment_charged" message meta "owner" "billing"
"billing.payment_charged" message msg "payment charged"
"billing.payment_charged" message tags "billing"

"request completed" field "duration" meta "unit" "ms"
"request completed" field "duration" type "usize"
"request completed" field "route" doc "Route template, e.g. `/users/:id`."
"request completed" field "route" type "&str"
"request completed" message doc "Recorded when a request finishes handling."
"request completed" message level INFO
"request completed" message meta "owner" "api"
"request completed" message msg "request completed"
"request completed" message tags "api"

"service started" field "attempt" meta "unit" "count"
"service started" field "attempt" type "usize"
"service started" field "legacy_id" deprecated "fold the id into `service`"
"service started" field "legacy_id" type "core::option::Option<usize>"
"service started" field "service" doc "Logical name of the service."
"service started" field "service" type "&str"
"service started" message doc "Emitted when a service finishes starting up."
"service started" message level WARN
"service started" message meta "owner" "platform"
"service started" message msg "service started"
"service started" message tags "platform" "startup"

```
