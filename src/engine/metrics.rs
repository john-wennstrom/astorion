//! Engine run metrics.
//!
//! This module defines a small set of structs used to observe and debug engine
//! performance and behavior.
//!
//! The intended usage is:
//!
//! - `Parser::run` for normal operation.
//! - `Parser::run_with_metrics` for profiling, debugging regressions, and
//!   inspecting what a pass produced.
//!
//! Metrics are intentionally simple and *opt-in*:
//!
//! - The hot path can avoid collecting detailed node lists.
//! - Callers can choose the level of visibility they want.
//!
//! ## Design notes
//!
//! - `PassMetrics::nodes` is primarily for debugging and may allocate.
//! - Fields prefixed with `_` are collected for potential future reporting but
//!   are not currently surfaced in user-facing output.

use crate::{Node, ResolvedToken};
use std::time::Duration;

// --- Metrics -----------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct RunMetrics {
    /// Total elapsed time for [`Parser::run_with_metrics`].
    pub total: Duration,
    /// Cumulative time spent in [`Parser::saturate`].
    pub saturation: SaturationMetrics,
    /// Time spent resolving tokens after saturation.
    pub resolve: Duration,
}

/// Timings for the saturation phase.
#[derive(Debug, Default, Clone)]
pub struct SaturationMetrics {
    /// Total elapsed time for saturation (initial regex pass + iterations).
    pub total: Duration,
    /// Metrics for the initial regex-only pass.
    pub initial_regex: PassMetrics,
    /// Metrics for each subsequent saturation iteration.
    pub iterations: Vec<PassMetrics>,
}

/// Timing (and node discovery counts) for a single pass.
#[derive(Debug, Default, Clone)]
pub struct PassMetrics {
    /// Elapsed time for the pass.
    pub duration: Duration,
    /// Number of new nodes added to the stash during the pass.
    pub produced: usize,
    /// New nodes produced in this pass (for debugging).
    pub nodes: Vec<Node>,
    /// Number of rules considered (attempted) during this pass.
    pub _rules_considered: usize,
    /// Number of rules that had at least one first-pattern match.
    pub _rules_seeded: usize,
    /// Number of regex first-pattern hits across all rules.
    pub _regex_first_pattern_hits: usize,
}

/// Parser output bundled with timing information.
#[derive(Debug, Clone)]
pub struct RunResult {
    /// All resolved tokens before classifier filtering.
    pub all_tokens: Vec<ResolvedToken>,
    /// Best tokens selected by classifiers.
    pub tokens: Vec<ResolvedToken>,
    /// Timing measurements for the run.
    pub metrics: RunMetrics,
}
