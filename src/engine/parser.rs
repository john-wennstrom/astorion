//! Matching and saturation parser.
//!
//! This module is the operational core of the engine:
//!
//! - Select a subset of rules that are plausible for the input (bucket + phrase
//!   gating; see `compiled_rules.rs` and `trigger.rs`).
//! - Repeatedly apply those rules to build up a `Stash` of `Node`s.
//! - Deduplicate produced nodes to keep saturation finite and deterministic
//!   (see `dedup.rs`).
//! - Resolve final nodes into `ResolvedToken`s (see `resolve.rs`) and perform
//!   final filtering/selection.
//!
//! The implementation is intentionally conservative: it favors clarity and
//! Duckling-like semantics (saturation/fixpoint) over aggressive micro-opts.
//! Many improvements described in `ENGINE_REFACTOR_PLAN.md` can be added without
//! changing the public API.
//!
//! ## Key concepts
//!
//! - **Rule** (`crate::time_expr::Rule`): a sequence of `Pattern`s with a production.
//! - **Node** (`crate::time_expr::Node`): a matched token with a span (`Range`) and a
//!   `Token` value.
//! - **Stash** (`crate::time_expr::Stash`): the growing set of discovered nodes.
//! - **Saturation**: repeatedly apply rules until an iteration produces no new
//!   nodes (a fixpoint).
//!
//! ## Pass structure
//!
//! Saturation is typically performed in passes to bias cheaper work first:
//!
//! ```text
//! (0) trigger scan         -> buckets + phrases
//! (1) initial regex pass   -> seed from raw input
//! (2) iterative passes     -> mix regex + predicate rules as stash grows
//! (3) resolve + filter     -> ResolvedToken output
//! ```
//!
//! The exact ordering and gating logic lives in this module, but the output
//! should remain deterministic given the same input, rules, and context.
//!
//! ## Debugging
//!
//! Setting `RUSTLING_DEBUG_RULES=1` prints useful trace information about rule
//! activation and resolution.

use super::compiled_rules::{
    BUCKET_HAS_AMPM, BUCKET_HAS_COLON, BUCKET_HAS_DIGITS, BUCKET_MONTHISH, BUCKET_ORDINALISH, BUCKET_WEEKDAYISH,
    BucketMask, CompiledRules, DimensionSet, RuleId,
};
use super::dedup::NodeKey;
use super::metrics::{PassMetrics, RunMetrics, RunResult, SaturationMetrics};
use super::resolve::resolve_node;
use super::trigger::TriggerInfo;
use crate::{Context, Dimension, Node, Options, Pattern, Range, ResolvedToken, Rule, Stash, Token, TokenKind};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

// Move the parser/partial-match implementation to module scope so other modules
// (for example `main.rs`) can construct and run the Parser directly.
/// Internal helper representing a partially matched rule as the engine
/// advances through the pattern. `route` holds the matched `Node`s so far.
///
/// Visual layout of a `PartialMatch` for a two-element rule:
///
/// ```text
/// pattern: [Regex("today"), Predicate(is_time)]
///          ^ next_idx (0-based) when the first token is consumed
///
/// route: [ Node(range:0..5, dim:RegexMatch) ]
/// position points to the end of the last consumed node (here: 5)
/// ```
struct PartialMatch<'a> {
    rule: &'a Rule,
    next_idx: usize,
    position: usize,
    route: Vec<Node>,
}

/// Parser orchestrates applying `Rule`s against an input string.
///
/// Usage: create with `Parser::new(input, &rules)` then call `run(context, options)`.
///
/// High-level flow inside `run`:
///
/// ```text
/// new() -> saturate() -> resolve_filtered()
///            │             └─ discard subsumed nodes
///            └─ repeatedly grow stash using rules
/// ```
#[derive(Debug)]
pub struct Parser<'a> {
    /// Input text to parse.
    input: &'a str,
    /// Stash of discovered nodes (intermediate parse results).
    stash: Stash,
    /// Set of seen node keys used to avoid re-adding identical nodes.
    seen: HashSet<NodeKey>,
    /// Compiled rules (shared reference).
    compiled: CompiledRules<'a>,
    /// Cached list of rules that start with a `Regex` pattern.
    regex_rules: Vec<&'a Rule>,
    /// Cached list of rules that start with a `Predicate` pattern.
    predicate_rules: Vec<&'a Rule>,
}

