extern crate self as astorion;

use regex::Regex;

#[macro_use]
mod macros;
mod api;
mod engine;
mod rules;

mod time_expr;

pub use api::{
    Context, Entity, NodeSummary, Options, ParseDetails, ParseResult, RegexProfilingOptions, parse,
    parse_verbose_with, parse_with,
};

use crate::time_expr::TimeExpr;

// --- Internal types ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Dimension {
    Time,
    RegexMatch,
    Numeral,
    // later: Number, AmountOfMoney, ...
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub dim: Dimension,
    pub kind: TokenKind,
}

#[derive(Debug, Clone)]
pub(crate) struct NumeralData {
    pub value: f64,
    pub grain: Option<u32>,
    pub multipliable: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum TokenKind {
    Numeral(NumeralData),
    TimeExpr(TimeExpr),
    RegexMatch(Vec<String>),
}

// Trait to convert rule production results into tokens
pub(crate) trait IntoToken {
    fn into_token(self) -> Option<Token>;
}

impl IntoToken for TimeExpr {
    fn into_token(self) -> Option<Token> {
        Some(Token { dim: Dimension::Time, kind: TokenKind::TimeExpr(self) })
    }
}

impl IntoToken for NumeralData {
    fn into_token(self) -> Option<Token> {
        Some(Token { dim: Dimension::Numeral, kind: TokenKind::Numeral(self) })
    }
}

// Pattern items used by rules: either a Regex to match text, or a Predicate
// that matches an existing token in the stash.
#[derive(Debug)]
pub(crate) enum Pattern {
    /// Match a regular expression against the original input. The `Regex`
    /// is stored as a static reference (created via a `regex!` helper macro
    /// in `src/macros.rs`).
    Regex(&'static Regex),

    /// Match an already-discovered `Token` using a predicate function. This
    /// allows rules to combine previously found tokens (from the `Stash`).
    Predicate(fn(&Token) -> bool),
}

pub(crate) type Production = Box<dyn Fn(&[Token]) -> Option<Token> + Send + Sync>;

/// A parsing rule: a name, a positional `pattern` (vector of `Pattern` items)
/// and a `production` function that receives the matched tokens and
/// optionally returns a new `Token`.
///
/// Optional metadata fields enable selective rule activation (Step 3-4).
pub(crate) struct Rule {
    pub name: &'static str,
    pub pattern: Vec<Pattern>,
    pub production: Production,
    /// Required phrases - ALL must appear in input for this rule to activate (AND logic).
    pub required_phrases: &'static [&'static str],
    /// Optional phrases - ANY one must appear in input for this rule to activate (OR logic).
    pub optional_phrases: &'static [&'static str],
    /// Bucket mask - rule only activates if input has matching buckets.
    pub buckets: u32,
    /// Required dimensions in stash before this rule activates.
    pub deps: &'static [Dimension],
    /// Priority for deterministic tie-breaking (higher = preferred).
    pub priority: u16,
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rule")
            .field("name", &self.name)
            .field("pattern", &self.pattern)
            .field("production", &"<function>")
            .field("buckets", &self.buckets)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Range {
    /// Start byte index (inclusive).
    pub start: usize,
    /// End byte index (exclusive).
    pub end: usize,
}

/// Internal resolved token: a `Node` (parse-tree leaf), its resolved string
/// value, and a `latent` flag. This is converted to the public `Entity`
/// by higher-level API functions (not implemented in v1).
#[derive(Debug, Clone)]
pub(crate) struct ResolvedToken {
    pub node: Node,
    pub value: String, // for now, resolved value is just a String
    pub latent: bool,
}

/// Basic parse tree node produced by rules. `Node` pairs a `Token` with the
/// consumed `Range` from the original input.
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub range: Range,
    pub token: Token,
    /// Name of the rule that produced this node (used for ranking/classification).
    pub rule_name: &'static str,
    /// Names of rules that directly contributed to producing this node.
    ///
    /// This is derived from the matched route (the tokens consumed by the rule),
    /// and is used as classifier "features".
    pub evidence: Vec<&'static str>,
}

