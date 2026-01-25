//! Phrase-gated time rules (required_phrases)

use crate::engine::BucketMask;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, TimeExpr};
use crate::{Rule, Token, TokenKind};

/// at <time-of-day>
pub fn rule_at_tod() -> Rule {
    rule! {
        name: "at <time-of-day>",
        pattern: [re!(r"(?i)(?:at|@)\s*"), pred!(is_time_of_day_expr)],
        optional_phrases: ["at", "@"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Extract the time-of-day expression from the second token
            let time_expr = match &tokens.get(1)?.kind {
                TokenKind::TimeExpr(expr) => expr.clone(),
                _ => return None,
            };

            // Return it as-is - the "at" is just a marker that doesn't change the expression
            Some(time_expr)
        }
    }
}

/// at <integer> - produces time-of-day from hour (1-24)
pub fn rule_at_integer_hour() -> Rule {
    use crate::rules::numeral::predicates::number_between;

    rule! {
        name: "at <integer>",
        pattern: [re!(r"(?i)(?:at|@)\s*"), pred!(|t: &Token| number_between::<0, 24>(t))],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = integer_value(tokens.get(1)?)? as u32;

            if hour > 24 {
                return None;
            }

            // Use hour as-is (0-24 format)
            let hour_24 = if hour == 24 { 0 } else { hour };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

/// at <hour> <minute>
pub fn rule_at_hour_minute() -> Rule {
    rule! {
        name: "at <hour> <minute>",
        pattern: [re!(r"(?i)at\s+(\d{1,2})\s+([0-5]\d)")],
        required_phrases: ["at"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)?;
            let minute = regex_group_int_value(tokens.first()?, 2)?;
            let adjusted_hour = if hour < 12 { hour + 12 } else { hour };
            time_expr_with_minutes(adjusted_hour, minute, false)
        }
    }
}

/// <month-day> at <time-of-day>
pub fn rule_month_day_at_tod() -> Rule {
    rule! {
        name: "<month-day> at <time-of-day>",
        pattern: [
            pred!(is_month_day_expr),
            re!(r"(?i)\s*(?:at|@)\s*"),
            pred!(is_time_of_day_expr),
        ],
        required_phrases: ["at", "@"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (month, day) = month_day_from_expr(tokens.first()?)?;
            let time = time_from_expr(tokens.get(2)?)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::MonthDay { month, day }),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

/// the ides of <named-month>
pub fn rule_ides_of_month() -> Rule {
    rule! {
        name: "the ides of <named-month>",
        pattern: [re!(r"(?i)the\s+ides?\s+of\s+"), pred!(is_month_expr)],
        optional_phrases: ["ides", "ide"],
        buckets: BucketMask::MONTHISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(1)?)?;
            // Ides are on the 15th for March, May, July, October; 13th for other months
            let day = if matches!(month, 3 | 5 | 7 | 10) { 15 } else { 13 };
            Some(TimeExpr::MonthDay { month, day })
        }
    }
}
