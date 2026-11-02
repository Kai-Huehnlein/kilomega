use crate::{FmtNumber, IntoNumber};

#[test]
fn test_unit_prefixes() {
    assert_eq!(format!("{}", FmtNumber::new(0.0)), "0");
    assert_eq!(format!("{}", FmtNumber::new(-0.0)), "-0");
    assert_eq!(format!("{}", FmtNumber::new(1.0)), "1");
    assert_eq!(format!("{}", FmtNumber::new(500.0)), "500");
    assert_eq!(format!("{}", FmtNumber::new(999.99)), "999.99");
    assert_eq!(format!("{}", FmtNumber::new(-1.0)), "-1");
    assert_eq!(format!("{}", FmtNumber::new(-500.0)), "-500");
    assert_eq!(format!("{}", FmtNumber::new(-999.99)), "-999.99");

    assert_eq!(format!("{}", FmtNumber::new(1000.0)), "1k");
    assert_eq!(format!("{}", FmtNumber::new(20_000.0)), "20k");
    assert_eq!(format!("{}", FmtNumber::new(999_999.0)), "999.999k");
    assert_eq!(format!("{}", FmtNumber::new(-1000.0)), "-1k");
    assert_eq!(format!("{}", FmtNumber::new(-20_000.0)), "-20k");
    assert_eq!(format!("{}", FmtNumber::new(-999_999.0)), "-999.999k");

    assert_eq!(format!("{}", FmtNumber::new(1e9)), "1G");
    assert_eq!(format!("{}", FmtNumber::new(20e9)), "20G");
    assert_eq!(format!("{}", FmtNumber::new(999.999e9)), "999.999G");
    assert_eq!(format!("{}", FmtNumber::new(-1e9)), "-1G");
    assert_eq!(format!("{}", FmtNumber::new(-20e9)), "-20G");
    assert_eq!(format!("{}", FmtNumber::new(-999.999e9)), "-999.999G");

    assert_eq!(format!("{}", FmtNumber::new(0.999)), "999m");
    assert_eq!(format!("{}", FmtNumber::new(0.1)), "100m");
    assert_eq!(format!("{}", FmtNumber::new(0.01)), "10m");
    assert_eq!(format!("{}", FmtNumber::new(0.001)), "1m");

    assert_eq!(format!("{}", FmtNumber::new(999e-3)), "999m");
    // Unicode U+00B5 is the micro sign. It is distinct from the greek letter mu.
    assert_eq!(format!("{}", FmtNumber::new(100e-6)), "100\u{b5}");
    assert_eq!(format!("{}", FmtNumber::new(10e-6)), "10\u{b5}");
    assert_eq!(format!("{}", FmtNumber::new(1e-6)), "1\u{b5}");
}

#[test]
fn test_every_prefix_is_reachable() {
    let macro_prefixes = ["", "k", "M", "G", "T", "P", "E", "Z", "Y", "R", "Q"];
    let micro_prefixes = ["", "m", "\u{b5}", "n", "p", "f", "a", "z", "y", "r", "q"];

    for (power, expected) in macro_prefixes.iter().enumerate() {
        let number = 10f64.powi(3 * power as i32);
        assert_eq!(
            format!("{}", FmtNumber::new(number)),
            format!("1{expected}")
        );
    }
    for (power, expected) in micro_prefixes.iter().enumerate() {
        let number = 10f64.powi(-3 * power as i32);
        assert_eq!(
            format!("{}", FmtNumber::new(number)),
            format!("1{expected}")
        );
    }
}

