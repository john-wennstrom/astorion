use crate::engine;
use crate::{Dimension, ResolvedToken, Rule};
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use once_cell::sync::Lazy;
use std::time::Duration;

static DEFAULT_RULES: Lazy<Vec<Rule>> = Lazy::new(crate::rules::time::rules::get);

/// Parsing context.
///
/// This holds environment needed to resolve relative expressions (like "tomorrow").
#[derive(Debug, Clone)]
pub struct Context {
    /// Reference datetime used to resolve relative expressions.
    pub reference_time: NaiveDateTime,
}

impl Default for Context {
    fn default() -> Self {
        if cfg!(test) {
            let date = NaiveDate::from_ymd_opt(2013, 2, 12).unwrap();
            let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            Self { reference_time: NaiveDateTime::new(date, time) }
        } else {
            Self { reference_time: Local::now().naive_local() }
        }
    }
}

/// Options that affect parsing/resolution behavior.
///
/// This is intentionally minimal today and will grow as more Duckling-like
/// configuration is implemented.
#[derive(Debug, Clone, Default)]
pub struct Options {
    // later: debug flags, locale, timezone, etc.
}

/// A resolved entity found in input.
///
/// `start`/`end` are byte offsets into the original input.
#[derive(Debug, Clone)]
pub struct Entity {
    /// Name of the dimension, e.g. `"time"` or `"numeral"`.
    pub name: String,
    /// Slice of the original input that matched.
    pub body: String,
    /// Resolved value (currently formatted as a string).
    pub value: String,
    /// Start byte index of the match.
    pub start: usize,
    /// End byte index of the match (exclusive).
    pub end: usize,
    /// Whether this is a "latent" (low-confidence) match.
    pub latent: bool,
    /// Name of the rule that produced this entity.
    pub rule: String,
}

/// Result from [`parse`] and [`parse_with`].
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The parsed input text.
    pub text: String,
    /// Resolved entities.
    pub results: Vec<Entity>,
    /// Total elapsed time spent parsing + resolving.
    pub elapsed: Duration,
}

/// A compact per-pass saturation trace.
#[derive(Debug, Clone)]
pub struct SaturationPass {
    pub pass: usize,
    pub duration: Duration,
    pub produced: usize,
    pub samples: Vec<NodeSummary>,
}

/// A compact node summary used in verbose traces.
#[derive(Debug, Clone)]
pub struct NodeSummary {
    pub start: usize,
    pub end: usize,
    pub rule: String,
    pub preview: String,
}

/// Additional details returned by [`parse_verbose`] and [`parse_verbose_with`].
///
/// This is intentionally compact: it’s meant for debugging and performance
/// inspection without dumping the entire internal state.
#[derive(Debug, Clone)]
pub struct ParseDetails {
    /// Total elapsed time.
    pub total: Duration,
    /// Time spent in saturation (rule application) + per-pass trace.
    pub saturation_total: Duration,
    pub saturation: Vec<SaturationPass>,
    /// Time spent resolving and filtering candidates.
    pub resolve: Duration,
    /// Names of rules that were active for this input.
    pub active_rules: Vec<String>,
    /// All resolved candidates before classifier filtering.
    pub all_candidates: Vec<Entity>,
}

/// Result from [`parse_verbose`] and [`parse_verbose_with`].
#[derive(Debug, Clone)]
pub struct ParseResultVerbose {
    pub text: String,
    pub results: Vec<Entity>,
    pub elapsed: Duration,
    pub details: ParseDetails,
}

/// Parse `text` using the default ruleset and a default [`Context`].
///
/// # Example
/// ```
/// use astorion::parse;
///
/// let out = parse("today");
/// assert!(!out.results.is_empty());
/// ```
pub fn parse(text: &str) -> ParseResult {
    parse_with(text, &Context::default(), &Options::default())
}

/// Parse `text` using the default ruleset and the provided `context`/`options`.
///
/// Use this when you want deterministic parsing by supplying a reference time.
pub fn parse_with(text: &str, context: &Context, options: &Options) -> ParseResult {
    let parser = engine::Parser::new(text, &DEFAULT_RULES);
    let run = parser.run_with_metrics(context, options);

    ParseResult {
        text: text.to_string(),
        results: run.tokens.iter().map(|rt| resolved_to_entity(text, rt)).collect(),
        elapsed: run.metrics.total,
    }
}

