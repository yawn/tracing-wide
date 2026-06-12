//! Ambient autocapture: fill a message's `Option` fields from the tracing span
//! scope at emit time. Gated by the `instrument` feature.
//!
//! tracing-core stores no field values, so capture is a cooperating
//! [`CaptureLayer`] that copies span field values into the registry's
//! `Extensions`; the read half ([`crate::__private::instrument::get`]) downcasts
//! the current dispatcher to the base `Registry` and walks the span scope
//! innermost-first. The contract, end to end:
//!
//! - **Only `Option` fields join.** Required (bare-typed) fields come from the
//!   `event!` struct literal, compile-checked; an ambient miss is legal and
//!   leaves the field `None` (which tracing then omits from the event).
//! - A field that is already `Some` is never overwritten — locals beat ambient.
//! - No registry, no [`CaptureLayer`], no current span → every lookup misses.
//!   `event!` never fails; degradation is graceful by construction.
//! - Type fidelity is bounded by tracing's visitor: primitives and strings
//!   round-trip; `?debug`-recorded values arrive as rendered strings (a
//!   `String` field absorbs them). Conversion is [`FromCaptured`]; an `Option`
//!   field whose inner type has no impl simply never fills.

use tracing::{
    Subscriber,
    field::{Field, Visit},
    span::{self, Id, Record},
};
use tracing_subscriber::{
    layer::{Context, Layer},
    registry::LookupSpan,
};

/// The per-span store the capture layer puts into the registry's `Extensions`.
/// A `Vec` (not a map): span field counts are small, and upsert keeps one entry
/// per name. `pub(crate)` so the read half — [`crate::__private::instrument::get`]
/// — can pull it back out of the registry.
pub(crate) struct CapturedFields(pub(crate) Vec<(&'static str, CapturedValue)>);

/// A span field value as the capture layer retained it. The variants mirror
/// tracing's `Visit` callbacks.
#[derive(Clone, Debug, PartialEq)]
pub enum CapturedValue {
    Bool(bool),
    /// Rendered via `fmt::Debug` (tracing's catch-all for non-primitive values).
    Debug(String),
    F64(f64),
    I64(i64),
    I128(i128),
    Str(String),
    U64(u64),
    U128(u128),
}

/// The capture half of ambient autocapture: retains every span's field values
/// in the registry's `Extensions` so the emit-time join can read them back.
/// Install it on a `tracing_subscriber::registry()`-based stack:
///
/// ```ignore
/// use tracing_subscriber::layer::SubscriberExt;
/// let subscriber = tracing_subscriber::registry().with(tracing_wide::instrument::layer());
/// ```
pub struct CaptureLayer {
    _private: (),
}

struct CaptureVisitor<'a>(&'a mut Vec<(&'static str, CapturedValue)>);

/// Conversion from a [`CapturedValue`] into a field's inner type. Implemented
/// for the primitives and `String`; integer conversions are lossless
/// (`try_from`) — an out-of-range capture is a miss, not a wrap.
pub trait FromCaptured: Sized {
    fn from_captured(value: &CapturedValue) -> Option<Self>;
}

/// Construct a [`CaptureLayer`] (the `fmt::layer()`-style spelling).
pub fn layer() -> CaptureLayer {
    CaptureLayer { _private: () }
}

impl CaptureVisitor<'_> {
    fn upsert(&mut self, field: &Field, value: CapturedValue) {
        let name = field.name();
        match self.0.iter_mut().find(|(n, _)| *n == name) {
            Some((_, v)) => *v = value,
            None => self.0.push((name, value)),
        }
    }
}

macro_rules! int_from_captured {
    ($($t:ty),* $(,)?) => {$(
        impl FromCaptured for $t {
            fn from_captured(value: &CapturedValue) -> Option<Self> {
                match *value {
                    CapturedValue::U64(n) => Self::try_from(n).ok(),
                    CapturedValue::I64(n) => Self::try_from(n).ok(),
                    CapturedValue::U128(n) => Self::try_from(n).ok(),
                    CapturedValue::I128(n) => Self::try_from(n).ok(),
                    _ => None,
                }
            }
        }
    )*};
}

int_from_captured!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl FromCaptured for bool {
    fn from_captured(value: &CapturedValue) -> Option<Self> {
        match *value {
            CapturedValue::Bool(b) => Some(b),
            _ => None,
        }
    }
}

impl FromCaptured for f64 {
    fn from_captured(value: &CapturedValue) -> Option<Self> {
        match *value {
            CapturedValue::F64(f) => Some(f),
            _ => None,
        }
    }
}

impl FromCaptured for String {
    fn from_captured(value: &CapturedValue) -> Option<Self> {
        match value {
            CapturedValue::Str(s) | CapturedValue::Debug(s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut fields = Vec::new();
        attrs.record(&mut CaptureVisitor(&mut fields));

        span.extensions_mut().insert(CapturedFields(fields));
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut ext = span.extensions_mut();

            match ext.get_mut::<CapturedFields>() {
                Some(captured) => values.record(&mut CaptureVisitor(&mut captured.0)),
                None => {
                    let mut fields = Vec::new();
                    values.record(&mut CaptureVisitor(&mut fields));
                    ext.insert(CapturedFields(fields));
                }
            }
        }
    }
}

impl Visit for CaptureVisitor<'_> {
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.upsert(field, CapturedValue::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.upsert(field, CapturedValue::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.upsert(field, CapturedValue::U64(value));
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.upsert(field, CapturedValue::I128(value));
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.upsert(field, CapturedValue::U128(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.upsert(field, CapturedValue::F64(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.upsert(field, CapturedValue::Str(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.upsert(field, CapturedValue::Debug(format!("{value:?}")));
    }
}