#[test]
fn test_range() {
    // The largest and smallest supported prefixed values.
    assert_eq!(format!("{}", FmtNumber::new(1e30)), "1Q");
    assert_eq!(format!("{}", FmtNumber::new(100e30)), "100Q");
    assert_eq!(format!("{}", FmtNumber::new(999.9e30)), "999.9Q");
    assert_eq!(format!("{}", FmtNumber::new(1e-30)), "1q");
    assert_eq!(format!("{}", FmtNumber::new(100e-30)), "100q");

    // Beyond `Q`/`q` there is no prefix, so scientific notation is used instead
    // of emitting absurdly long strings such as "1000...000Q".
    assert_eq!(format!("{}", FmtNumber::new(1e33)), "1e33");
    assert_eq!(format!("{}", FmtNumber::new(42e34)), "4.2e35");
    assert_eq!(format!("{}", FmtNumber::new(-1e33)), "-1e33");
    assert_eq!(format!("{}", FmtNumber::new(1e-33)), "1e-33");
    assert_eq!(format!("{}", FmtNumber::new(42e-34)), "4.2e-33");
    assert_eq!(format!("{}", FmtNumber::new(9.99e-31)), "9.99e-31");

    // Extreme magnitudes stay short no matter what.
    let huge = format!("{}", FmtNumber::new(f64::MAX));
    assert_eq!(huge, "1.79769313486232e308");
    let tiny = format!("{}", FmtNumber::new(f64::MIN_POSITIVE));
    assert_eq!(tiny, "2.2250738585072e-308");
}

#[test]
fn test_significant_digits() {
    assert_eq!(
        format!("{}", FmtNumber::new(444.44).with_significant_digits(1)),
        "400"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(-444.44).with_significant_digits(1)),
        "-400"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(444.44).with_significant_digits(2)),
        "440"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(-444.44).with_significant_digits(2)),
        "-440"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(999.99).with_significant_digits(2)),
        "1.0k"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(-999.99).with_significant_digits(2)),
        "-1.0k"
    );
}

#[test]
fn test_significant_digits_below_one() {
    assert_eq!(
        format!("{}", FmtNumber::new(0.444).with_significant_digits(1)),
        "400m"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(0.005).with_significant_digits(1)),
        "5m"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(4.2e-33).with_significant_digits(1)),
        "4e-33"
    );
    assert_eq!(
        format!("{}", FmtNumber::new(0.00042).with_significant_digits(2)),
        "420\u{b5}"
    );
}

#[test]
fn test_out_of_range_significant_digits() {
    // Precision is clamped instead of panicking or producing nonsense.
    assert_eq!(
        format!("{}", FmtNumber::new(1234.0).with_significant_digits(0)),
        "1k"
    );
    assert_eq!(
        format!(
            "{}",
            FmtNumber::new(1234.0).with_significant_digits(usize::MAX)
        ),
        "1.2340000000000000k"
    );
    assert_eq!(
        FmtNumber::new(1234.0)
            .with_significant_digits(0)
            .significant_digits(),
        Some(1)
    );
    assert_eq!(
        format!("{:.0}", FmtNumber::new(1234.0)),
        "1k",
        "`{{:.0}}` is clamped to one significant digit"
    );
    assert_eq!(
        format!("{:.999}", FmtNumber::new(1.5)),
        "1.5000000000000000",
        "`{{:.999}}` is clamped to 17 digits"
    );

    // A tiny value used to round to zero, which then reported the prefix as
    // "q" and printed "0q". Explicit digits also stay explicit in scientific
    // notation.
    assert_eq!(
        format!("{}", FmtNumber::new(1e-300).with_significant_digits(15)),
        "1.00000000000000e-300"
    );
    assert_eq!(format!("{}", FmtNumber::new(1e-300)), "1e-300");
}

#[test]
fn test_rounding_across_prefix_boundary() {
    // Rounding the mantissa can push it to 1000, which must carry into the next
    // prefix instead of printing e.g. "1000k". This must hold for the implicit
    // precision, an explicit `{:.N}`, and the builder alike.
    assert_eq!(format!("{}", FmtNumber::new(999_999.999_999_999_9)), "1M");
    assert_eq!(format!("{:.0}", FmtNumber::new(999.6)), "1k");
    assert_eq!(format!("{:.2}", FmtNumber::new(999.99)), "1.0k");
    assert_eq!(format!("{:.3}", FmtNumber::new(999_999.9)), "1.00M");
    assert_eq!(format!("{:.0}", FmtNumber::new(0.9999)), "1");
    assert_eq!(format!("{:.2}", FmtNumber::new(999.99e-3)), "1.0");
}

