//! Dead simple and robust formatting of numbers with SI unit prefixes.
//!
//! [`FmtNumber`] wraps an `f64` and prints it with the SI prefix that keeps the
//! mantissa in `[1, 1000)`, so `1_500_000.0` renders as `1.5M` and
//! `0.000_002_5` renders as `2.5µ`.
//!
//! ```
//! use kilomega::FmtNumber;
//!
//! // `Display` picks the prefix that keeps the number easy to read.
//! assert_eq!(FmtNumber::new(1500).to_string(), "1.5k");
//! assert_eq!(FmtNumber::new(2.5e-6).to_string(), "2.5µ");
//! assert_eq!(FmtNumber::new(-0.25).to_string(), "-250m");
//!
//! // A precision asks for that many *significant digits*.
//! assert_eq!(format!("{:.3}", FmtNumber::new(1234.0)), "1.23k");
//! assert_eq!(FmtNumber::new(1234.0).with_significant_digits(2).to_string(), "1.2k");
//!
//! // Anything without a prefix falls back to plain scientific notation.
//! assert_eq!(FmtNumber::new(1e300).to_string(), "1e300");
//! ```
//!
//! # Behaviour
//!
//! * `Display` keeps at most 15 significant digits and drops trailing zeros.
//!   That is enough to hide the last-bit float noise introduced by arithmetic
//!   (`0.1 + 0.2` prints as `300m`, not `300.00000000000004m`) while still
//!   round-tripping every value a human would care about.
//! * `{:.N}` asks for `N` *significant digits*, not decimal places, so
//!   `{:.3}` means "three meaningful digits" at every magnitude
//!   (`1.23k`, `123m`, `1.23G`). Explicit digits are kept verbatim, including
//!   trailing zeros (`{:.3}` of `1000.0` is `1.00k`). `N` is clamped to
//!   `1..=17`.
//! * [`FmtNumber::with_significant_digits`] is the same knob from the builder
//!   side and takes precedence over `{:.N}`.
//! * Width, alignment, fill, `+`, and zero padding are honoured, and the prefix
//!   is part of the padded output: `{:>8}` of `1500` is `"    1.5k"`.
//! * Values that fit a supported prefix (`10^-30 <= |x| < 10^33`, i.e. the
//!   range covered by `q` through `Q`) are prefixed. Anything outside it is
//!   printed in scientific notation instead of inventing prefixes or emitting
//!   hundreds of digits.
//! * `NaN` and the infinities are printed as `NaN`, `inf`, and `-inf`.
//! * The micro prefix is [MICRO SIGN](https://www.unicode.org/charts/PDF/U0080.pdf)
//!   (`U+00B5`), not the Greek letter mu (`U+03BC`).
//!
//! # Integers
//!
//! [`FmtNumber::new`] accepts any integer or float through [`IntoNumber`],
//! including `u64`/`usize` values such as byte counts. Integers that an `f64`
//! cannot represent exactly are rounded to the nearest representable value
//! before formatting. [`IntoNumber`] is open, so newtype wrappers like
//! `Bytes(u64)` can opt in with a few lines.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use std::fmt;

#[cfg(test)]
mod tests;

/// SI prefixes for magnitudes of `10^0` and above, in steps of `10^3`.
const MACRO_SUFFIXES: &[&str] = &[
    "",  // 10^0
    "k", // kilo
    "M", // mega
    "G", // giga
    "T", // tera
    "P", // peta
    "E", // exa
    "Z", // zetta
    "Y", // yotta
    "R", // ronna
    "Q", // quetta
];

/// SI prefixes for magnitudes below `10^0`, in steps of `10^3`.
///
/// The micro prefix is the micro sign `U+00B5`, not the Greek letter mu.
const MICRO_SUFFIXES: &[&str] = &[
    "",  // 10^0 (for consistency)
    "m", // milli
    "µ", // micro
    "n", // nano
    "p", // pico
    "f", // femto
    "a", // atto
    "z", // zepto
    "y", // yocto
    "r", // ronto
    "q", // quecto
];

/// Largest prefix index, i.e. `Q` (`10^30`).
const MAX_INDEX: i32 = MACRO_SUFFIXES.len() as i32 - 1;
/// Smallest prefix index, i.e. `q` (`10^-30`).
const MIN_INDEX: i32 = -(MICRO_SUFFIXES.len() as i32 - 1);

