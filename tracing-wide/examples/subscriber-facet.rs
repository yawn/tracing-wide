//! Filtering on a message *field* with facet reflection.
//!
//! Run: `just example-subscriber-facet`
//!
//! Tags route by type — fixed at definition, identical for every instance. Some
//! routing is about the *data*: "only the `eu` region". That's per-event and
//! can't be a tag. With the `facet` feature, [`Message::as_facet`] hands back a
//! [`facet::Peek`] over the live body, so a subscriber pulls a field out *by
//! name* and filters on its runtime value — no downcast, no per-type match. A
//! type that doesn't derive `Facet` simply yields `None`.

use tracing_wide::{
    Message, event,
    facet::{Facet, HasFields},
    message,
    subscriber::{Subscriber, Subscribers},
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

impl Subscriber for RegionFilter {
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
    let mut subscribers = Subscribers::default();
    subscribers.register(Box::new(RegionFilter { want: "eu" }));
    subscribers.install().ok();

    // Same types, same tags — only `region` differs. The filter keeps the `eu`
    // events and drops the rest, which tags alone could never do.
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
