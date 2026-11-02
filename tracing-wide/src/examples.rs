//! The runnable programs of `examples/`: one module per example, with its source and its output.

/// Declares one module per example: its summary, the source of `examples/<file>.rs`, and its recorded output.
macro_rules! examples {
    ($($module:ident = $file:literal => $summary:literal;)*) => {$(
        #[doc = $summary]
        #[doc = ""]
        #[doc = "```rust,ignore"]
        #[doc = include_str!(concat!("../examples/", $file, ".rs"))]
        #[doc = "```"]
        #[doc = ""]
        #[doc = include_str!(concat!("../examples/output/", $file, ".md"))]
        pub mod $module {}
    )*};
}

examples! {
    catalogue_facet = "catalogue-facet" => "Dump the catalogue as YAML via `facet-yaml`.";
    catalogue_serde = "catalogue-serde" => "Dump the catalogue as YAML via `serde`.";
    catalogue_snapshot = "catalogue-snapshot" => "Print the catalogue snapshot to commit and diff in CI.";
    instrument = "instrument" => "Fill `Option` fields from surrounding spans.";
    metrics = "metrics" => "Derive OpenTelemetry metrics from messages in a sink.";
    sink = "sink" => "A generic line printer plus a typed downcasting sink.";
    sink_facet = "sink-facet" => "Filter on a live field value via `as_facet`.";
    sink_serde = "sink-serde" => "Forward each event as JSON via `as_serialize`.";
    tags = "tags" => "Route messages by tag with no downcast.";
}
