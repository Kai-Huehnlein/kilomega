//! Dead simple and robust formatting of numbers with SI and binary unit prefixes.
//!
//! [`FmtNumber`] wraps a number and prints it with the SI prefix that keeps the
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
//! [`FmtNumber::new`] accepts any integer or float through [`IntoNumber`].
//! Every primitive integer is stored exactly, so byte counts and other large
//! counts are never rounded through an `f64`:
//!
//! ```
//! use kilomega::FmtNumber;
//!
//! assert_eq!(format!("{}", FmtNumber::new(1536u64 * 1024 * 1024)), "1.610612736G");
//! // 17 significant digits of a value an `f64` cannot represent exactly.
//! assert_eq!(format!("{:.17}", FmtNumber::new(10_000_000_000_000_001u128)), "10.000000000000001P");
//! ```
//!
//! [`IntoNumber`] is open, so newtype wrappers like `Bytes(u64)` can opt in
//! with a few lines.
//!
//! # Binary prefixes
//!
//! [`PrefixSystem::Binary`] switches to the 1024-based IEC prefixes (`Ki`,
//! `Mi`, ... `Qi`), keeping the mantissa in `[1, 1024)`:
//!
//! ```
//! use kilomega::{FmtNumber, PrefixSystem};
//!
//! let binary = |number| FmtNumber::new(number).with_prefix_system(PrefixSystem::Binary);
//! assert_eq!(binary(1536u64).to_string(), "1.5Ki");
//! assert_eq!(binary(1024u64 * 1024).to_string(), "1Mi");
//! assert_eq!(binary(1023u64).to_string(), "1023");
//! ```
//!
//! Binary prefixes only exist for magnitudes of `1` and above. Values below
//! `1` are printed as plain decimals (`0.5`, not `0.5Ki`), and values at or
//! above `Qi` (`2^100`) fall back to scientific notation.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use std::fmt;

mod number;
mod prefix;
mod round;

pub use number::{IntoNumber, Number};
pub use prefix::PrefixSystem;

#[cfg(test)]
mod tests;

/// Significant digits shown when the caller does not ask for a specific count.
///
/// This is enough to hide last-bit noise while keeping the output readable.
const DEFAULT_SIGNIFICANT_DIGITS: usize = 15;

/// A binary64 float carries at most 17 significant decimal digits, and more
/// would only print the noise in its representation, so requested precisions
/// are clamped to this.
const MAX_SIGNIFICANT_DIGITS: usize = 17;

/// A number that is displayed with a unit prefix.
///
/// ```
/// use kilomega::FmtNumber;
///
/// assert_eq!(format!("{}", FmtNumber::new(4_700_000_000.0)), "4.7G");
/// ```
///
/// `Debug` matches `Display`, like `std::path::Path`.
///
/// Equality compares the exact wrapped value and ignores the formatting
/// settings, so wrapping `1500` as an integer or as a float compares equal.
#[must_use = "a FmtNumber does nothing unless it is formatted"]
#[derive(Clone, Copy)]
pub struct FmtNumber {
    number: Number,
    significant_digits: Option<usize>,
    prefix_system: PrefixSystem,
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
            prefix_system: PrefixSystem::default(),
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

    /// Selects the prefix system, either SI (`1000`) or IEC binary (`1024`).
    ///
    /// ```
    /// use kilomega::{FmtNumber, PrefixSystem};
    ///
    /// let number = FmtNumber::new(1536u64).with_prefix_system(PrefixSystem::Binary);
    /// assert_eq!(format!("{number}"), "1.5Ki");
    /// ```
    pub fn with_prefix_system(mut self, prefix_system: PrefixSystem) -> Self {
        self.prefix_system = prefix_system;
        self
    }

    /// Returns the wrapped value.
    ///
    /// Integers come back as [`Number::Uint`] or [`Number::Int`], so no
    /// precision is lost.
    #[must_use]
    pub fn number(self) -> Number {
        self.number
    }

    /// Returns the configured significant digits, if any.
    #[must_use]
    pub fn significant_digits(self) -> Option<usize> {
        self.significant_digits
    }

    /// Returns the configured prefix system.
    #[must_use]
    pub fn prefix_system(self) -> PrefixSystem {
        self.prefix_system
    }

