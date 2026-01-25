//! Deduplication keys for saturation.
//!
//! Saturation works by repeatedly applying rules and adding newly produced
//! `Node`s to a stash. Without a *stable* deduplication strategy, the engine can:
//!
//! - Loop indefinitely (rules re-deriving the same results in different ways)
//! - Grow memory unbounded
//! - Produce non-deterministic output (depending on iteration order)
//!
//! This module defines `NodeKey`, a compact, hashable representation of a node
//! that is used by the parser to avoid re-adding equivalent nodes.
//!
//! ## What counts as “the same node”
//!
//! The key combines:
//!
//! - Span (`start`, `end`)
//! - Dimension (`dim`)
//! - Producing rule name (`rule_name`)
//! - A dimension-specific `kind_key`
//!
//! This is deliberately conservative: including `rule_name` avoids collapsing
//! distinct derivations that share the same span/value, which is useful for
//! debugging and can matter for evidence.
//!
//! ## Tradeoffs
//!
//! - `TimeExpr` uses a stringified debug representation for correctness. This is
//!   not allocation-free, but keeps behavior stable until a more structured,
//!   hashable representation is introduced.

use crate::{Dimension, Node};

/// Lightweight key for deduplicating nodes in the stash.
///
/// Avoids allocating strings for the common case, but still ensures correctness.
/// For TimeExpr, we use a formatted string representation as a stable key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NodeKey {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) dim: Dimension,
    pub(crate) rule_name: &'static str,
    pub(crate) kind_key: NodeKindKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum NodeKindKey {
    Numeral(u64),       // Store bits of f64 value for hashing
    TimeExpr(String),   // Use debug format for uniqueness (falls back to allocation for correctness)
    RegexMatch(String), // Keep group 0 for regex matches
}

impl NodeKey {
    pub(crate) fn from_node(node: &Node) -> Self {
        let kind_key = match &node.token.kind {
            crate::TokenKind::Numeral(d) => {
                // Use bits of f64 for hashing to handle floats
                NodeKindKey::Numeral(d.value.to_bits())
            }
            crate::TokenKind::TimeExpr(expr) => {
                // Use debug format for stable key - still better than old approach
                // which formatted the entire node context with many allocations
                NodeKindKey::TimeExpr(format!("{:?}", expr))
            }
            crate::TokenKind::RegexMatch(groups) => {
                // Keep the first capture group for identification
                NodeKindKey::RegexMatch(groups.first().map(|s| s.as_str()).unwrap_or("").to_string())
            }
        };

        NodeKey {
            start: node.range.start,
            end: node.range.end,
            dim: node.token.dim,
            rule_name: node.rule_name,
            kind_key,
        }
    }
}
