use crate::{Token, TokenKind};

/// Returns true when the token is a positive numeral value (> 0).
pub fn is_positive(t: &Token) -> bool {
    matches!(&t.kind, TokenKind::Numeral(nd) if nd.value > 0.0)
}

/// Returns true when the token has a grain (power-of-ten information).
pub fn has_grain(t: &Token) -> bool {
    matches!(&t.kind, TokenKind::Numeral(nd) if nd.grain.is_some())
}

/// Returns true when the token represents a numeral strictly between the
/// provided bounds.
pub fn number_between<const MIN: i64, const MAX: i64>(t: &Token) -> bool {
    matches!(&t.kind, TokenKind::Numeral(nd) if nd.value >= MIN as f64 && nd.value < MAX as f64)
}

/// Returns true when the token can be used as a multiplier in composite numbers.
pub fn is_multipliable(t: &Token) -> bool {
    matches!(&t.kind, TokenKind::Numeral(nd) if nd.multipliable)
}

/// Returns true when the token holds an integral value.
pub fn is_integer(t: &Token) -> bool {
    matches!(&t.kind, TokenKind::Numeral(nd) if nd.value.fract().abs() < f64::EPSILON)
}

/// Returns true when the token is a tens numeral between 20 and 90 (inclusive)
/// that is also a multiple of ten.
pub fn tens_multiple_between_20_and_90(t: &Token) -> bool {
    matches!(&t.kind, TokenKind::Numeral(nd)
        if nd.value >= 20.0
            && nd.value <= 90.0
            && (nd.value % 10.0).abs() < f64::EPSILON)
}
