//! Year references and time formatting rules (quarter to/past, half past, nth week)

use crate::engine::BucketMask;
use crate::rules::numeral::predicates::{is_integer, number_between};
use crate::rules::time::helpers::parse::time_expr_with_minutes;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};
// Already imported above

fn is_hour_numeral(token: &Token) -> bool {
    is_integer(token) && number_between::<1, 13>(token)
}

/// "this year", "next year", "last year"
pub fn rule_year_reference() -> Rule {
    rule! {
        name: "year reference",
        pattern: [re!(r"(?i)(this|current|last|previous|past|next)\s+(year|yr)\b")],
        required_phrases: ["this", "current", "last", "previous", "past", "next", "year", "yr"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let qualifier = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.trim(),
                _ => return None,
            };

            let amount = match qualifier {
                "this" | "current" => 0,
                "next" => 1,
                "last" | "previous" | "past" => -1,
                _ => return None,
            };

            let base = if amount == 0 {
                TimeExpr::Reference
            } else {
                shift_by_grain(TimeExpr::Reference, amount, Grain::Year)
            };

            Some(TimeExpr::StartOf {
                expr: Box::new(base),
                grain: Grain::Year,
            })
        }
    }
}

/// "quarter to|till|before <hour>"
pub fn rule_quarter_to_hod() -> Rule {
    rule! {
        name: "quarter to|till|before <hour-of-day>",
        pattern: [re!(r"(?i)(?:a|one)?\s*quarter\s+(?:to|till|before|of)\s+"), pred!(is_time_of_day_expr)],
        optional_phrases: ["quarter", "to", "till", "before", "of"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.get(1)?, -15) }
    }
}

/// "quarter after|past <hour>"
pub fn rule_quarter_after_hod() -> Rule {
    rule! {
        name: "quarter after|past <hour-of-day>",
        pattern: [re!(r"(?i)(?:for\s+)?(?:a|one)?\s*quarter\s+(?:after|past)\s+"), pred!(is_time_of_day_expr)],
        optional_phrases: ["quarter", "after", "past"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.get(1)?, 15) }
    }
}

/// "half after|past <hour>"
pub fn rule_half_after_hod() -> Rule {
    rule! {
        name: "half after|past <hour-of-day>",
        pattern: [re!(r"(?i)half (after|past)\s+"), pred!(is_time_of_day_expr)],
        optional_phrases: ["half", "after", "past"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.get(1)?, 30) }
    }
}

/// "half to|till|before <hour>"
pub fn rule_half_to_hod() -> Rule {
    rule! {
        name: "half to|till|before <hour-of-day>",
        pattern: [re!(r"(?i)half (to|till|before|of)\s+"), pred!(is_time_of_day_expr)],
        optional_phrases: ["half", "to", "till", "before", "of"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.get(1)?, -30) }
    }
}

/// "half <hour>" (e.g. "half three" -> 3:30)
pub fn rule_half_hod() -> Rule {
    rule! {
        name: "half <hour>",
        pattern: [re!(r"(?i)\bhalf\s+"), pred!(is_hour_numeral)],
        optional_phrases: ["half"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let mut hour = match &tokens.get(1)?.kind {
                TokenKind::Numeral(nd) => nd.value as i64,
                _ => return None,
            };

            // Duckling-style heuristic for ambiguous 12h phrasing in this ruleset:
            // treat "half three" as 15:30.
            if hour > 0 && hour < 12 {
                hour += 12;
            }

            time_expr_with_minutes(hour, 30, false)
        }
    }
}

/// "first/second/third/fourth/fifth week of <month> [year]"
pub fn rule_nth_week_of_month() -> Rule {
    rule! {
        name: "first/second/... week of <month> [year]",
        pattern: [
            re!(r"(?i)(first|second|third|fourth|fifth|1st|2nd|3rd|4th|5th)\s+week\s+of\s+"),
            pred!(is_month_expr),
            re!(r"(?i)\s*(\d{4})?")
        ],
        buckets: (BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal_str = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let n = match ordinal_str.as_str() {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };

            let month = month_from_expr(tokens.get(1)?)?;

            let year = if let Some(year_token) = tokens.get(2) {
                if let TokenKind::RegexMatch(groups) = &year_token.kind {
                    if let Some(year_str) = groups.get(1) {
                        if !year_str.is_empty() {
                            year_str.parse::<i32>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Some(TimeExpr::NthWeekOf {
                n,
                year,
                month: Some(month),
            })
        }
    }
}
