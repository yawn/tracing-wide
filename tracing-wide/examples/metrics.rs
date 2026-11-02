//! OpenTelemetry metrics derived from messages in a sink.
//!
//! The app owns the meter provider and flushes it on `shutdown()`; the sink holds only the instrument handles.
//!
//! Run: `just example-metrics`

use opentelemetry::{
    KeyValue,
    metrics::{Counter, Gauge, Histogram, MeterProvider},
};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_stdout::MetricExporter;
use tracing_wide::{
    Message, event, message,
    sink::{Sink, Sinks},
};

/// Maps messages tagged `metrics` to instruments; labels come from low-cardinality fields only.
struct Metrics {
    queue_depth: Gauge<u64>,
    request_duration: Histogram<u64>,
    requests: Counter<u64>,
}

/// Carries the current value: a sync gauge is only as fresh as the last such event.
#[message(msg = "queue sampled", tags = ["metrics"])]
struct QueueSampled {
    depth: u64,
}

#[message(msg = "request completed", tags = ["metrics"])]
struct RequestCompleted {
    duration_ms: u64,
    route: &'static str,
}

/// Not tagged `metrics`: the sink skips it before any downcast.
#[message(msg = "user signed in")]
struct UserSignedIn {
    user_id: u64,
}

impl Sink for Metrics {
    fn on_message(&self, m: &dyn Message) {
        if !m.tags().contains("metrics") {
            return;
        }

        let any = m.as_any();

        if let Some(req) = any.downcast_ref::<RequestCompleted>() {
            let labels = [KeyValue::new("route", req.route)];

            self.requests.add(1, &labels);
            self.request_duration.record(req.duration_ms, &labels);
        } else if let Some(sample) = any.downcast_ref::<QueueSampled>() {
            self.queue_depth.record(sample.depth, &[]);
        }
    }
}

fn main() {
    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(MetricExporter::default())
        .build();

    let meter = provider.meter("tracing-wide-example");

    let mut sinks = Sinks::default();

    sinks.register(Box::new(Metrics {
        queue_depth: meter.u64_gauge("queue.depth").build(),
        request_duration: meter
            .u64_histogram("request.duration")
            .with_unit("ms")
            .build(),
        requests: meter.u64_counter("requests").build(),
    }));

    sinks.install().ok();

    event!(RequestCompleted {
        route: "/users/:id",
        duration_ms: 12,
    });

    event!(QueueSampled { depth: 3 });

    event!(UserSignedIn { user_id: 42 });

    provider.shutdown().expect("metrics flush on shutdown");
}