    /// Formats an unsigned integer magnitude.
    fn integer_body(&self, magnitude: u128, significant_digits: usize, keep: bool) -> String {
        if magnitude == 0 {
            return if keep {
                format!("0.{}", "0".repeat(significant_digits - 1))
            } else {
                "0".to_owned()
            };
        }

        match self.prefix_system {
            PrefixSystem::Si => {
                let (digits, exponent) = round::significant(magnitude, significant_digits);
                let digits = render_digits(digits, keep);
                let index = exponent.div_euclid(3);
                if !(prefix::MIN_SI_INDEX..=prefix::MAX_SI_INDEX).contains(&index) {
                    return layout_scientific(&digits, exponent);
                }
                let point = exponent.rem_euclid(3) as usize + 1;
                layout_decimal(&digits, point, PrefixSystem::si_suffix(index))
            }
            PrefixSystem::Binary => {
                if let Some((digits, exponent, index)) =
                    round::binary(magnitude, significant_digits)
                {
                    let digits = render_digits(digits, keep);
                    layout_decimal(
                        &digits,
                        exponent as usize + 1,
                        PrefixSystem::binary_suffix(index as i32),
                    )
                } else {
                    // Beyond `Qi`: one prefix would not fit, so use scientific
                    // notation rather than printing something like "1024Qi".
                    let (digits, exponent) = round::significant(magnitude, significant_digits);
                    let digits = render_digits(digits, keep);
                    layout_scientific(&digits, exponent)
                }
            }
        }
    }

    /// Formats a float, returning its sign separately from its body.
    fn float_body(&self, number: f64, significant_digits: usize, keep: bool) -> (bool, String) {
        let negative = number.is_sign_negative();
        let magnitude = number.abs();

        match self.prefix_system {
            PrefixSystem::Si => {
                let (digits, exponent) = decimal_parts(magnitude, significant_digits, keep);
                let index = exponent.div_euclid(3);
                if !(prefix::MIN_SI_INDEX..=prefix::MAX_SI_INDEX).contains(&index) {
                    return (negative, layout_scientific(&digits, exponent));
                }
                let point = exponent.rem_euclid(3) as usize + 1;
                (
                    negative,
                    layout_decimal(&digits, point, PrefixSystem::si_suffix(index)),
                )
            }
            PrefixSystem::Binary => {
                let index = round::binary_index(magnitude);
                if index < 0 {
                    // Below 1 there is no binary prefix to pick.
                    let (digits, exponent) = decimal_parts(magnitude, significant_digits, keep);
                    return (negative, layout_plain(&digits, exponent));
                }
                if index > prefix::MAX_BINARY_INDEX {
                    let (digits, exponent) = decimal_parts(magnitude, significant_digits, keep);
                    return (negative, layout_scientific(&digits, exponent));
                }

                // Dividing by a power of two is exact.
                let mantissa = magnitude / 1024f64.powi(index);
                let (digits, exponent) = decimal_parts(mantissa, significant_digits, keep);
                let point = exponent as usize + 1;

                if mantissa_head(&digits, point) >= 1024 {
                    // The mantissa rounded up to exactly 1024, which is one
                    // unit of the next prefix.
                    let next = index + 1;
                    if next > prefix::MAX_BINARY_INDEX {
                        let (digits, exponent) = decimal_parts(magnitude, significant_digits, keep);
                        return (negative, layout_scientific(&digits, exponent));
                    }
                    let digits = if keep {
                        format!("1{}", "0".repeat(significant_digits - 1))
                    } else {
                        "1".to_owned()
                    };
                    return (
                        negative,
                        layout_decimal(&digits, 1, PrefixSystem::binary_suffix(next)),
                    );
                }

                (
                    negative,
                    layout_decimal(&digits, point, PrefixSystem::binary_suffix(index)),
                )
            }
        }
    }
}

impl PartialEq for FmtNumber {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}

impl Default for FmtNumber {
    fn default() -> Self {
        Self::new(0.0)
    }
}

macro_rules! impl_from_number {
    ($($ty:ty),* $(,)?) => {$(
        impl From<$ty> for FmtNumber {
            fn from(number: $ty) -> Self {
                Self::new(number)
            }
        }
    )*};
}

impl_from_number!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
);