// --- Stash: lightweight container for discovered nodes ----------------------

#[derive(Debug, Clone)]
pub(crate) struct Stash {
    nodes: Vec<Node>,
}

impl Stash {
    /// Create an empty `Stash`.
    pub fn empty() -> Self {
        Stash { nodes: Vec::new() }
    }

    /// Return true if the stash is empty.
    pub fn null(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the nodes in this stash.
    pub fn get_nodes(&self) -> Vec<Node> {
        self.nodes.clone()
    }

    /// Return nodes sorted by `(start, end)`.
    pub fn to_pos_ordered_list(&self) -> Vec<Node> {
        let mut v = self.nodes.clone();
        v.sort_by_key(|n| (n.range.start, n.range.end));
        v
    }

    /// Return nodes sorted and filtered to those starting at or after `position`.
    pub fn to_pos_ordered_list_from(&self, position: usize) -> Vec<Node> {
        self.to_pos_ordered_list().into_iter().filter(|n| n.range.start >= position).collect()
    }

    /// Union two stashes; keeps nodes deduplicated by (start,end,dim[,numeral value]).
    ///
    /// When two nodes share the same position and dimension they are
    /// de-duplicated; for `Numeral` tokens the numeric value is also
    /// compared to avoid merging distinct numbers.
    pub fn union(&self, other: &Stash) -> Stash {
        let mut combined = self.nodes.clone();
        combined.extend(other.nodes.clone());

        // Deduplicate by position + dimension + token content.
        //
        // This must stay in sync with `Parser::node_key` semantics: many rules
        // can produce multiple distinct `Time` values for the same span (e.g.
        // raw-input vs a normalized holiday description), and we must not
        // collapse them before resolution.
        combined.sort_by_key(|n| (n.range.start, n.range.end));
        combined.dedup_by(|a, b| {
            if a.range.start != b.range.start
                || a.range.end != b.range.end
                || a.token.dim != b.token.dim
                || a.rule_name != b.rule_name
                || a.evidence != b.evidence
            {
                return false;
            }

            match (&a.token.kind, &b.token.kind) {
                (crate::TokenKind::Numeral(da), crate::TokenKind::Numeral(db)) => da.value == db.value,
                (crate::TokenKind::TimeExpr(ea), crate::TokenKind::TimeExpr(eb)) => ea == eb,
                (crate::TokenKind::RegexMatch(ga), crate::TokenKind::RegexMatch(gb)) => ga.first() == gb.first(),
                _ => false,
            }
        });

        Stash { nodes: combined }
    }

    /// Insert a node into the stash (appends to internal vector).
    pub fn insert(&mut self, node: Node) {
        self.nodes.push(node);
    }
}

// (Public API lives in `src/api.rs`.)

// --- Internal pipeline ------------------------------------------------------

// For v1: dumb "analyzer".
// Here is where you’ll later plug in real rules / models.
//
// For demonstration, we:
//   - look for the word "tomorrow"
//   - return a single Time token if found
// fn analyze(input: &str) -> Vec<ResolvedToken> {
//     let needle = "tomorrow";
//     if let Some(start) = input.find(needle) {
//         let end = start + needle.len();
//
//         let token = Token { dim: Dimension::Time };
//
//         let resolved = ResolvedToken {
//             range: Range { start, end },
//             token,
//             // Later this could be a structured datetime value
//             value: "2025-12-12".to_string(), // dummy example
//             latent: false,
//         };
//
//         vec![resolved]
//     } else {
//         vec![]
//     }
// }

// Convert internal representation to API `Entity`.
// fn format_token(input: &str, resolved: ResolvedToken) -> Entity {
//     let Range { start, end } = resolved.range;
//     let dim = resolved.token.dim;
//
//     let body = input.get(start..end).unwrap_or("").to_string();
//     let name = to_name(dim).to_string();
//
//     Entity {
//         name,
//         body,
//         value: resolved.value,
//         start,
//         end,
//         latent: resolved.latent,
//     }
// }

// Map dimension to its string name.
// fn to_name(dim: Dimension) -> &'static str {
//     match dim {
//         Dimension::Time => "time",
//     }
// }
