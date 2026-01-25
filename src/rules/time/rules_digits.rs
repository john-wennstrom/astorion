//! Time rules requiring digits (HAS_DIGITS bucket)

use crate::engine::BucketMask;
use crate::rules::time::helpers::producers::year_from;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, TimeExpr};
use crate::{Rule, Token};

/// yyyy-mm-dd format
pub fn rule_yyyy_mm_dd() -> Rule {
    rule! {
        name: "yyyy-mm-dd",
        pattern: [re!(r"(\d{2,4})-(0?[1-9]|1[0-2])-(3[01]|[12]\d|0?[1-9])")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year_val = regex_group_int_value(tokens.first()?, 1)?;
            let month = regex_group_int_value(tokens.first()?, 2)? as u32;
            let day = regex_group_int_value(tokens.first()?, 3)? as u32;

            let year = year_from(year_val);

            Some(TimeExpr::Absolute { year, month, day, hour: None, minute: None })
        },
    }
}

/// yyyy year-only format (e.g., "1974")
pub fn rule_yyyy() -> Rule {
    rule! {
        name: "yyyy (year-only)",
        pattern: [re!(r"\b(\d{4})\b")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year = regex_group_int_value(tokens.first()?, 1)? as i32;

            Some(TimeExpr::Absolute {
                year,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            })
        },
    }
}

/// yyyy-mm or yyyy/mm format
pub fn rule_yyyy_mm() -> Rule {
    rule! {
        name: "yyyy-mm or yyyy/mm",
        pattern: [re!(r"(\d{4})[-/](1[0-2]|0?[1-9])")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year = regex_group_int_value(tokens.first()?, 1)? as i32;
            let month = regex_group_int_value(tokens.first()?, 2)? as u32;

            Some(TimeExpr::Absolute { year, month, day: 1, hour: None, minute: None })
        }
    }
}

/// yyyyqq format (e.g., 2024q1)
pub fn rule_yyyy_qq() -> Rule {
    rule! {
        name: "yyyyqq",
        pattern: [re!(r"(?i)(\d{2,4})q([1-4])")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year_val = regex_group_int_value(tokens.first()?, 1)?;
            let quarter = regex_group_int_value(tokens.first()?, 2)? as u32;
            let year = year_from(year_val);

            let start_month = (quarter - 1) * 3 + 1;
            Some(TimeExpr::Absolute {
                year,
                month: start_month,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}

/// mm/yyyy format
pub fn rule_mm_yyyy() -> Rule {
    rule! {
        name: "mm/yyyy",
        pattern: [re!(r"(?i)(0?[1-9]|1[0-2])[/-](\d{4})")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = regex_group_int_value(tokens.first()?, 1)? as u32;
            let year = regex_group_int_value(tokens.first()?, 2)? as i32;

            Some(TimeExpr::Absolute { year, month, day: 1, hour: None, minute: None })
        }
    }
}

/// month/day numeric (e.g., 12/25)
pub fn rule_month_day_numeric() -> Rule {
    rule! {
        name: "month/day numeric",
        pattern: [
            re!(r"(?i)(?:on\s+)?(\d{1,2})\s*[/-]\s*(\d{1,2})")
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = regex_group_int_value(tokens.first()?, 1)? as u32;
            let day = regex_group_int_value(tokens.first()?, 2)? as u32;

            // Validate ranges
            if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
                return None;
            }

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

/// month/day/year numeric (e.g., 12/25/2024)
pub fn rule_month_day_year_numeric() -> Rule {
    rule! {
        name: "month/day/year numeric",
        pattern: [
            re!(r"(?i)(?:on\s+)?(\d{1,2})\s*[/\-.]\s*(\d{1,2})\s*[/\-.]\s*(\d{2,4})")
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = regex_group_int_value(tokens.first()?, 1)? as u32;
            let day = regex_group_int_value(tokens.first()?, 2)? as u32;
            let year_val = regex_group_int_value(tokens.first()?, 3)?;

            let year = year_from(year_val);

            // Validate ranges
            if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
                return None;
            }

            Some(TimeExpr::Absolute {
                year,
                month,
                day,
                hour: None,
                minute: None,
            })
        }
    }
}

/// Integer day of month (e.g., "15")
pub fn rule_integer_day_of_month() -> Rule {
    rule! {
        name: "integer (day of month)",
        pattern: [re!(r"\b([1-9]|[12]\d|3[01])\b")],
        buckets: BucketMask::HAS_DIGITS.bits(),
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

/// <month-day> year
pub fn rule_month_day_year() -> Rule {
    rule! {
        name: "<month-day> year",
        pattern: [pred!(is_month_day_expr), re!(r"\s+(\d{2,4})")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (month, day) = month_day_from_expr(tokens.first()?)?;
            let year_val = regex_group_int_value(tokens.get(1)?, 1)?;
            let year = year_from(year_val);

            Some(TimeExpr::Absolute {
                year,
                month,
                day,
                hour: None,
                minute: None,
            })
        }
    }
}

/// Year AD (e.g., "2024 AD")
pub fn rule_year_ad() -> Rule {
    rule! {
        name: "in <year> AD",
        pattern: [re!(r"(?i)(in\s+)?(\d{1,4})\s+(a\.?d\.?)")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year = regex_group_int_value(tokens.first()?, 2)? as i32;

            Some(TimeExpr::Absolute {
                year,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}

/// <time> at <time-of-day>
pub fn rule_time_expr_at_tod() -> Rule {
    rule! {
        name: "<time> at <time-of-day>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s+(?:at\s+)?"),
            pred!(is_time_of_day_expr),
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::HAS_AMPM).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let date_expr = get_time_expr(tokens.first()?)?.clone();
            if matches!(date_expr, TimeExpr::Intersect { constraint: Constraint::TimeOfDay(_), .. }) {
                return None;
            }
            let time = time_from_expr(tokens.get(2)?)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}
