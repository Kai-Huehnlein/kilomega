//! Exact significant-digit rounding for integer inputs.

use crate::prefix::MAX_BINARY_INDEX;

/// Rounds `value` to `significant_digits` significant decimal digits.
///
/// Returns `(digits, exponent)` so that
/// `value ~= digits * 10^(exponent - significant_digits + 1)` and `digits` has
/// exactly `significant_digits` digits (padded with zeros when the value has
/// fewer). Rounding is half-to-even, matching the standard library's float
/// formatter.
pub(crate) fn significant(value: u128, significant_digits: usize) -> (u128, i32) {
    debug_assert!(value > 0);
    debug_assert!((1..=17).contains(&significant_digits));

    let mut exponent = value.ilog10() as i32;
    let shift = exponent - (significant_digits as i32 - 1);

    let mut digits = if shift > 0 {
        // `shift <= 38` for a `u128`, and `10^38` is representable.
        let divisor = 10u128.pow(shift as u32);
        let quotient = value / divisor;
        let remainder = value % divisor;
        round_up(quotient, remainder * 2, divisor) // `divisor < 2^127`
    } else if shift < 0 {
        // Fewer digits than requested: scale up so the padding is explicit.
        value * 10u128.pow((-shift) as u32)
    } else {
        value
    };

    if digits >= 10u128.pow(significant_digits as u32) {
        digits /= 10;
        exponent += 1;
    }

    (digits, exponent)
}

/// Rounds `value / 1024^index` to `significant_digits` decimal digits.
///
/// Returns `(digits, exponent, index)` so that
/// `value ~= digits * 10^(exponent - significant_digits + 1) * 1024^index`.
/// Returns `None` when the value needs a prefix beyond `Qi`.
///
/// Binary prefixes are powers of two, so the mantissa is computed with exact
/// long division instead of a floating point division.
pub(crate) fn binary(value: u128, significant_digits: usize) -> Option<(u128, i32, usize)> {
    debug_assert!(value > 0);
    debug_assert!((1..=17).contains(&significant_digits));

    let mut index = (value.ilog2() / 10) as i32;
    if index > MAX_BINARY_INDEX {
        return None;
    }
    let shift = 10 * index as u32;
    let mask = if shift == 0 { 0 } else { (1u128 << shift) - 1 };
    let integer = if shift == 0 { value } else { value >> shift };
    let mut fraction = if shift == 0 { 0 } else { value & mask };

    let mut exponent = integer.ilog10() as i32;
    let integer_digits = integer.ilog10() as usize + 1;

    let mut digits = 0u128;
    let mut taken = 0usize;
    let mut extra = 0u32;
    let mut sticky = false;
    let mut stopped_in_integer = false;

    // Integer digits, most significant first.
    let mut power = 10u128.pow((integer_digits - 1) as u32);
    while power > 0 {
        let digit = (integer / power) % 10;
        if taken < significant_digits {
            digits = digits * 10 + digit;
            taken += 1;
            power /= 10;
        } else {
            extra = digit as u32;
            sticky = integer % power != 0 || fraction != 0;
            stopped_in_integer = true;
            break;
        }
    }

    if !stopped_in_integer {
        // `fraction < 2^shift <= 2^120`, so `fraction * 10` fits in a `u128`.
        while taken < significant_digits {
            fraction *= 10;
            let digit = if shift == 0 { 0 } else { fraction >> shift };
            if shift != 0 {
                fraction &= mask;
            }
            digits = digits * 10 + digit;
            taken += 1;
        }

        fraction *= 10;
        extra = if shift == 0 {
            0
        } else {
            (fraction >> shift) as u32
        };
        if shift != 0 {
            fraction &= mask;
        }
        sticky = fraction != 0;
    }

    if extra > 5 || (extra == 5 && (sticky || digits % 2 == 1)) {
        digits += 1;
        if digits >= 10u128.pow(significant_digits as u32) {
            digits /= 10;
            exponent += 1;
        }
    }

    // Rounding can lift the mantissa to exactly 1024, which is one unit of the
    // next binary prefix.
    if mantissa_head(digits, exponent as usize + 1) >= 1024 {
        index += 1;
        if index > MAX_BINARY_INDEX {
            return None;
        }
        return Some((
            10u128.pow((significant_digits - 1) as u32),
            0,
            index as usize,
        ));
    }

    Some((digits, exponent, index as usize))
}

/// The integer part of `digits` with the decimal point after `point` digits.
pub(crate) fn mantissa_head(digits: u128, point: usize) -> u128 {
    let length = digits.ilog10() as usize + 1;
    if length <= point {
        digits * 10u128.pow((point - length) as u32)
    } else {
        digits / 10u128.pow((length - point) as u32)
    }
}

/// Returns whether `value / divisor` should round up, given `twice_remainder`.
///
/// Implements round-half-to-even using only the comparison of `2 * remainder`
/// with the divisor, so it never overflows for a `u128` divisor.
fn round_up(value: u128, twice_remainder: u128, divisor: u128) -> u128 {
    if twice_remainder > divisor || (twice_remainder == divisor && value % 2 == 1) {
        value + 1
    } else {
        value
    }
}

/// The binary prefix index for a non-negative finite float.
///
/// Returns a negative value when the number is below `1` and therefore has no
/// binary prefix.
pub(crate) fn binary_index(value: f64) -> i32 {
    debug_assert!(value >= 0.0 && value.is_finite());
    if value == 0.0 {
        return -1;
    }
    let exponent = ((value.to_bits() >> 52) & 0x7ff) as i32 - 1023;
    if exponent < 0 { -1 } else { exponent / 10 }
}
