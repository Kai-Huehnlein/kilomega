# `kilomega`

Dead simple and robust formatting of numbers with SI unit prefixes.

```rust
use kilomega::FmtNumber;

assert_eq!(format!("{}", FmtNumber::new(1500.0)), "1.5k");
assert_eq!(format!("{}", FmtNumber::new(1_500_000.0)), "1.5M");
assert_eq!(format!("{}", FmtNumber::new(2.5e-6)), "2.5µ");
assert_eq!(format!("{}", FmtNumber::new(-0.25)), "-250m");
```

`FmtNumber` is a `Copy` wrapper around an `f64` that implements `Display`. It
picks the prefix that keeps the mantissa in `[1, 1000)`, so numbers stay short
and comparable at any magnitude.

## Precision

Significant digits can be specified with both `{:.N}` and the builder API. The
builder takes precedence over `{:.N}`.

```rust
use kilomega::FmtNumber;

assert_eq!(format!("{:.3}", FmtNumber::new(1234.0)), "1.23k");
assert_eq!(format!("{:.3}", FmtNumber::new(0.001234)), "1.23m");
assert_eq!(format!("{:.3}", FmtNumber::new(1.234e9)), "1.23G");

let number = FmtNumber::new(1234.5678).with_significant_digits(2);
assert_eq!(number.to_string(), "1.2k");
```

## Integers, bytes, and durations

`FmtNumber::new` accepts every primitive integer and float, including `u64`,
`usize`, and `i128`, so byte counts and durations work without casts:

```rust
use kilomega::FmtNumber;

assert_eq!(format!("{}", FmtNumber::new(1536u64 * 1024 * 1024)), "1.610612736G");
assert_eq!(format!("{}", FmtNumber::new(u64::MAX)), "18.4467440737096E");
```

Integers wider than the 53-bit `f64` significand are rounded to the nearest
representable value before formatting.

Your own numeric types and newtype wrappers opt in by implementing `IntoNumber`:

```rust
use kilomega::{FmtNumber, IntoNumber};

struct Bytes(u64);

impl IntoNumber for Bytes {
    fn into_number(self) -> f64 {
        self.0 as f64
    }
}

assert_eq!(format!("{}", FmtNumber::new(Bytes(1536 * 1024 * 1024))), "1.610612736G");
```

## Formatting flags

Width, alignment, fill, `+`, and zero padding all apply to the number *and* its
prefix:

```rust
use kilomega::FmtNumber;

assert_eq!(format!("[{:>8}]", FmtNumber::new(1500.0)), "[    1.5k]");
assert_eq!(format!("[{:*^8}]", FmtNumber::new(1500.0)), "[**1.5k**]");
assert_eq!(format!("{:+}", FmtNumber::new(1500.0)), "+1.5k");
```

## Behaviour

| Case | Output |
| --- | --- |
| `0.0`, `-0.0` | `0`, `-0` |
| `NaN`, `inf`, `-inf` | `NaN`, `inf`, `-inf` |
| `10^-30 <= abs(x) < 10^33` | one of `q r y z a f p n µ m — k M G T P E Z Y R Q` |
| outside that range | scientific notation, e.g. `1e33`, `4.2e-35` |

* `Display` keeps at most **15 significant digits** and drops trailing zeros.
  That hides last-bit float noise (`0.1 + 0.2` prints as `300m`, not
  `300.00000000000004m`) while still round-tripping anything a human reads.
* An explicit precision keeps trailing zeros (`{:.3}` of `1000.0` is `1.00k`)
  and is clamped to `1..=17`.
* Only prefixes up to `Q` (`10^30`) exist, so `10^30` itself is `1Q` while
  `1e33` is already `1e33`.
* Rounding never produces an out-of-range mantissa: `{:.2}` of `999_999.9` is
  `1.00M`, not `1000.00k`.
* The micro prefix is the [micro sign](https://www.unicode.org/charts/PDF/U0080.pdf)
  `U+00B5` (`µ`), not the Greek letter mu (`μ`).

## Minimum supported Rust version

Rust 1.85 (2024 edition).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
