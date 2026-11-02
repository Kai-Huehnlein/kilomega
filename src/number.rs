//! The exact numeric representation behind [`FmtNumber`](crate::FmtNumber).

/// The exact value that a [`FmtNumber`](crate::FmtNumber) wraps.
///
/// Integers are stored exactly, without a detour through `f64`. Floats are
/// stored as given, because an `f64` has already rounded by the time it reaches
/// the formatter.
///
/// The enum is `#[non_exhaustive]`, so implementations of [`IntoNumber`] may
/// construct the existing variants but matches need a wildcard arm.
///
/// ```
/// use kilomega::Number;
///
/// let small = Number::Uint(1500);
/// let float = Number::Float(1500.0);
/// assert_eq!(small, float);
/// ```
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum Number {
    /// An unsigned integer, stored exactly.
    Uint(u128),
    /// A signed integer, stored exactly.
    Int(i128),
    /// A floating point number, stored as given.
    Float(f64),
}

impl PartialEq for Number {
    /// Compares the exact numeric values, regardless of representation.
    ///
    /// `Number::Uint(1500) == Number::Float(1500.0)` holds while
    /// `Number::Uint(u128::MAX) == Number::Float(u128::MAX as f64)` does not,
    /// because the float has rounded to `2^128`.
    ///
    /// `NaN` is not equal to anything, including itself, and `-0.0` equals
    /// `0.0`, matching `f64`.
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Self::Uint(left), Self::Uint(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Uint(left), Self::Int(right)) | (Self::Int(right), Self::Uint(left)) => {
                right >= 0 && left == right.unsigned_abs()
            }
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Float(float), Self::Uint(integer))
            | (Self::Uint(integer), Self::Float(float)) => float_equals_uint(float, integer),
            (Self::Float(float), Self::Int(integer)) | (Self::Int(integer), Self::Float(float)) => {
                float_equals_uint(-float, integer.unsigned_abs())
            }
        }
    }
}

/// Returns whether `float` is exactly the integer `integer`.
fn float_equals_uint(float: f64, integer: u128) -> bool {
    if !float.is_finite() || float < 0.0 || float.fract() != 0.0 {
        return false;
    }
    // `u128::MAX as f64` rounds up to `2^128`, so anything at or above that is
    // out of range even though the cast would saturate.
    const TWO_POW_128: f64 = 340_282_366_920_938_463_463_374_607_431_768_211_456.0;
    if float >= TWO_POW_128 {
        return false;
    }
    (float as u128) == integer
}

/// Types that [`FmtNumber::new`](crate::FmtNumber::new) accepts.
///
/// Implemented for every primitive integer and float type. Integers are kept
/// exactly; `f32` is widened to `f64`, which is lossless.
///
/// The trait is open: implementing it for your own numeric type or newtype
/// wrapper is the supported way to make it format like any other number.
///
/// ```
/// use kilomega::{FmtNumber, IntoNumber, Number};
///
/// struct Bytes(u64);
///
/// impl IntoNumber for Bytes {
///     fn into_number(self) -> Number {
///         Number::Uint(self.0 as u128)
///     }
/// }
///
/// assert_eq!(format!("{}", FmtNumber::new(Bytes(1536 * 1024 * 1024))), "1.610612736G");
/// ```
pub trait IntoNumber {
    /// Converts `self` into the [`Number`] that will be formatted.
    ///
    /// Return the most exact variant available: `Number::Uint`/`Number::Int`
    /// for integers and `Number::Float` for floats. Non-finite and negative
    /// values need no special handling; the formatter prints `NaN`/`inf` and a
    /// leading `-` respectively.
    fn into_number(self) -> Number;
}

macro_rules! impl_into_number_uint {
    ($($ty:ty),* $(,)?) => {$(
        impl IntoNumber for $ty {
            fn into_number(self) -> Number {
                Number::Uint(self as u128)
            }
        }

        impl From<$ty> for Number {
            fn from(number: $ty) -> Self {
                Number::Uint(number as u128)
            }
        }
    )*};
}

macro_rules! impl_into_number_int {
    ($($ty:ty),* $(,)?) => {$(
        impl IntoNumber for $ty {
            fn into_number(self) -> Number {
                Number::Int(self as i128)
            }
        }

        impl From<$ty> for Number {
            fn from(number: $ty) -> Self {
                Number::Int(number as i128)
            }
        }
    )*};
}

impl_into_number_uint!(u8, u16, u32, u64, u128, usize);
impl_into_number_int!(i8, i16, i32, i64, i128, isize);

impl IntoNumber for f32 {
    fn into_number(self) -> Number {
        Number::Float(f64::from(self))
    }
}

impl IntoNumber for f64 {
    fn into_number(self) -> Number {
        Number::Float(self)
    }
}

impl From<f32> for Number {
    fn from(number: f32) -> Self {
        Number::Float(f64::from(number))
    }
}

impl From<f64> for Number {
    fn from(number: f64) -> Self {
        Number::Float(number)
    }
}
