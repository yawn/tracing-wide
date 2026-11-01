//! Filtering on a message *field* with facet reflection.
//!
//! Run: `just example-sink-facet`

use tracing_wide::{
    Message, event,
    facet::{Facet, HasFields},
    message,
    sink::{Sink, Sinks},
};

#[message(msg = "request completed", tags = ["http"])]
#[derive(Facet)]
struct RequestCompleted {
    region: &'static str,
    route: &'static str,
    status: u16,
}

#[message(msg = "payment captured", tags = ["billing"])]
#[derive(Facet)]
struct PaymentCaptured {
    amount_cents: u64,
    currency: &'static str,
    region: &'static str,
}

struct RegionFilter {
    want: &'static str,
}

impl Sink for RegionFilter {
    fn on_message(&self, m: &dyn Message) {
        if let Some(peek) = m.as_facet()
            && let Ok(body) = peek.into_struct()
        {
            let Ok(region) = body.field_by_name("region") else {
                return;
            };

            if region.as_str() != Some(self.want) {
                return;
            }

            // Ad-hoc serialize the message through reflection
            print!("[{}] {} {{", self.want, m.msg());

            for (i, (field, value)) in body.fields().enumerate() {
                let sep = if i == 0 { " " } else { ", " };
                print!("{sep}{} = {value}", field.name);
            }

            println!(" }}");
        }
    }
}

fn main() {
    let mut sinks = Sinks::default();
    sinks.register(Box::new(RegionFilter { want: "eu" }));
    sinks.install().ok();

    // Same types and tags; only `region` differs, which a tag cannot filter on.
    event!(RequestCompleted {
        region: "eu",
        route: "/users/:id",
        status: 200
    });

    event!(RequestCompleted {
        region: "us",
        route: "/orders",
        status: 503
    });

    event!(PaymentCaptured {
        amount_cents: 4200,
        currency: "EUR",
        region: "eu"
    });

    event!(PaymentCaptured {
        amount_cents: 999,
        currency: "USD",
        region: "us"
    });
}
