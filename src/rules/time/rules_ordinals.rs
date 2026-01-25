//! Ordinal date rules (ORDINALISH bucket)

use crate::engine::BucketMask;
use crate::rules::time::helpers::producers::year_from;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, TimeExpr};
use crate::{Rule, Token, TokenKind};

/// Ordinal day of month (e.g., "15th")
pub fn rule_ordinal_day_of_month() -> Rule {
    rule! {
        name: "ordinal (day of month)",
        pattern: [re!(r"(?i)\b([1-9]|[12]\d|3[01])(st|nd|rd|th)\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = regex_group_int_value(tokens.first()?, 1)? as u32;
            if !(1..=31).contains(&day) {
                return None;
            }
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfMonth(day),
            })
        }
    }
}

/// the <day-of-month> (ordinal)
pub fn rule_the_ordinal_day() -> Rule {
    rule! {
        name: "the <day-of-month> (ordinal)",
        pattern: [re!(r"(?i)(?:on\s+the|the)\s+"), pred!(is_day_of_month_numeral)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.get(1)?)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfMonth(day),
            })
        }
    }
}

/// <day-of-month>(ordinal) <named-month> year
pub fn rule_dom_ordinal_month_year() -> Rule {
    rule! {
        name: "<day-of-month>(ordinal) <named-month> year",
        pattern: [pred!(is_dom_ordinal), pred!(is_month), re!(r"(\d{2,4})")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::ORDINALISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = dom_value(tokens.first()?)? as u32;
            let month_match = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups.first()?.as_str(),
                _ => return None,
            };
            let month = MONTH_NAME.get(month_match.to_lowercase().as_str())?;
            let year_val = regex_group_int_value(tokens.get(2)?, 1)?;
            let year = year_from(year_val);

            Some(TimeExpr::Absolute { year, month: *month, day, hour: None, minute: None })
        }
    }
}
