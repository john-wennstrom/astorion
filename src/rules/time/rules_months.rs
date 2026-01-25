//! Month-based time rules (MONTHISH bucket)

use crate::TokenKind;
use crate::engine::BucketMask;
use crate::rules::time::helpers::producers::year_from;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::{Dimension, Rule, TimeExpr, Token};

/// <month> <year>
pub fn rule_month_year() -> Rule {
    rule! {
        name: "<month> <year>",
        pattern: [pred!(is_month_expr), re!(r"\s+(\d{4})")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.first()?)?;
            let year = regex_group_int_value(tokens.get(1)?, 1)? as i32;

            Some(TimeExpr::Absolute { year, month, day: 1, hour: None, minute: None })
        }
    }
}

/// <month> <day-of-month> (ordinal)
pub fn rule_month_ordinal_day() -> Rule {
    rule! {
        name: "<month> <day-of-month> (ordinal)",
        pattern: [pred!(is_month_expr), re!(r"\s+"), pred!(is_day_of_month_numeral)],
        buckets: (BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        deps: [Dimension::Numeral],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.first()?)?;
            let day = day_of_month_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

/// dd/month/yyyy (e.g., "25/December/2024")
pub fn rule_dd_slash_month_slash_yyyy() -> Rule {
    rule! {
        name: "dd/month/yyyy",
        pattern: [
            re!(r"([1-9]|[12]\d|3[01])/"),
            pred!(is_month_expr),
            re!(r"/(\d{2,4})")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = regex_group_int_value(tokens.first()?, 1)? as u32;
            if !(1..=31).contains(&day) {
                return None;
            }

            let month = month_from_expr(tokens.get(1)?)?;

            let year_val = regex_group_int_value(tokens.get(2)?, 1)?;
            let year = year_from(year_val);

            Some(TimeExpr::Absolute { year, month, day, hour: None, minute: None })
        }
    }
}

/// dd-month-yy (e.g., "25-December-24")
pub fn rule_dd_dash_month_dash_yy() -> Rule {
    rule! {
        name: "dd-month-yy",
        pattern: [
            re!(r"([1-9]|[12]\d|3[01])-"),
            pred!(is_month_expr),
            re!(r"-(\d{2,4})")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = regex_group_int_value(tokens.first()?, 1)? as u32;
            if !(1..=31).contains(&day) {
                return None;
            }

            let month = month_from_expr(tokens.get(1)?)?;

            let year_val = regex_group_int_value(tokens.get(2)?, 1)?;
            let year = year_from(year_val);

            Some(TimeExpr::Absolute { year, month, day, hour: None, minute: None })
        }
    }
}

/// <day>/<named-month>/<year> numeric
pub fn rule_dom_month_name_year_numeric() -> Rule {
    rule! {
        name: "<day>/<named-month>/<year> numeric",
        pattern: [
            re!(r"(?i)(\d{1,2})\s*[/\-]\s*(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)\s*[/\-]\s*(\d{2,4})")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = regex_group_int_value(tokens.first()?, 1)? as u32;
            let month_match = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(2)?.as_str(),
                _ => return None,
            };
            let month = MONTH_NAME.get(month_match.to_lowercase().as_str())?;
            let year_val = regex_group_int_value(tokens.first()?, 3)?;

            let year = year_from(year_val);

            if day == 0 || day > 31 {
                return None;
            }

            Some(TimeExpr::Absolute {
                year,
                month: *month,
                day,
                hour: None,
                minute: None,
            })
        }
    }
}

/// <month-day>, <year>
pub fn rule_month_day_comma_year() -> Rule {
    rule! {
        name: "<month-day>, <year>",
        pattern: [pred!(is_month_day_expr), re!(r",\s*(\d{2,4})")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (month, day) = month_day_from_expr(tokens.first()?)?;
            let year_val = regex_group_int_value(tokens.get(1)?, 1)?;
            let year = year_from(year_val);
            Some(TimeExpr::Absolute { year, month, day, hour: None, minute: None })
        }
    }
}