impl<'a> Parser<'a> {
    /// Create a new `Parser` for `input` using pre-compiled rules.
    pub fn new_compiled(input: &'a str, compiled: CompiledRules<'a>) -> Self {
        // Scan input to get coarse buckets + key phrases.
        let trigger_info = TriggerInfo::scan(input);

        if std::env::var_os("RUSTLING_DEBUG_RULES").is_some() {
            eprintln!("[trigger_scan] buckets={:?} phrases={:?}", trigger_info.buckets, trigger_info.phrases);
        }

        // Compute active rule set from trigger buckets.
        let mut active_rule_ids: HashSet<RuleId> = compiled.index.always_on.iter().copied().collect();

        // Add rules whose bucket requirements are satisfied by the input
        // Direct checks avoid HashMap overhead
        if trigger_info.buckets.contains(BucketMask::HAS_DIGITS) {
            active_rule_ids.extend(&compiled.index.by_bucket[BUCKET_HAS_DIGITS]);
        }
        if trigger_info.buckets.contains(BucketMask::HAS_COLON) {
            active_rule_ids.extend(&compiled.index.by_bucket[BUCKET_HAS_COLON]);
        }
        if trigger_info.buckets.contains(BucketMask::HAS_AMPM) {
            active_rule_ids.extend(&compiled.index.by_bucket[BUCKET_HAS_AMPM]);
        }
        if trigger_info.buckets.contains(BucketMask::WEEKDAYISH) {
            active_rule_ids.extend(&compiled.index.by_bucket[BUCKET_WEEKDAYISH]);
        }
        if trigger_info.buckets.contains(BucketMask::MONTHISH) {
            active_rule_ids.extend(&compiled.index.by_bucket[BUCKET_MONTHISH]);
        }
        if trigger_info.buckets.contains(BucketMask::ORDINALISH) {
            active_rule_ids.extend(&compiled.index.by_bucket[BUCKET_ORDINALISH]);
        }

        // Phrase gating - filter out rules whose phrase requirements are not met.
        let mut phrase_filtered = 0;
        active_rule_ids.retain(|&id| {
            let meta = &compiled.metas[id];

            // Check required_phrases (AND logic - all must be present)
            if !meta.required_phrases.is_empty() {
                let all_required_present =
                    meta.required_phrases.iter().all(|&phrase| trigger_info.phrases.contains(phrase));
                if !all_required_present {
                    phrase_filtered += 1;
                    return false;
                }
            }

            // Check optional_phrases (OR logic - at least one must be present)
            if !meta.optional_phrases.is_empty() {
                let any_optional_present =
                    meta.optional_phrases.iter().any(|&phrase| trigger_info.phrases.contains(phrase));
                if !any_optional_present {
                    phrase_filtered += 1;
                    return false;
                }
            }

            true
        });

        if std::env::var_os("RUSTLING_DEBUG_RULES").is_some() {
            eprintln!(
                "[active_rules] {}/{} rules active (phrase-filtered: {})",
                active_rule_ids.len(),
                compiled.rules.len(),
                phrase_filtered
            );
        }

        let regex_rules: Vec<&Rule> = compiled
            .rules
            .iter()
            .enumerate()
            .filter(|(id, _)| active_rule_ids.contains(id))
            .filter(|(_, r)| matches!(r.pattern.first(), Some(Pattern::Regex(_))))
            .map(|(_, r)| *r)
            .collect();

        let predicate_rules: Vec<&Rule> = compiled
            .rules
            .iter()
            .enumerate()
            .filter(|(id, _)| active_rule_ids.contains(id))
            .filter(|(_, r)| matches!(r.pattern.first(), Some(Pattern::Predicate(_))))
            .map(|(_, r)| *r)
            .collect();

        if std::env::var_os("RUSTLING_DEBUG_RULES").is_some() {
            eprintln!("[regex_rules] {} regex rules, {} predicate rules", regex_rules.len(), predicate_rules.len());
            eprintln!("[regex_rules] Rules with regex first pattern:");
            for rule in &regex_rules {
                eprintln!("  - {}", rule.name);
            }
        }

        Parser { input, stash: Stash::empty(), seen: HashSet::new(), compiled, regex_rules, predicate_rules }
    }

