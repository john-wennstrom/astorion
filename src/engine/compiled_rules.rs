//! Rule compilation and indexing.
//!
//! This module holds the *static* side of the engine: the structures derived from
//! the full rule list that make a parse run faster and more predictable.
//!
//! In this engine, parsing is intentionally split into two phases:
//!
//! 1. **Compile/index rules** (this module): create a cheap representation of the
//!    rule set (`CompiledRules`) and pre-index it with coarse metadata.
//! 2. **Run** (see `parser.rs`): scan the input for coarse triggers (`trigger.rs`),
//!    select a subset of rules, then perform saturation and resolution.
//!
//! The indexing currently supports:
//!
//! - **Buckets** (`BucketMask`): coarse boolean features of the input (e.g.
//!   “contains digits”) to quickly discard entire swathes of rules.
//! - **Phrases** (stored on each `RuleMeta`): key words/phrases used for further
//!   gating in the parser.
//!
//! ## Extension points
//!
//! - Adding a new bucket:
//!   1. Add a `BucketMask` bit.
//!   2. Add a `BUCKET_*` constant and bump `BUCKET_COUNT`.
//!   3. Teach `CompiledRules::new` to index that bucket.
//!   4. Teach `TriggerInfo::scan` (in `trigger.rs`) to detect it.
//!   5. Teach `Parser::new_compiled` (in `parser.rs`) to activate rules from it.
//!
//! - Adding new per-rule metadata:
//!   extend `RuleMeta` and populate it from the `Rule` in `CompiledRules::new`.
//!
//! ## Invariants
//!
//! - `RuleId` is an index into `CompiledRules::rules` and `CompiledRules::metas`.
//!   Those vectors must stay aligned.
//! - `RuleIndex::by_bucket` uses fixed indices (`BUCKET_*`) to avoid `HashMap`
//!   overhead in the hot path.

use crate::{Dimension, Rule};

// --- Rule compilation and indexing -------------------------------------------

/// Rule identifier (index into the rules vector).
pub(crate) type RuleId = usize;

bitflags::bitflags! {
    /// Coarse buckets for fast input classification.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BucketMask: u32 {
        const HAS_DIGITS   = 1 << 0;
        const HAS_COLON    = 1 << 1;
        const HAS_AMPM     = 1 << 2;
        const WEEKDAYISH   = 1 << 3;
        const MONTHISH     = 1 << 4;
        const ORDINALISH   = 1 << 5;
    }
}

bitflags::bitflags! {
    /// Tracks which dimensions are present in the stash.
    ///
    /// This is used by the parser to skip rules that depend on dimensions that
    /// cannot possibly match yet.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DimensionSet: u8 {
        const TIME    = 1 << 0;
        const NUMERAL = 1 << 1;
        const REGEX   = 1 << 2;
    }
}

/// Metadata attached to a rule (initially defaults, later filled in selectively).
#[derive(Clone, Copy, Debug)]
pub struct RuleMeta {
    pub required_phrases: &'static [&'static str],
    pub optional_phrases: &'static [&'static str],
    pub buckets: BucketMask,
    pub _deps: &'static [Dimension],
    pub _priority: u16,
}

#[derive(Default, Debug)]
pub struct RuleIndex {
    pub always_on: Vec<RuleId>,
    pub by_bucket: [Vec<RuleId>; BUCKET_COUNT],
}

pub const BUCKET_COUNT: usize = 6;
pub const BUCKET_HAS_DIGITS: usize = 0;
pub const BUCKET_HAS_COLON: usize = 1;
pub const BUCKET_HAS_AMPM: usize = 2;
pub const BUCKET_WEEKDAYISH: usize = 3;
pub const BUCKET_MONTHISH: usize = 4;
pub const BUCKET_ORDINALISH: usize = 5;

/// Pre-compiled rule set with metadata and indexes.
#[derive(Debug)]
pub struct CompiledRules<'a> {
    pub rules: Vec<&'a Rule>,
    pub metas: Vec<RuleMeta>,
    pub index: RuleIndex,
}

impl<'a> CompiledRules<'a> {
    /// Create a compiled rule set from a slice of rules.
    ///
    /// Notes:
    /// - This is intentionally lightweight: it does not rewrite patterns, does
    ///   not build automata, and does not allocate per-rule regex state.
    /// - Metadata currently comes directly from `Rule` fields.
    pub fn new(rules: &'a [Rule]) -> Self {
        let rule_refs: Vec<&Rule> = rules.iter().collect();

        // Extract metadata from rules
        let metas: Vec<RuleMeta> = rule_refs
            .iter()
            .map(|r| RuleMeta {
                required_phrases: r.required_phrases,
                optional_phrases: r.optional_phrases,
                buckets: BucketMask::from_bits_truncate(r.buckets),
                _deps: r.deps,
                _priority: r.priority,
            })
            .collect();

        // Build indexes
        let mut index = RuleIndex::default();

        for (id, meta) in metas.iter().enumerate() {
            if meta.buckets.is_empty() {
                // No bucket requirements -> always on (phrase filtering will happen later)
                index.always_on.push(id);
            } else {
                // Index by buckets using fixed array
                if meta.buckets.contains(BucketMask::HAS_DIGITS) {
                    index.by_bucket[BUCKET_HAS_DIGITS].push(id);
                }
                if meta.buckets.contains(BucketMask::HAS_COLON) {
                    index.by_bucket[BUCKET_HAS_COLON].push(id);
                }
                if meta.buckets.contains(BucketMask::HAS_AMPM) {
                    index.by_bucket[BUCKET_HAS_AMPM].push(id);
                }
                if meta.buckets.contains(BucketMask::WEEKDAYISH) {
                    index.by_bucket[BUCKET_WEEKDAYISH].push(id);
                }
                if meta.buckets.contains(BucketMask::MONTHISH) {
                    index.by_bucket[BUCKET_MONTHISH].push(id);
                }
                if meta.buckets.contains(BucketMask::ORDINALISH) {
                    index.by_bucket[BUCKET_ORDINALISH].push(id);
                }
            }
        }

        CompiledRules { rules: rule_refs, metas, index }
    }
}