/// Significant digits shown when the caller does not ask for a specific count.
///
/// This is enough to hide last-bit noise while keeping the output readable.
const DEFAULT_SIGNIFICANT_DIGITS: usize = 15;

/// An `f64` carries at most 17 significant decimal digits. Anything more would
/// only print noise, so requested precisions are clamped to this.
const MAX_SIGNIFICANT_DIGITS: usize = 17;

/// An `f64` that is displayed with an SI unit prefix.
///
/// ```
/// use kilomega::FmtNumber;
///
/// assert_eq!(format!("{}", FmtNumber::new(4_700_000_000.0)), "4.7G");
/// ```
///
/// `Debug` matches `Display`, like `std::path::Path`.
#[must_use = "a FmtNumber does nothing unless it is formatted"]
#[derive(Clone, Copy, PartialEq)]
pub struct FmtNumber {
    number: f64,
    significant_digits: Option<usize>,
}

impl FmtNumber {
    /// Wraps `number` for prefixed formatting.
    ///
    /// ```
    /// use kilomega::FmtNumber;
    ///
    /// assert_eq!(format!("{}", FmtNumber::new(1536u64 * 1024 * 1024)), "1.610612736G");
    /// ```
    pub fn new(number: impl IntoNumber) -> Self {
        Self {
            number: number.into_number(),
            significant_digits: None,
        }
    }

    /// Sets how many significant digits to display.
    ///
    /// The value is clamped to `1..=17`. This takes precedence over a
    /// `{:.N}` precision given at the formatting site.
    ///
    /// ```
    /// use kilomega::FmtNumber;
    ///
    /// let number = FmtNumber::new(1234.5678).with_significant_digits(3);
    /// assert_eq!(format!("{number}"), "1.23k");
    /// ```
    pub fn with_significant_digits(mut self, significant_digits: usize) -> Self {
        self.significant_digits = Some(significant_digits.clamp(1, MAX_SIGNIFICANT_DIGITS));
        self
    }

    /// Returns the wrapped number.
    #[must_use]
    pub fn number(self) -> f64 {
        self.number
    }

    /// Returns the configured significant digits, if any.
    #[must_use]
    pub fn significant_digits(self) -> Option<usize> {
        self.significant_digits
    }
}

impl Default for FmtNumber {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl From<f64> for FmtNumber {
    fn from(number: f64) -> Self {
        Self::new(number)
    }
}

impl From<f32> for FmtNumber {
    fn from(number: f32) -> Self {
        Self::new(number)
    }
}

/// Types that [`FmtNumber::new`] accepts.
///
/// Implemented for every primitive integer and float type. Integers wider than
/// the 53-bit `f64` significand are rounded to the nearest representable value.
///
/// The trait is open: implementing it for your own numeric type or newtype
/// wrapper is the supported way to make it format like any other number.
///
/// ```
/// use kilomega::{FmtNumber, IntoNumber};
///
/// struct Bytes(u64);
///
/// impl IntoNumber for Bytes {
///     fn into_number(self) -> f64 {
///         self.0 as f64
///     }
/// }
///
/// assert_eq!(format!("{}", FmtNumber::new(Bytes(1536 * 1024 * 1024))), "1.610612736G");
/// ```
pub trait IntoNumber {
    /// Converts `self` into the `f64` that will be formatted.
    ///
    /// Non-finite and negative values are handled: they print as `NaN`/`inf`
    /// and with a leading `-` respectively. Nothing else is required of an
    /// implementation.
    fn into_number(self) -> f64;
}

/// Integers (and `f32`) below the 53-bit `f64` significand convert exactly.
macro_rules! impl_into_number_exact {
    ($($ty:ty),* $(,)?) => {$(
        impl IntoNumber for $ty {
            fn into_number(self) -> f64 {
                f64::from(self)
            }
        }
    )*};
}

/// Types wider than the `f64` significand convert to the closest representable
/// value.
macro_rules! impl_into_number_lossy {
    ($($ty:ty),* $(,)?) => {$(
        impl IntoNumber for $ty {
            fn into_number(self) -> f64 {
                #[allow(clippy::cast_precision_loss)]
                let number = self as f64;
                number
            }
        }
    )*};
}

impl_into_number_exact!(f32, i8, i16, i32, u8, u16, u32);
impl_into_number_lossy!(f64, i64, i128, isize, u64, u128, usize);

impl fmt::Display for FmtNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let number = self.number;