    /// Create a new `Parser` for `input` using `rules`.
    ///
    /// This is a convenience wrapper that builds a temporary `CompiledRules`.
    /// Two separate vectors are built for rules that start with a regex vs. a
    /// predicate. This lets [`saturate`] bias the first pass toward cheap,
    /// positional regex matches, then follow up with predicate-driven matches
    /// that rely on previously discovered nodes.
    pub fn new(input: &'a str, rules: &'a [Rule]) -> Self {
        // We build `CompiledRules` on the fly.
        // Callers that want to reuse compiled rules can use `new_compiled`.
        Self::new_compiled(input, CompiledRules::new(rules))
    }

    pub(crate) fn active_rule_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> =
            self.regex_rules.iter().chain(self.predicate_rules.iter()).map(|r| r.name).collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Find nodes that match `pat` and start exactly at `position`.
    ///
    /// ```text
    /// input: "tomorrow at 5"
    /// position: 10 (start of "5")
    /// Pattern::Regex("\\d+") -> Node at 10..11
    /// Pattern::Predicate(is_time) -> Nodes pulled from stash at same offset
    /// ```
    fn lookup_item(&self, pat: &Pattern, position: usize) -> Vec<Node> {
        match pat {
            Pattern::Regex(re) => {
                let mut res = Vec::new();
                for caps in re.captures_iter(self.input) {
                    let m = caps.get(0).unwrap();
                    if m.start() == position {
                        let groups: Vec<String> =
                            (0..caps.len()).filter_map(|i| caps.get(i).map(|g| g.as_str().to_lowercase())).collect();
                        res.push(Node {
                            range: Range { start: m.start(), end: m.end() },
                            token: Token { dim: Dimension::RegexMatch, kind: TokenKind::RegexMatch(groups) },
                            rule_name: "<regex>",
                            evidence: Vec::new(),
                        });
                    }
                }
                res
            }
            Pattern::Predicate(pred) => self
                .stash
                .to_pos_ordered_list_from(position)
                .into_iter()
                .filter(|n| n.range.start == position && pred(&n.token))
                .collect(),
        }
    }

    /// Find nodes that match `pat` anywhere in the input.
    ///
    /// Used to seed partial matches for rules whose first pattern can match at
    /// any position. The regex branch scans the raw input, while the predicate
    /// branch leverages every node already in the stash.
    fn lookup_item_anywhere(&self, pat: &Pattern) -> Vec<Node> {
        match pat {
            Pattern::Regex(re) => {
                let mut res = Vec::new();
                for caps in re.captures_iter(self.input) {
                    let m = caps.get(0).unwrap();
                    let groups: Vec<String> =
                        (0..caps.len()).filter_map(|i| caps.get(i).map(|g| g.as_str().to_lowercase())).collect();
                    res.push(Node {
                        range: Range { start: m.start(), end: m.end() },
                        token: Token { dim: Dimension::RegexMatch, kind: TokenKind::RegexMatch(groups) },
                        rule_name: "<regex>",
                        evidence: Vec::new(),
                    });
                }
                res
            }
            Pattern::Predicate(pred) => {
                self.stash.to_pos_ordered_list().into_iter().filter(|n| pred(&n.token)).collect()
            }
        }
    }