#[test]
fn test_significant_rounding_has_no_float_noise() {
    assert_eq!(format!("{}", FmtNumber::new(12_345.678)), "12.345678k");

    // Arithmetic noise must not leak into the output.
    assert_eq!(format!("{}", FmtNumber::new(0.1 + 0.2)), "300m");
    assert_eq!(format!("{}", FmtNumber::new(1.1 * 3.0)), "3.3");

    // Largest f64 below 1.0. Rounding to the displayed 15 significant digits
    // yields exactly 1.0, which must not print as "999.9999999999999m".
    assert_eq!(format!("{}", FmtNumber::new(1.0 - f64::EPSILON / 2.0)), "1");
}

#[test]
fn test_format() {
    // `{:.N}` means significant digits, not decimal places.
    assert_eq!(format!("{:.3}", FmtNumber::new(1.0)), "1.00");
    assert_eq!(format!("{:.2}", FmtNumber::new(3.456)), "3.5");
    assert_eq!(format!("{:.3}", FmtNumber::new(1000.0)), "1.00k");
    assert_eq!(format!("{:.2}", FmtNumber::new(3456)), "3.5k");
    assert_eq!(format!("{:.3}", FmtNumber::new(1e-4)), "100\u{b5}");
    assert_eq!(format!("{:.2}", FmtNumber::new(3.456e-3)), "3.5m");

    // The builder wins over the formatting-site precision.
    assert_eq!(
        format!(
            "{:.1}",
            FmtNumber::new(1234.5678).with_significant_digits(4)
        ),
        "1.235k"
    );
}

#[test]
fn test_width_and_alignment() {
    // The prefix belongs inside the padding.
    assert_eq!(format!("[{:>8}]", FmtNumber::new(1500.0)), "[    1.5k]");
    assert_eq!(format!("[{:<8}]", FmtNumber::new(1500.0)), "[1.5k    ]");
    assert_eq!(format!("[{:^8}]", FmtNumber::new(1500.0)), "[  1.5k  ]");
    assert_eq!(format!("[{:*^8}]", FmtNumber::new(1500.0)), "[**1.5k**]");
    assert_eq!(format!("[{:08}]", FmtNumber::new(1500.0)), "[00001.5k]");
    assert_eq!(format!("[{:08}]", FmtNumber::new(-1500.0)), "[-0001.5k]");

    // Sign flags.
    assert_eq!(format!("{:+}", FmtNumber::new(1500.0)), "+1.5k");
    assert_eq!(format!("{:+}", FmtNumber::new(-1500.0)), "-1.5k");
    assert_eq!(format!("{:+08}", FmtNumber::new(1500.0)), "+0001.5k");
}

#[test]
fn test_non_finite() {
    assert_eq!(format!("{}", FmtNumber::new(f64::NAN)), "NaN");
    assert_eq!(format!("{}", FmtNumber::new(f64::INFINITY)), "inf");
    assert_eq!(format!("{}", FmtNumber::new(f64::NEG_INFINITY)), "-inf");
    assert_eq!(
        format!("[{:>6}]", FmtNumber::new(f64::INFINITY)),
        "[   inf]"
    );
    assert_eq!(format!("{:+}", FmtNumber::new(f64::INFINITY)), "+inf");
}

#[test]
fn test_integer_inputs() {
    assert_eq!(format!("{}", FmtNumber::new(1_000_000u32)), "1M");
    assert_eq!(format!("{}", FmtNumber::new(1_500_000i64)), "1.5M");
    assert_eq!(format!("{}", FmtNumber::new(1_500_000usize)), "1.5M");
    assert_eq!(format!("{}", FmtNumber::new(-1_500_000i128)), "-1.5M");
    assert_eq!(format!("{}", FmtNumber::new(u64::MAX)), "18.4467440737096E");
    assert_eq!(format!("{}", FmtNumber::new(1.5f32)), "1.5");
}