        if number.is_nan() {
            return f.pad_integral(true, "", "NaN");
        }
        if number.is_infinite() {
            return f.pad_integral(number > 0.0, "", "inf");
        }

        // The builder takes precedence over `{:.N}`. Both mean significant
        // digits, so they map onto the same code path.
        let significant_digits = self.significant_digits.or_else(|| f.precision());
        let (negative, digits, exponent) = decimal_parts(number, significant_digits);
        let index = exponent.div_euclid(3);

        if !(MIN_INDEX..=MAX_INDEX).contains(&index) {
            // No SI prefix this large exists, so fall back to scientific
            // notation rather than printing something like "1000Q".
            let mut body = String::with_capacity(digits.len() + 8);
            body.push_str(&digits[..1]);
            if digits.len() > 1 {
                body.push('.');
                body.push_str(&digits[1..]);
            }
            body.push('e');
            body.push_str(&exponent.to_string());
            return f.pad_integral(!negative, "", &body);
        }

        // `digits` holds the significant digits of `mantissa * 10^exponent`,
        // where the mantissa is in `[1, 10)`. Position the decimal point so the
        // prefixed value lands in `[1, 1000)`.
        let point = usize::try_from(exponent.rem_euclid(3)).unwrap_or(0) + 1;

        let mut body = String::with_capacity(digits.len() + 4);
        if digits.len() <= point {
            body.push_str(&digits);
            body.extend(std::iter::repeat_n('0', point - digits.len()));
        } else {
            body.push_str(&digits[..point]);
            body.push('.');
            body.push_str(&digits[point..]);
        }
        body.push_str(suffix(index));

        // `pad_integral` handles the sign, width, fill, alignment, and zero
        // padding for the number and its prefix together.
        f.pad_integral(!negative, "", &body)
    }
}

impl fmt::Debug for FmtNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Returns the prefix for `index`, or an empty string if it is out of range.
fn suffix(index: i32) -> &'static str {
    let magnitude = usize::try_from(index.unsigned_abs()).unwrap_or(usize::MAX);
    let table = if index >= 0 {
        MACRO_SUFFIXES
    } else {
        MICRO_SUFFIXES
    };
    table.get(magnitude).copied().unwrap_or_default()
}

/// Splits `number` into its sign, its significant digits, and the exponent of
/// the trailing decimal point.
///
/// The digits are produced by the standard library's scientific formatter, so
/// they are correctly rounded and the exponent is already normalized for
/// rounding across a magnitude boundary (`999.6` at one significant digit is
/// `1e3`, not `1000e0`).
fn decimal_parts(number: f64, significant_digits: Option<usize>) -> (bool, String, i32) {
    let negative = number.is_sign_negative();
    let precision = match significant_digits {
        Some(digits) => digits.clamp(1, MAX_SIGNIFICANT_DIGITS) - 1,
        None => DEFAULT_SIGNIFICANT_DIGITS - 1,
    };

    let formatted = format!("{:.*e}", precision, number.abs());
    // The `e` format specifier always emits an exponent, so the fallback is
    // unreachable; it exists only to keep formatting infallible.
    let (mantissa, exponent) = if let Some((mantissa, exponent)) = formatted.split_once('e') {
        (mantissa, exponent.parse::<i32>().unwrap_or(0))
    } else {
        (formatted.as_str(), 0)
    };

    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();

    if digits.bytes().all(|byte| byte == b'0') {
        // Zero, or something that rounded to zero. Keep one digit per
        // requested significant digit so `{:.3}` of zero still prints "0.00".
        digits = "0".repeat(precision + 1);
        if significant_digits.is_none() {
            digits.truncate(1);
        }
    } else if significant_digits.is_none() {
        // Implicit precision: drop the zeros that only pad out the internal
        // digit budget, which is what turns "300.00000000000m" into "300m".
        while digits.len() > 1 && digits.ends_with('0') {
            digits.pop();
        }
    }

    (negative, digits, exponent)
}

/// Compiles and runs the examples in `README.md` as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
