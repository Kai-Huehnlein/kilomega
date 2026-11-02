//! Prefix tables and the supported prefix systems.

/// SI prefixes for magnitudes of `10^0` and above, in steps of `10^3`.
pub(crate) const MACRO_SUFFIXES: &[&str] = &[
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
pub(crate) const MICRO_SUFFIXES: &[&str] = &[
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

/// IEC binary prefixes, in steps of `1024` (`2^10`).
///
/// Binary prefixes only exist for magnitudes of `1` and above, so there is no
/// counterpart for `m`, `µ`, and friends.
pub(crate) const BINARY_SUFFIXES: &[&str] = &[
    "",   // 1024^0
    "Ki", // kibi
    "Mi", // mebi
    "Gi", // gibi
    "Ti", // tebi
    "Pi", // pebi
    "Ei", // exbi
    "Zi", // zebi
    "Yi", // yobi
    "Ri", // robi
    "Qi", // quebi
];

/// Largest SI prefix index, i.e. `Q` (`10^30`).
pub(crate) const MAX_SI_INDEX: i32 = MACRO_SUFFIXES.len() as i32 - 1;
/// Smallest SI prefix index, i.e. `q` (`10^-30`).
pub(crate) const MIN_SI_INDEX: i32 = -(MICRO_SUFFIXES.len() as i32 - 1);
/// Largest binary prefix index, i.e. `Qi` (`2^100`).
pub(crate) const MAX_BINARY_INDEX: i32 = BINARY_SUFFIXES.len() as i32 - 1;

/// The prefix system used when formatting.
///
/// ```
/// use kilomega::{FmtNumber, PrefixSystem};
///
/// let bytes = FmtNumber::new(1536u64).with_prefix_system(PrefixSystem::Binary);
/// assert_eq!(format!("{bytes}"), "1.5Ki");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PrefixSystem {
    /// SI prefixes in steps of `1000`, keeping the mantissa in `[1, 1000)`.
    ///
    /// This is the default and handles both large and small magnitudes, from
    /// `q` (`10^-30`) through `Q` (`10^30`).
    #[default]
    Si,
    /// IEC binary prefixes in steps of `1024`, keeping the mantissa in
    /// `[1, 1024)`.
    ///
    /// Prefixes run from `Ki` (`2^10`) through `Qi` (`2^100`). Values below
    /// `1` have no binary prefix and are printed as plain decimals, and values
    /// that exceed `Qi` fall back to scientific notation.
    Binary,
}

impl PrefixSystem {
    /// The SI prefix for `index`, where `index = 0` is `10^0`.
    pub(crate) fn si_suffix(index: i32) -> &'static str {
        let magnitude = usize::try_from(index.unsigned_abs()).unwrap_or(usize::MAX);
        let table = if index >= 0 {
            MACRO_SUFFIXES
        } else {
            MICRO_SUFFIXES
        };
        table.get(magnitude).copied().unwrap_or_default()
    }

    /// The binary prefix for `index`, where `index = 0` is `1024^0`.
    pub(crate) fn binary_suffix(index: i32) -> &'static str {
        let magnitude = usize::try_from(index.unsigned_abs()).unwrap_or(usize::MAX);
        BINARY_SUFFIXES.get(magnitude).copied().unwrap_or_default()
    }
}
