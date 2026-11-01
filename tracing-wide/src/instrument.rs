//! Ambient autocapture: `event!` fills still-`None` `Option` fields from the span scope (feature `instrument`).
//!
//! - Install [`layer()`] on a `tracing_subscriber::registry()` stack; without it, every lookup misses.
//! - A field that is already `Some` keeps its value.
//! - A `?debug` span value arrives as a string, which only a `String` field takes; see [`FromCaptured`].

use tracing::{
    Subscriber,
    field::{Field, Visit},
    span::{self, Id, Record},
};
use tracing_subscriber::{
    layer::{Context, Layer},
    registry::LookupSpan,
};

/// The captured fields of one span, stored in the registry `Extensions`.
pub(crate) struct CapturedFields(pub(crate) Vec<(&'static str, CapturedValue)>);

/// A captured span field value; the variants mirror tracing's `Visit` callbacks.
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

/// The layer that captures span field values for the ambient join; build it with [`layer()`].
pub struct CaptureLayer {
    _private: (),
}

struct CaptureVisitor<'a>(&'a mut Vec<(&'static str, CapturedValue)>);

/// Converts a [`CapturedValue`] into a field type; a lossy conversion is a miss (`None`), not a wrap.
pub trait FromCaptured: Sized {
    fn from_captured(value: &CapturedValue) -> Option<Self>;
}

/// Builds a [`CaptureLayer`]; install it on a `tracing_subscriber::registry()` stack:
///
/// ```ignore
/// use tracing_subscriber::layer::SubscriberExt;
/// let subscriber = tracing_subscriber::registry().with(tracing_wide::instrument::layer());
/// ```
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

impl FromCaptured for f32 {
    fn from_captured(value: &CapturedValue) -> Option<Self> {
        match *value {
            CapturedValue::F64(f) => {
                let narrowed = f as f32;
                (f64::from(narrowed) == f || f.is_nan()).then_some(narrowed)
            }
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
