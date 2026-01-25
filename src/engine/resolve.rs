//! Token resolution.
//!
//! Saturation produces `Node`s, which are intermediate parse results containing a
//! span (`Range`) and a `Token`. Resolution turns a `Node` into a user-facing
//! `ResolvedToken` by:
//!
//! - Interpreting the token value (dimension-specific logic)
//! - Formatting a canonical value string
//! - Marking whether the result is *latent*
//! - Applying option-based filtering (where applicable)
//!
//! ## Where this fits
//!
//! The parser (`parser.rs`) builds a stash of nodes. Once saturation reaches a
//! fixpoint, the engine resolves those nodes here and then applies final
//! selection/filtering.
//!
//! ## Extension points
//!
//! In Duckling, resolution is per-dimension. As more dimensions are ported,
//! prefer the following structure:
//!
//! - `resolve_node` stays as a thin, stable wrapper.
//! - The dimension dispatch calls small, dimension-specific functions/modules.
//! - Dimension-specific tests live alongside the relevant rule sets.

use crate::rules::time::normalize::{format_time_value, normalize};
use crate::{Context, Dimension, Node, Options, ResolvedToken, Token, TokenKind};

/// Rough equivalent of Haskell `resolveNode`.
///
/// ```text
/// Node (with dim + range) ──▶ resolve() ──▶ ResolvedToken
///                         └─▶ Option filtering
/// ```
///
/// The wrapper keeps the signature small while still making it clear that the
/// heavy lifting happens in [`resolve`].
pub(crate) fn resolve_node(context: &Context, options: &Options, node: Node) -> Option<ResolvedToken> {
    // In real Duckling, `resolve` is per-dimension.
    // Here we just hardcode something for the Time dimension.
    let (value, latent) = resolve(context, options, &node.token)?;

    if std::env::var_os("RUSTLING_DEBUG_RULES").is_some() {
        eprintln!("[resolve] dim={:?} range={:?} value=\"{}\" latent={}", node.token.dim, node.range, value, latent);
    }

    Some(ResolvedToken { node, value, latent })
}

/// Super-simple "resolve" that returns a dummy value.
///
/// Later, you'll have per-dimension logic here.
///
/// ```text
/// Token ──┬─ Time       -> hard-coded YYYY-MM-DD string
///         ├─ Numeral    -> stringified numeric value (no trailing .0)
///         └─ RegexMatch -> None (not a semantic value)
/// ```
///
/// When porting more Duckling dimensions, keep this function thin and move the
/// rules for each dimension into its own module to keep compilation units small
/// and testable.
fn resolve(context: &Context, _options: &Options, token: &Token) -> Option<(String, bool)> {
    match token.dim {
        Dimension::Time => match &token.kind {
            TokenKind::TimeExpr(expr) => {
                let value = normalize(expr, context.reference_time)?;
                Some((format_time_value(&value), false))
            }
            _ => None,
        },
        Dimension::RegexMatch => None,
        Dimension::Numeral => {
            // Extract numeral value from the token kind and return as string.
            match &token.kind {
                TokenKind::Numeral(data) => {
                    let v = data.value;
                    let s = if v.fract() == 0.0 {
                        // whole number: print without decimal point
                        format!("{}", v as i64)
                    } else {
                        // fractional: keep as float
                        format!("{}", v)
                    };
                    Some((s, false))
                }
                _ => None,
            }
        }
    }
}
