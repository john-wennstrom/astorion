use crate::{Token, TokenKind};

pub mod boundaries;
pub mod grain;
pub mod parse;
pub mod producers;
pub mod shift;
pub mod timezone;

// Re-export commonly used functions
pub use grain::*;
pub use parse::*;

/// Return the first regex capture group from `tokens[0]`.
pub fn first(tokens: &[Token]) -> Option<String> {
    match &tokens.first()?.kind {
        // Groups are already lowercased by the parser.
        TokenKind::RegexMatch(groups) => groups.first().cloned(),
        _ => None,
    }
}
