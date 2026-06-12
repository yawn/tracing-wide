# `tracing-wide`

> [!CAUTION]
> This is *NOT* an official tokio / tokio-tracing product and / or
> associated crate.

`tracing-wide` is a complementary crate to use with tokio `tracing`. It can be used to emit "wide" events - structs that capture all data that is relevant for any kinds of observability needs at a single site.

The core of this crate can run in WASM environments.
