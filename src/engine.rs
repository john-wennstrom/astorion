//! Parsing and resolution engine.
//!
//! This module is the *public entry point* for the Duckling-like engine.
//! Historically, the engine lived in a single monolithic `engine.rs`; it is now
//! split into focused submodules under `src/engine/` while keeping public paths
//! stable (for example `crate::engine::Parser` and `crate::engine::BucketMask`).
//!
//! ## How the parts work together
//!
//! At a high level, parsing an input string is a pipeline:
//!
//! ```text
//! rules (all)  ──┐
//!               │  CompiledRules::new           (compiled_rules.rs)
//!               └───────────────┬──────────────
//!                               │
//! input ── TriggerInfo::scan ───┼─ select active rules (buckets + phrases)
//!         (trigger.rs)          │
//!                               v
//!                     Parser::saturate (parser.rs)
//!                       - seed matches (regex-first)
//!                       - iterate to fixpoint
//!                       - add nodes to stash
//!                       - dedup via NodeKey (dedup.rs)
//!                               │
//!                               v
//!                     resolve_node (resolve.rs)
//!                       - per-dimension resolve
//!                       - option filtering
//!                               │
//!                               v
//!                        Vec<ResolvedToken>
//! ```
//!
//! The engine leans on **saturation**: repeatedly apply rules until an
//! iteration produces no new nodes. This mirrors Duckling’s approach and makes
//! rule composition work naturally (one rule can create nodes that enable other
//! rules).
//!
//! ## Responsibilities by module
//!
//! - `compiled_rules.rs`: derives `CompiledRules` from `Rule`s and builds cheap
//!   indexes (bucket lists, per-rule metadata).
//! - `trigger.rs`: scans the raw input to compute coarse buckets and key
//!   phrases for rule activation.
//! - `parser.rs`: performs matching + saturation over a `Stash`, producing
//!   candidate nodes and resolving them to output tokens.
//! - `dedup.rs`: defines stable dedup keys to keep saturation finite.
//! - `resolve.rs`: turns nodes into user-facing values (`ResolvedToken`s), with
//!   dimension-specific logic.
//! - `metrics.rs`: optional timing/debug data for runs and passes.
//!
//! ## Public surface
//!
//! Most code interacts with the engine via:
//!
//! - [`Parser`]
//! - [`CompiledRules`] (optional; for reusing compiled rule sets)
//! - [`BucketMask`] (used by rules to declare coarse requirements)
//!
//! ## Adding new rules / dimensions
//!
//! - New rules are added under `src/rules/**` and ultimately passed into
//!   `Parser::new(..)` / `CompiledRules::new(..)`.
//! - If a new rule needs a new coarse trigger, add a new `BucketMask` bit and
//!   teach `TriggerInfo::scan` + `CompiledRules::new` + `Parser::new_compiled` to
//!   wire it through.
//! - If a new semantic dimension is added, extend `resolve.rs` so that
//!   `resolve_node` can produce a stable canonical value for that dimension.
//!
//! ## Debugging
//!
//! Set `RUSTLING_DEBUG_RULES=1` to print activation and resolution traces.

#[path = "engine/compiled_rules.rs"]
mod compiled_rules;
#[path = "engine/dedup.rs"]
mod dedup;
#[path = "engine/metrics.rs"]
mod metrics;
#[path = "engine/parser.rs"]
mod parser;
#[path = "engine/resolve.rs"]
mod resolve;
#[path = "engine/trigger.rs"]
mod trigger;

#[allow(unused_imports)]
pub use compiled_rules::{BucketMask, CompiledRules, DimensionSet, RuleIndex, RuleMeta};
#[allow(unused_imports)]
pub use metrics::{PassMetrics, RunMetrics, RunResult, SaturationMetrics};
#[allow(unused_imports)]
pub use parser::Parser;
#[allow(unused_imports)]
pub use trigger::TriggerInfo;
