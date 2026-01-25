//! Weekend and week-related rules

use crate::engine::BucketMask;
use crate::rules::numeral::helpers::first_match_lower;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::predicates::*;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token};

/// "weekend", "this weekend"
pub fn rule_weekend() -> Rule {
    rule! {
        name: "week-end",
        pattern: [re!(r"(?i)(?:this|current)?\s*(week(\s|-)?end|wkend)s?")],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let week_start = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Week,
            };
            let friday_start = shift_by_grain(week_start.clone(), 4, Grain::Day);
            let weekend_start = shift_by_grain(friday_start, 18, Grain::Hour);
            let weekend_end = shift_by_grain(week_start, 7, Grain::Day);

            Some(TimeExpr::IntervalBetween {
                start: Box::new(weekend_start),
                end: Box::new(weekend_end),
            })
        }
    }
}

/// "past weekend", "last weekend"
pub fn rule_past_last_weekend() -> Rule {
    rule! {
        name: "(this) past/last weekend",
        pattern: [re!(r"(?i)(?:this\s+)?(?:past|last)\s*week(\s|-)?end")],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let week_start = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Week,
            };
            let friday_start = shift_by_grain(week_start.clone(), -3, Grain::Day);
            let weekend_start = shift_by_grain(friday_start, 18, Grain::Hour);
            let weekend_end = week_start;

            Some(TimeExpr::IntervalBetween {
                start: Box::new(weekend_start),
                end: Box::new(weekend_end),
            })
        }
    }
}

/// "last weekend of October", "last week-end in October"
pub fn rule_last_weekend_of_month() -> Rule {
    rule! {
        name: "last weekend of <month>",
        pattern: [
            re!(r"(?i)last\s*(?:week(\s|-)?end|wkend)\s+(?:of|in)\s+"),
            pred!(is_time_expr),
        ],
        buckets: (BucketMask::MONTHISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?;
            let (year, month) = match time_expr {
                TimeExpr::Absolute {
                    year,
                    month,
                    day: 1,
                    hour: None,
                    minute: None,
                } => (Some(*year), *month),
                TimeExpr::Intersect {
                    expr,
                    constraint: crate::time_expr::Constraint::Month(month),
                } if matches!(**expr, TimeExpr::Reference) => (None, *month),
                _ => return None,
            };

            let last_friday = TimeExpr::LastWeekdayOfMonth {
                year,
                month,
                weekday: chrono::Weekday::Fri,
            };
            let start = shift_by_grain(last_friday.clone(), 18, Grain::Hour);
            let end = shift_by_grain(last_friday, 3, Grain::Day);

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "all week", "rest of the week", "the week"
pub fn rule_week() -> Rule {
    rule! {
        name: "week",
        pattern: [re!(r"(?i)(all|rest of the|the) week")],
        required_phrases: ["week"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let week_start = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Week,
            };
            let week_end = shift_by_grain(week_start.clone(), 6, Grain::Day);

            let start = match matched.as_str() {
                "all week" => week_start,
                "rest of the week" | "the week" => TimeExpr::StartOf {
                    expr: Box::new(TimeExpr::Reference),
                    grain: Grain::Day,
                },
                _ => return None,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(week_end),
            })
        }
    }
}
