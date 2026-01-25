use crate::{NumeralData, Token, TokenKind};

/// Return the first regex capture group from `tokens[0]`.
pub fn first_match_lower(tokens: &[Token]) -> Option<String> {
    match &tokens.first()?.kind {
        // Groups are already lowercased by the parser.
        TokenKind::RegexMatch(groups) => groups.first().cloned(),
        _ => None,
    }
}

/// Helper to create a `NumeralData` with given `value`.
pub fn make_numeral(value: f64) -> NumeralData {
    let grain = infer_grain(value);
    let abs_val = value.abs();
    let multipliable = grain.map(|g| (abs_val - 10f64.powi(g as i32)).abs() < f64::EPSILON).unwrap_or(false);
    NumeralData { value, grain, multipliable }
}

/// Parse a decimal number string into `f64`.
pub fn parse_decimal(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

/// Parse a numeric string into `f64` (alias for `parse_decimal` for now).
pub fn parse_double(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

/// Infer the power-of-ten "grain" for a numeral value. For integers that end
/// with at least one zero, the grain is the count of trailing zeros; otherwise
/// `None`.
pub fn infer_grain(value: f64) -> Option<u32> {
    let abs_val = value.abs();

    if abs_val == 0.0 || value.fract().abs() > f64::EPSILON {
        return None;
    }

    let mut n = abs_val as i64;
    let mut grain = 0u32;
    while n % 10 == 0 {
        grain += 1;
        n /= 10;
    }

    if grain > 0 { Some(grain) } else { None }
}

/// Create a NumeralData with explicit grain/multipliable flags.
pub fn make_numeral_with(value: f64, grain: Option<u32>, multipliable: bool) -> NumeralData {
    NumeralData { value, grain, multipliable }
}

/// Convert an integer value into its fractional decimal form (e.g. 12 -> 0.12).
pub fn decimals_to_double(value: f64) -> f64 {
    let abs_val = value.abs();
    if abs_val == 0.0 {
        return 0.0;
    }

    let mut n = abs_val as u64;
    let mut digits = 0u32;
    while n > 0 {
        digits += 1;
        n /= 10;
    }

    value / 10f64.powi(digits as i32)
}

/// Multiply two numerals, carrying over the grain from the multiplier when available.
pub fn multiply_numerals(nd1: &NumeralData, nd2: &NumeralData) -> NumeralData {
    make_numeral_with(nd1.value * nd2.value, nd2.grain, false)
}