impl fmt::Display for FmtNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The builder takes precedence over `{:.N}`. Both mean significant
        // digits, so they map onto the same code path.
        let requested = self.significant_digits.or_else(|| f.precision());
        let keep_trailing_zeros = requested.is_some();
        let significant_digits = requested.map_or(DEFAULT_SIGNIFICANT_DIGITS, |digits| {
            digits.clamp(1, MAX_SIGNIFICANT_DIGITS)
        });

        let (negative, body) = match self.number {
            Number::Uint(magnitude) => (
                false,
                self.integer_body(magnitude, significant_digits, keep_trailing_zeros),
            ),
            Number::Int(number) => (
                number < 0,
                self.integer_body(
                    number.unsigned_abs(),
                    significant_digits,
                    keep_trailing_zeros,
                ),
            ),
            Number::Float(number) => {
                if number.is_nan() {
                    return f.pad_integral(true, "", "NaN");
                }
                if number.is_infinite() {
                    return f.pad_integral(number > 0.0, "", "inf");
                }
                self.float_body(number, significant_digits, keep_trailing_zeros)
            }
        };

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

/// Turns rounded integer digits into a string, dropping the zeros that only
/// pad out the internal digit budget unless the caller asked for them.
fn render_digits(digits: u128, keep: bool) -> String {
    if keep {
        return digits.to_string();
    }
    let mut digits = digits;
    while digits > 9 && digits % 10 == 0 {
        digits /= 10;
    }
    digits.to_string()
}

/// Splits `number` into its significant digits and the exponent of the trailing
/// decimal point.
///
/// The digits are produced by the standard library's scientific formatter, so
/// they are correctly rounded and the exponent is already normalized for
/// rounding across a magnitude boundary (`999.6` at one significant digit is
/// `1e3`, not `1000e0`).
fn decimal_parts(number: f64, significant_digits: usize, keep: bool) -> (String, i32) {
    let precision = significant_digits - 1;
    let formatted = format!("{:.*e}", precision, number);
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
        digits = "0".repeat(significant_digits);
        if !keep {
            digits.truncate(1);
        }
    } else if !keep {
        // Implicit precision: drop the zeros that only pad out the internal
        // digit budget, which is what turns "300.00000000000m" into "300m".
        while digits.len() > 1 && digits.ends_with('0') {
            digits.pop();
        }
    }

    (digits, exponent)
}

/// The integer part of `digits` with the decimal point after `point` digits.
fn mantissa_head(digits: &str, point: usize) -> u128 {
    let Some(mut head) = digits.parse::<u128>().ok() else {
        return 0;
    };
    if digits.len() <= point {
        head *= 10u128.pow((point - digits.len()) as u32);
        head
    } else {
        head / 10u128.pow((digits.len() - point) as u32)
    }
}

/// Positions the decimal point inside `digits` and appends `suffix`.
fn layout_decimal(digits: &str, point: usize, suffix: &str) -> String {
    let mut body = String::with_capacity(digits.len() + suffix.len() + 2);
    if digits.len() <= point {
        body.push_str(digits);
        body.extend(std::iter::repeat_n('0', point - digits.len()));
    } else {
        body.push_str(&digits[..point]);
        body.push('.');
        body.push_str(&digits[point..]);
    }
    body.push_str(suffix);
    body
}

/// Formats a plain decimal, used for binary-mode values below `1`.
fn layout_plain(digits: &str, exponent: i32) -> String {
    if exponent >= 0 {
        return layout_decimal(digits, exponent as usize + 1, "");
    }
    let mut body = String::with_capacity(digits.len() + (-exponent) as usize + 2);
    body.push_str("0.");
    body.extend(std::iter::repeat_n('0', (-exponent - 1) as usize));
    body.push_str(digits);
    body
}

/// Formats `digits * 10^exponent` as `d.dddde<exponent>`.
fn layout_scientific(digits: &str, exponent: i32) -> String {
    let mut body = String::with_capacity(digits.len() + 8);
    body.push_str(&digits[..1]);
    if digits.len() > 1 {
        body.push('.');
        body.push_str(&digits[1..]);
    }
    body.push('e');
    body.push_str(&exponent.to_string());
    body
}

/// Compiles and runs the examples in `README.md` as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
