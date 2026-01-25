//! Month and month-part related rules

use crate::engine::BucketMask;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Grain, MonthPart, TimeExpr};
use crate::{Rule, Token};

/// "early March", "mid-March", "late of March"
pub fn rule_part_of_month() -> Rule {
    rule! {
        name: "part of <named-month>",
        pattern: [
            re!(r"(?i)(early|mid|late)-?(?:\s+of)?\s+"),
            pred!(is_month_expr),
        ],
        optional_phrases: ["early", "mid", "late"],
        buckets: BucketMask::MONTHISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(tokens)?;

            let part = if matched.contains("early") {
                MonthPart::Early
            } else if matched.contains("mid") {
                MonthPart::Mid
            } else {
                MonthPart::Late
            };

            let month = month_from_expr(tokens.get(1)?)?;

            Some(TimeExpr::MonthPart { month: Some(month), part })
        }
    }
}

/// "beginning of January", "at the end of April"
pub fn rule_end_or_beginning_of_month() -> Rule {
    rule! {
        name: "at the beginning|end of <named-month>",
        pattern: [
            re!(r"(?i)(at the )?(beginning|end) of\s+"),
            pred!(is_month_expr),
        ],
        optional_phrases: ["beginning", "end"],
        buckets: BucketMask::MONTHISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(tokens)?;

            let part = if matched.contains("beginning") {
                MonthPart::Early
            } else {
                MonthPart::Late
            };

            let month = month_from_expr(tokens.get(1)?)?;

            Some(TimeExpr::MonthPart { month: Some(month), part })
        }
    }
}

/// "EOM", "end of month", "by EOM"
pub fn rule_end_of_month() -> Rule {
    rule! {
        name: "end of month",
        pattern: [re!(r"(?i)(by (the )?|(at )?the )?(EOM|end of (the )?month)")],
        optional_phrases: ["eom", "month"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(tokens)?;
            let is_by_eom = matched.to_lowercase().starts_with("by");

            if is_by_eom {
                let current_month = TimeExpr::IntervalOf {
                    expr: Box::new(TimeExpr::Reference),
                    grain: Grain::Month,
                };
                let next_month = TimeExpr::Shift {
                    expr: Box::new(current_month),
                    amount: 1,
                    grain: Grain::Month,
                };
                let end_of_month = TimeExpr::StartOf {
                    expr: Box::new(next_month),
                    grain: Grain::Month,
                };
                Some(TimeExpr::IntervalUntil {
                    target: Box::new(end_of_month),
                })
            } else {
                Some(TimeExpr::MonthPart {
                    month: None,
                    part: MonthPart::Late,
                })
            }
        }
    }
}

/// "BOM", "beginning of month"
pub fn rule_beginning_of_month() -> Rule {
    rule! {
        name: "beginning of month",
        pattern: [re!(r"(?i)((at )?the )?(BOM|beginning of (the )?month)")],
        optional_phrases: ["bom", "month"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let start_of_month = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Month,
            };

            let day_11 = TimeExpr::Shift {
                expr: Box::new(start_of_month.clone()),
                amount: 10,
                grain: Grain::Day,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_of_month),
                end: Box::new(day_11),
            })
        }
    }
}

/// "by (the) end of <time>"
pub fn rule_by_end_of_time() -> Rule {
    rule! {
        name: "by (the) end of <time>",
        pattern: [
            re!(r"(?i)by\s+(the\s+)?end\s+of\s+"),
            pred!(is_time_expr),
        ],
        required_phrases: ["by", "end"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?.clone();

            let grain = container_grain_for_expr(&time_expr);

            let end_of_period = TimeExpr::Shift {
                expr: Box::new(time_expr),
                amount: 1,
                grain,
            };

            let start_of_next = TimeExpr::StartOf {
                expr: Box::new(end_of_period),
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(TimeExpr::Reference),
                end: Box::new(start_of_next),
            })
        }
    }
}