#[allow(dead_code)]
pub fn parse_verbose(text: &str) -> ParseResultVerbose {
    parse_verbose_with(text, &Context::default(), &Options::default())
}

/// Parse `text` with `context`/`options` and return extra (compact) debug details.
///
/// This is useful for profiling and rule debugging. The default [`parse_with`]
/// path does not allocate these extra traces.
pub fn parse_verbose_with(text: &str, context: &Context, options: &Options) -> ParseResultVerbose {
    let parser = engine::Parser::new(text, &DEFAULT_RULES);
    let active_rules = parser.active_rule_names().into_iter().map(|s| s.to_string()).collect();

    let run = parser.run_with_metrics(context, options);

    let results: Vec<Entity> = run.tokens.iter().map(|rt| resolved_to_entity(text, rt)).collect();
    let all_candidates: Vec<Entity> = run.all_tokens.iter().map(|rt| resolved_to_entity(text, rt)).collect();

    let mut saturation: Vec<SaturationPass> = Vec::new();

    let initial = &run.metrics.saturation.initial_regex;
    saturation.push(SaturationPass {
        pass: 0,
        duration: initial.duration,
        produced: initial.produced,
        samples: initial.nodes.iter().take(8).map(node_to_summary).collect(),
    });

    for (idx, pass) in run.metrics.saturation.iterations.iter().enumerate() {
        saturation.push(SaturationPass {
            pass: idx + 1,
            duration: pass.duration,
            produced: pass.produced,
            samples: pass.nodes.iter().take(8).map(node_to_summary).collect(),
        });
    }

    let details = ParseDetails {
        total: run.metrics.total,
        saturation_total: run.metrics.saturation.total,
        saturation,
        resolve: run.metrics.resolve,
        active_rules,
        all_candidates,
    };

    ParseResultVerbose { text: text.to_string(), results, elapsed: run.metrics.total, details }
}

fn resolved_to_entity(input: &str, rt: &ResolvedToken) -> Entity {
    let start = rt.node.range.start;
    let end = rt.node.range.end;
    let body = input.get(start..end).unwrap_or("").to_string();

    Entity {
        name: dimension_name(rt.node.token.dim).to_string(),
        body,
        value: rt.value.clone(),
        start,
        end,
        latent: rt.latent,
        rule: rt.node.rule_name.to_string(),
    }
}

fn dimension_name(dim: Dimension) -> &'static str {
    match dim {
        Dimension::Time => "time",
        Dimension::RegexMatch => "regex",
        Dimension::Numeral => "numeral",
    }
}

fn node_to_summary(node: &crate::Node) -> NodeSummary {
    NodeSummary {
        start: node.range.start,
        end: node.range.end,
        rule: node.rule_name.to_string(),
        preview: format_token_preview(&node.token.kind),
    }
}

fn format_token_preview(kind: &crate::TokenKind) -> String {
    let s = match kind {
        crate::TokenKind::TimeExpr(expr) => format!("{:?}", expr),
        crate::TokenKind::Numeral(n) => format!("({})", n.value),
        crate::TokenKind::RegexMatch(groups) => groups.first().cloned().unwrap_or_default(),
    };
    s.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    fn reference_context() -> Context {
        let date = NaiveDate::from_ymd_opt(2013, 2, 12).unwrap();
        let time = NaiveTime::from_hms_opt(4, 30, 0).unwrap();
        Context { reference_time: NaiveDateTime::new(date, time) }
    }

    #[test]
    fn parse_with_returns_entities() {
        let ctx = reference_context();
        let res = parse_with("today", &ctx, &Options::default());

        assert_eq!(res.text, "today");
        assert!(res.elapsed >= Duration::ZERO);

        let time = res.results.iter().find(|e| e.name == "time").unwrap();
        assert_eq!(time.body, "today");
        assert_eq!(time.start, 0);
        assert_eq!(time.end, 5);
        assert_eq!(time.value, "2013-02-12 00:00:00");
    }

    #[test]
    fn parse_verbose_includes_metrics_and_rules() {
        let ctx = reference_context();
        let res = parse_verbose_with("today", &ctx, &Options::default());

        assert_eq!(res.text, "today");
        assert_eq!(res.elapsed, res.details.total);
        assert!(res.details.saturation_total <= res.details.total);
        assert!(!res.details.active_rules.is_empty());
    }
}
