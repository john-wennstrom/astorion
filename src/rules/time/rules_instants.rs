use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::predicates::*;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token};

/// "now", "right now", "immediately", "at the moment", "atm", etc.
pub fn rule_instants_right_now() -> Rule {
    rule! {
        name: "right now",
        pattern: [re!(r"(?i)(?:((just|right)\s*)now|immediately|at\s+the\s+moment|at\s+this\s+moment|at\s+the\s+present\s+time|at\s+present|\batm\b)")],
        optional_phrases: ["now", "immediately", "moment", "atm"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::Reference)
        }
    }
}

/// "today", "todays"
pub fn rule_instants_today() -> Rule {
    rule! {
        name: "today",
        pattern: [re!(r"(?i)todays?|(at\s+this\s+time)")],
        optional_phrases: ["today", "this"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Day,
            })
        }
    }
}

/// "tomorrow", "tmrw", "tommorow"
pub fn rule_instants_tomorrow() -> Rule {
    rule! {
        name: "tomorrow",
        pattern: [re!(r"(?i)(tmrw?|tomm?or?rows?)")],
        optional_phrases: ["tomorrow", "tmrw", "tommorow", "tomorrows"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, 1, Grain::Day);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "day after tomorrow"
pub fn rule_day_after_tomorrow() -> Rule {
    rule! {
        name: "day after tomorrow",
        pattern: [re!(r"(?i)(the\s+)?day\s+after\s+tomorrow")],
        required_phrases: ["after", "tomorrow"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, 2, Grain::Day);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "<time-of-day> tomorrow"
pub fn rule_time_of_day_tomorrow() -> Rule {
    rule! {
        name: "<time-of-day> tomorrow",
        pattern: [pred!(is_time_of_day_expr), re!(r"\s+"), re!(r"(?i)(tmrw?|tomm?or?rows?)")],
        optional_phrases: ["tomorrow", "tmrw"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?.clone();
            let tomorrow = shift_by_grain(TimeExpr::Reference, 1, Grain::Day);
            let tomorrow_start = TimeExpr::StartOf {
                expr: Box::new(tomorrow),
                grain: Grain::Day,
            };

            // Intersect the time-of-day with tomorrow
            Some(TimeExpr::Intersect {
                expr: Box::new(tomorrow_start),
                constraint: match time_expr {
                    TimeExpr::Intersect { constraint, .. } => constraint,
                    _ => return None,
                },
            })
        }
    }
}

/// "yesterday"
pub fn rule_instants_yesterday() -> Rule {
    rule! {
        name: "yesterday",
        pattern: [re!(r"(?i)y(ester|ester|str)days?")],
        optional_phrases: ["yesterday", "ystrday", "yestrday"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, -1, Grain::Day);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "day before yesterday"
pub fn rule_day_before_yesterday() -> Rule {
    rule! {
        name: "day before yesterday",
        pattern: [re!(r"(?i)(the\s+)?day\s+before\s+yesterday")],
        required_phrases: ["before", "yesterday"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, -2, Grain::Day);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "now"
pub fn rule_now() -> Rule {
    rule! {
        name: "now",
        pattern: [re!(r"(?i)\bnow\b")],
        required_phrases: ["now"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::Reference)
        }
    }
}

/// "asap", "as soon as possible"
pub fn rule_asap() -> Rule {
    rule! {
        name: "asap",
        pattern: [re!(r"(?i)(asap|as\s+soon\s+as\s+possible)")],
        optional_phrases: ["asap", "soon"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::After(Box::new(
                TimeExpr::Reference,
            )))
        }
    }
}