/// "beginning of this week", "at the beginning of next week"
pub fn rule_beginning_of_week() -> Rule {
    rule! {
        name: "beginning of <week>",
        pattern: [re!(r"(?i)((at )?the )?beginning of\s+(around\s+)?"), pred!(is_time_expr)],
        required_phrases: ["beginning"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?.clone();

            // Only applies to week cycles like "this week" / "next week".
            if !matches!(time_expr, TimeExpr::IntervalOf { grain: Grain::Week, .. }) {
                return None;
            }

            let start_of_week = TimeExpr::StartOf {
                expr: Box::new(time_expr),
                grain: Grain::Week,
            };

            // First 3 days of the week: Mon 00:00 -> Thu 00:00.
            let end = TimeExpr::Shift {
                expr: Box::new(start_of_week.clone()),
                amount: 3,
                grain: Grain::Day,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_of_week),
                end: Box::new(end),
            })
        }
    }
}

/// "end of this week", "at the end of coming week"
pub fn rule_end_of_week() -> Rule {
    rule! {
        name: "end of <week>",
        pattern: [re!(r"(?i)((at )?the )?end of\s+(around\s+)?"), pred!(is_time_expr)],
        required_phrases: ["end"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?.clone();

            // Only applies to week cycles like "this week" / "next week".
            if !matches!(time_expr, TimeExpr::IntervalOf { grain: Grain::Week, .. }) {
                return None;
            }

            let start_of_week = TimeExpr::StartOf {
                expr: Box::new(time_expr),
                grain: Grain::Week,
            };

            // Last 3 days of the week: Fri 00:00 -> Mon 00:00.
            let start = TimeExpr::Shift {
                expr: Box::new(start_of_week.clone()),
                amount: 4,
                grain: Grain::Day,
            };
            let end = TimeExpr::Shift {
                expr: Box::new(start_of_week),
                amount: 1,
                grain: Grain::Week,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "end of year", "EOY"
pub fn rule_end_of_year() -> Rule {
    rule! {
        name: "end of year",
        pattern: [re!(r"(?i)((at )?the )?(EOY|end of (the )?year)")],
        optional_phrases: ["eoy", "end", "year"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let start_of_year = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            let start_of_eoy = TimeExpr::Shift {
                expr: Box::new(start_of_year.clone()),
                amount: 8,
                grain: Grain::Month,
            };
            let end_of_year = TimeExpr::Shift {
                expr: Box::new(start_of_year),
                amount: 1,
                grain: Grain::Year,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_of_eoy),
                end: Box::new(end_of_year),
            })
        }
    }
}

/// "end of 2012"
pub fn rule_end_of_specific_year() -> Rule {
    rule! {
        name: "end of <year>",
        pattern: [re!(r"(?i)(at the )?end of\s+"), pred!(is_time_expr)],
        required_phrases: ["end"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year_expr = get_time_expr(tokens.get(1)?)?;
            let year = match year_expr {
                TimeExpr::Absolute {
                    year,
                    month: 1,
                    day: 1,
                    hour: None,
                    minute: None,
                } => *year,
                _ => return None,
            };

            let start = TimeExpr::Absolute {
                year,
                month: 9,
                day: 1,
                hour: None,
                minute: None,
            };
            let end = TimeExpr::Absolute {
                year: year + 1,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "beginning of 2017"
pub fn rule_beginning_of_specific_year() -> Rule {
    rule! {
        name: "beginning of <year>",
        pattern: [re!(r"(?i)(at the )?beginning of\s+"), pred!(is_time_expr)],
        required_phrases: ["beginning"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year_expr = get_time_expr(tokens.get(1)?)?;
            let year = match year_expr {
                TimeExpr::Absolute {
                    year,
                    month: 1,
                    day: 1,
                    hour: None,
                    minute: None,
                } => *year,
                _ => return None,
            };

            let start = TimeExpr::Absolute {
                year,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            };
            let end = TimeExpr::Absolute {
                year,
                month: 4,
                day: 1,
                hour: None,
                minute: None,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "beginning of year", "BOY"
pub fn rule_beginning_of_year() -> Rule {
    rule! {
        name: "beginning of year",
        pattern: [re!(r"(?i)((at )?the )?(BOY|beginning of (the )?year)")],
        optional_phrases: ["boy", "beginning", "year"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let start_of_year = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };

            // Corpus expects "beginning of year" as the first quarter.
            let start_of_q2 = TimeExpr::Shift {
                expr: Box::new(start_of_year.clone()),
                amount: 3,
                grain: Grain::Month,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_of_year),
                end: Box::new(start_of_q2),
            })
        }
    }
}