#[test]
fn test_conversions_and_traits() {
    let number = FmtNumber::from(1500.0);
    assert_eq!(number.number(), 1500.0);
    assert_eq!(number.significant_digits(), None);
    assert_eq!(format!("{number}"), "1.5k");

    // Copy
    let copy = number;
    assert_eq!(format!("{copy}"), format!("{number}"));

    // Debug matches Display
    assert_eq!(format!("{:?}", FmtNumber::new(1500.0)), "1.5k");

    // Default
    assert_eq!(format!("{}", FmtNumber::default()), "0");

    // PartialEq compares the wrapped number, not the rendering.
    assert_eq!(FmtNumber::new(1000.0), FmtNumber::new(1000.0));
    assert_ne!(FmtNumber::new(1000.0), FmtNumber::new(1000.1));
}

/// Parses a prefixed rendering back into `(mantissa, prefix index)`.
///
/// Returns `None` for scientific notation.
fn parse_prefixed(output: &str) -> Option<(f64, i32)> {
    const MACRO: [char; 10] = ['k', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y', 'R', 'Q'];
    const MICRO: [char; 10] = ['m', '\u{b5}', 'n', 'p', 'f', 'a', 'z', 'y', 'r', 'q'];

    // Scientific notation is the only case without a prefix that still parses.
    if output.contains('e') {
        return None;
    }

    let last = output.chars().next_back()?;
    let (number, index) = if let Some(position) = MACRO.iter().position(|c| *c == last) {
        (
            &output[..output.len() - last.len_utf8()],
            position as i32 + 1,
        )
    } else if let Some(position) = MICRO.iter().position(|c| *c == last) {
        (
            &output[..output.len() - last.len_utf8()],
            -(position as i32 + 1),
        )
    } else {
        (output, 0)
    };

    Some((number.parse().ok()?, index))
}

#[test]
fn test_mantissa_invariant_over_full_range() {
    let mut prefixed = 0usize;
    let mut scientific = 0usize;

    for exponent in -320..=300 {
        for mantissa in [1.0, 1.5, 9.99, 123.456, 999.9] {
            for sign in [1.0, -1.0] {
                let value = sign * mantissa * 10f64.powi(exponent);
                if value == 0.0 || !value.is_finite() {
                    continue;
                }

                let output = FmtNumber::new(value).to_string();
                assert!(
                    !output.contains("NaN") && !output.contains("inf"),
                    "{output}"
                );
                assert!(output.len() < 30, "unreasonably long output {output}");

                if let Some((mantissa, index)) = parse_prefixed(&output) {
                    prefixed += 1;
                    assert!((-10..=10).contains(&index), "{output} has no prefix");
                    assert!(
                        (1.0..1000.0).contains(&mantissa.abs()),
                        "{output} has a mantissa outside [1, 1000)"
                    );
                    let reconstructed = mantissa * 10f64.powi(3 * index);
                    assert!(
                        (reconstructed - value).abs() <= value.abs() * 1e-12,
                        "{output} ({reconstructed}) does not round-trip to {value}"
                    );
                } else {
                    scientific += 1;
                    let parsed: f64 = output.parse().expect("scientific notation");
                    assert!(
                        (parsed - value).abs() <= value.abs() * 1e-12,
                        "{output} does not round-trip to {value}"
                    );
                }
            }
        }
    }

    // The sweep must exercise both paths: every magnitude from `q` to `Q` uses a
    // prefix, everything outside that falls back to scientific notation.
    assert!(prefixed > 500, "only {prefixed} prefixed outputs");
    assert!(scientific > 3000, "only {scientific} scientific outputs");
}

/// Implementations of the IntoNumber trait for user-defined numeric types
/// must keep compiling.
struct Bytes(u64);

impl IntoNumber for Bytes {
    fn into_number(self) -> f64 {
        self.0 as f64
    }
}

#[test]
fn test_into_number_is_implementable_downstream() {
    assert_eq!(
        format!("{}", FmtNumber::new(Bytes(1536 * 1024 * 1024))),
        "1.610612736G"
    );
    assert_eq!(
        format!("{:.3}", FmtNumber::new(Bytes(1536))),
        "1.54k",
        "custom types get the same precision handling"
    );
}
