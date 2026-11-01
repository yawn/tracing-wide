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
/// ```
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use proptest::prelude::*;

    use super::{CapturedValue, FromCaptured};

    /// Asserts that each integer type takes the capture exactly when its value is in the type's range.
    macro_rules! assert_ints_take_what_fits {
        ($value:expr, $($t:ty),*) => {$({
            let (negative, magnitude) = sign_magnitude($value);
            let fits = if negative {
                magnitude <= (<$t>::MIN as i128).unsigned_abs()
            } else {
                magnitude <= <$t>::MAX as u128
            };
            let expected = fits.then(|| {
                if negative { (magnitude as i128).wrapping_neg() as $t } else { magnitude as $t }
            });

            prop_assert_eq!(<$t>::from_captured($value), expected);
        })*};
    }

    /// An integer capture of any variant, with values on and next to the bounds of every integer width.
    fn captured_int() -> impl Strategy<Value = CapturedValue> {
        let bounds = [
            i128::from(i8::MIN),
            i128::from(i8::MAX),
            i128::from(u8::MAX),
            i128::from(i16::MIN),
            i128::from(i16::MAX),
            i128::from(u16::MAX),
            i128::from(i32::MIN),
            i128::from(i32::MAX),
            i128::from(u32::MAX),
            i128::from(i64::MIN),
            i128::from(i64::MAX),
            i128::from(u64::MAX),
            0,
        ];
        let edges: Vec<i128> = bounds
            .into_iter()
            .flat_map(|b| [b - 1, b, b + 1])
            .chain([i128::MIN, i128::MAX])
            .collect();
        let edge = prop::sample::select(edges);

        prop_oneof![
            any::<i64>().prop_map(CapturedValue::I64),
            any::<i128>().prop_map(CapturedValue::I128),
            any::<u64>().prop_map(CapturedValue::U64),
            any::<u128>().prop_map(CapturedValue::U128),
            edge.clone().prop_map(|e| CapturedValue::I64(e as i64)),
            edge.clone().prop_map(CapturedValue::I128),
            edge.clone().prop_map(|e| CapturedValue::U64(e as u64)),
            edge.prop_map(|e| CapturedValue::U128(e as u128)),
        ]
    }

    /// The integer of a capture as sign and magnitude, which holds the value of every variant.
    fn sign_magnitude(value: &CapturedValue) -> (bool, u128) {
        match *value {
            CapturedValue::I64(n) => (n < 0, n.unsigned_abs().into()),
            CapturedValue::I128(n) => (n < 0, n.unsigned_abs()),
            CapturedValue::U64(n) => (false, n.into()),
            CapturedValue::U128(n) => (false, n),
            _ => unreachable!("not an integer capture"),
        }
    }

    proptest! {
        /// An `f32` that went through `f64` converts back to itself.
        #[test]
        fn an_f32_converts_back_to_itself(x in any::<f32>()) {
            let back = f32::from_captured(&CapturedValue::F64(f64::from(x)));

            if x.is_nan() {
                prop_assert!(back.is_some_and(f32::is_nan));
            } else {
                prop_assert_eq!(back.map(f32::to_bits), Some(x.to_bits()));
            }
        }

        /// An `f64` between two neighbouring `f32` values is a miss, not a rounding.
        #[test]
        fn an_f64_between_two_f32_is_a_miss(x in any::<f32>().prop_filter("finite", |x| x.is_finite())) {
            prop_assert_eq!(f32::from_captured(&CapturedValue::F64(f64::from(x).next_up())), None);
        }

        /// An integer capture converts exactly when its value fits the target type, and never wraps.
        #[test]
        fn an_integer_converts_exactly_when_it_fits(value in captured_int()) {
            assert_ints_take_what_fits!(&value, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
        }
    }
}