    /// Attempt to match a rule's first pattern anywhere and return initial
    /// `PartialMatch` instances for each match.
    ///
    /// ```text
    /// rule.pattern = [Regex(A), Predicate(B), Predicate(C)]
    /// 1. find all Regex(A) hits
    /// 2. create PartialMatch for each, pointing next_idx to Predicate(B)
    /// ```
    fn seed_first_pattern_anywhere(&self, rule: &'a Rule) -> Vec<PartialMatch<'a>> {
        if rule.pattern.is_empty() {
            return Vec::new();
        }
        let first = &rule.pattern[0];
        self.lookup_item_anywhere(first)
            .into_iter()
            .map(|node| PartialMatch { rule, next_idx: 1, position: node.range.end, route: vec![node] })
            .collect()
    }

    /// Depth-first expansion of `PartialMatch` objects until the entire rule is
    /// satisfied.
    ///
    /// Uses a stack-based DFS approach to avoid excessive route cloning.
    /// We process each PartialMatch independently and use a stack to iterate
    /// through branches instead of recursion.
    ///
    /// ```text
    /// [m0] --Regex--> [m1] --Predicate--> [m2]
    ///   │                           │
    ///   └─ (backtracks)             └─ success -> collected
    /// ```
    fn match_all(&self, input_matches: Vec<PartialMatch<'a>>) -> Vec<PartialMatch<'a>> {
        let mut results = Vec::new();
        let mut stack: Vec<PartialMatch<'a>> = input_matches;

        while let Some(m) = stack.pop() {
            if m.next_idx >= m.rule.pattern.len() {
                // Pattern complete - this is a result
                results.push(m);
                continue;
            }

            let pat = &m.rule.pattern[m.next_idx];
            let nodes = self.lookup_item(pat, m.position);

            // For each matching node, create a new partial match
            // Push in reverse order so we explore them in forward order (stack is LIFO)
            for node in nodes.into_iter().rev() {
                let mut new_route = m.route.clone();
                new_route.push(node.clone());
                stack.push(PartialMatch {
                    rule: m.rule,
                    next_idx: m.next_idx + 1,
                    position: node.range.end,
                    route: new_route,
                });
            }
        }

        results
    }

    /// Convert a completed `PartialMatch` into a `Node` by invoking the rule's
    /// production callback.
    ///
    /// ```text
    /// route tokens ──> production closure ──> Token ──> Node spanning route
    /// ```
    fn produce_node(&self, m: &PartialMatch) -> Option<Node> {
        if m.next_idx < m.rule.pattern.len() {
            return None;
        }
        let tokens: Vec<Token> = m.route.iter().map(|n| n.token.clone()).collect();
        let debug = std::env::var_os("RUSTLING_DEBUG_RULES").is_some();

        match (m.rule.production)(&tokens) {
            Some(tok) => {
                if let (Some(first), Some(last)) = (m.route.first(), m.route.last()) {
                    if debug {
                        let span_text = &self.input[first.range.start..last.range.end.min(self.input.len())];
                        eprintln!(
                            "[rule:production_ok] name=\"{}\" span={}..{} text=\"{}\" token={:?}",
                            m.rule.name, first.range.start, last.range.end, span_text, tok,
                        );
                    }
                    // Collect evidence: rule names from the route plus nested evidence
                    let mut evidence = Vec::new();
                    for node in &m.route {
                        evidence.push(node.rule_name);
                        evidence.extend_from_slice(&node.evidence);
                    }
                    return Some(Node {
                        range: Range { start: first.range.start, end: last.range.end },
                        token: tok,
                        rule_name: m.rule.name,
                        evidence,
                    });
                }
                None
            }
            None => {
                if debug {
                    eprintln!("[rule:production_none] name=\"{}\" route={:?}", m.rule.name, m.route);
                }
                None
            }
        }
    }

    /// Apply an ordered set of rules once and return the nodes produced.
    ///
    /// Designed to be called from `saturate` with different rule subsets to
    /// keep the staging clear in logs or profilers.
    fn apply_rules_once(&self, rule_set: &[&Rule]) -> (Vec<Node>, usize, usize, usize) {
        let mut discovered = Vec::new();
        let debug = std::env::var_os("RUSTLING_DEBUG_RULES").is_some();
        let mut rules_seeded = 0;
        let mut regex_first_pattern_hits = 0;

        for rule in rule_set {
            let starts = self.seed_first_pattern_anywhere(rule);
            let starts_count = starts.len();

            // Count seeded rules (those with at least one first-pattern match)
            if starts_count > 0 {
                rules_seeded += 1;
                // Count regex hits if the first pattern is a regex
                if matches!(rule.pattern.first(), Some(Pattern::Regex(_))) {
                    regex_first_pattern_hits += starts_count;
                }
            }

            if debug && starts_count > 0 {
                eprintln!("[rule:seed] name=\"{}\" initial_matches={}", rule.name, starts_count);
            }
            let full = self.match_all(starts);
            if debug && !full.is_empty() {
                eprintln!("[rule:full_matches] name=\"{}\" count={}", rule.name, full.len());
            }
            for m in full {
                if let Some(node) = self.produce_node(&m) {
                    discovered.push(node);
                }
            }
        }
        (discovered, rule_set.len(), rules_seeded, regex_first_pattern_hits)
    }

    /// Compute which dimensions are present in the stash.
    fn dimensions_in_stash(&self) -> DimensionSet {
        let mut dims = DimensionSet::empty();
        for node in &self.stash.get_nodes() {
            match node.token.dim {
                Dimension::Time => dims |= DimensionSet::TIME,
                Dimension::Numeral => dims |= DimensionSet::NUMERAL,
                Dimension::RegexMatch => dims |= DimensionSet::REGEX,
            }
        }
        dims
    }

    /// Check if all required dimensions for a rule are present in the stash.
    fn deps_satisfied(rule: &Rule, dims_in_stash: DimensionSet) -> bool {
        // Rules with no deps always run
        if rule.deps.is_empty() {
            return true;
        }
        // Check if all required dimensions exist in stash
        rule.deps.iter().all(|&dep| match dep {
            Dimension::Time => dims_in_stash.contains(DimensionSet::TIME),
            Dimension::Numeral => dims_in_stash.contains(DimensionSet::NUMERAL),
            Dimension::RegexMatch => dims_in_stash.contains(DimensionSet::REGEX),
        })
    }

    /// Saturate the stash by repeatedly applying rules until a fixpoint.
    ///
    /// Visualization:
    ///
    /// ```text
    /// regex_rules pass
    ///      │
    ///      ▼
    ///  stash grows ──┐
    ///                │ predicate + regex passes
    ///                └── repeat until fixed point
    /// ```
    fn saturate(&mut self) -> SaturationMetrics {
        let mut metrics = SaturationMetrics::default();
        let saturation_start = Instant::now();
        let debug = std::env::var_os("RUSTLING_DEBUG_RULES").is_some();

        // Initial regex-first pass.
        let regex_start = Instant::now();
        let (discovered, rules_considered, rules_seeded, regex_first_pattern_hits) =
            self.apply_rules_once(&self.regex_rules);
        let mut newly_added = Stash::empty();
        let mut produced = 0;
        for node in discovered {
            let key = NodeKey::from_node(&node);
            if !self.seen.contains(&key) {
                self.seen.insert(key);
                newly_added.insert(node);
                produced += 1;
            }
        }
        let nodes: Vec<Node> = if debug { newly_added.get_nodes() } else { Vec::new() };
        metrics.initial_regex = PassMetrics {
            duration: regex_start.elapsed(),
            produced,
            nodes,
            _rules_considered: rules_considered,
            _rules_seeded: rules_seeded,
            _regex_first_pattern_hits: regex_first_pattern_hits,
        };
        if newly_added.null() {
            metrics.total = saturation_start.elapsed();
            return metrics;
        }
        self.stash = self.stash.union(&newly_added);

        // Saturation: predicate-first rules then regex rules.
        let mut all_saturate_rules: Vec<&Rule> = Vec::new();
        all_saturate_rules.extend(self.predicate_rules.iter().cloned());
        all_saturate_rules.extend(self.regex_rules.iter().cloned());

        loop {
            let iteration_start = Instant::now();

            // Filter rules based on deps satisfaction.
            let dims_in_stash = self.dimensions_in_stash();
            let saturate_rules: Vec<&Rule> =
                all_saturate_rules.iter().filter(|rule| Self::deps_satisfied(rule, dims_in_stash)).copied().collect();

            let (discovered, rules_considered, rules_seeded, regex_first_pattern_hits) =
                self.apply_rules_once(&saturate_rules);
            let mut newly_added = Stash::empty();
            let mut produced = 0;
            for node in discovered {
                let key = NodeKey::from_node(&node);
                if !self.seen.contains(&key) {
                    self.seen.insert(key);
                    newly_added.insert(node);
                    produced += 1;
                }
            }
            let duration = iteration_start.elapsed();
            let nodes: Vec<Node> = if debug { newly_added.get_nodes() } else { Vec::new() };
            metrics.iterations.push(PassMetrics {
                duration,
                produced,
                nodes,
                _rules_considered: rules_considered,
                _rules_seeded: rules_seeded,
                _regex_first_pattern_hits: regex_first_pattern_hits,
            });
            if newly_added.null() {
                break;
            }
            self.stash = self.stash.union(&newly_added);
        }

        metrics.total = saturation_start.elapsed();
        metrics
    }

    /// Resolve nodes, then filter out spans that are fully contained within a
    /// larger match of the same dimension.
    ///
    /// Important: we filter *after* resolving so that unresolved catch-all
    /// nodes (like raw-input) can't suppress resolvable, more specific parses.
    fn resolve_filtered(&self, context: &Context, options: &Options) -> Vec<ResolvedToken> {
        let mut resolved: Vec<ResolvedToken> =
            self.stash.get_nodes().into_iter().filter_map(|node| resolve_node(context, options, node)).collect();

        // Build priority lookup from rule names.
        let mut rule_priority: HashMap<&str, u16> = HashMap::new();
        for rule in &self.compiled.rules {
            rule_priority.insert(rule.name, rule.priority);
        }

        // Sort with priority as tie-breaker.
        resolved.sort_by(|a, b| {
            let priority_a = rule_priority.get(a.node.rule_name).copied().unwrap_or(0);
            let priority_b = rule_priority.get(b.node.rule_name).copied().unwrap_or(0);

            (a.node.token.dim as u8)
                .cmp(&(b.node.token.dim as u8))
                .then(a.node.range.start.cmp(&b.node.range.start))
                .then(b.node.range.end.cmp(&a.node.range.end))
                // Higher priority wins when ranges are equal.
                .then(priority_b.cmp(&priority_a))
        });

        let mut filtered: Vec<ResolvedToken> = Vec::new();
        let mut last_kept_dim = None;
        let mut last_kept_range: Option<Range> = None;

        for rt in resolved {
            if last_kept_dim != Some(rt.node.token.dim) {
                last_kept_dim = Some(rt.node.token.dim);
                last_kept_range = None;
            }

            let is_subsumed = last_kept_range
                .as_ref()
                .map(|range| {
                    range.start <= rt.node.range.start
                        && range.end >= rt.node.range.end
                        && (range.start != rt.node.range.start || range.end != rt.node.range.end)
                })
                .unwrap_or(false);

            if !is_subsumed {
                last_kept_range = Some(rt.node.range.clone());
                filtered.push(rt);
            }
        }

        filtered
    }

    /// Run the parser (saturate the stash and resolve nodes into `ResolvedToken`s)
    /// and return timing details.
    pub fn run_with_metrics(mut self, context: &Context, options: &Options) -> RunResult {
        let total_start = Instant::now();
        let saturation = self.saturate();
        let resolve_start = Instant::now();
        let all_tokens = self.resolve_filtered(context, options);
        // Classifier deactivated for now - return all tokens
        // let tokens = pick_best_time_tokens(all_tokens.clone(), &self.stash);
        let tokens = all_tokens.clone();
        let resolve = resolve_start.elapsed();
        let total = total_start.elapsed();

        RunResult { all_tokens, tokens, metrics: RunMetrics { total, saturation, resolve } }
    }

    /// Run the parser (saturate the stash and resolve nodes into `ResolvedToken`s).
    /// This is run by the tester
    ///
    /// Convenience wrapper that discards timing details. Use [`run_with_metrics`]
    /// to inspect stage-by-stage durations.
    #[allow(dead_code)]
    pub fn run(self, context: &Context, options: &Options) -> Vec<ResolvedToken> {
        self.run_with_metrics(context, options).tokens
    }
}
