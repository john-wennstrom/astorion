use crate::time_expr::{Constraint, Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};

use crate::{
    engine::BucketMask,
    rules::time::{
        helpers::{shift::shift_by_grain, *},
        predicates::*,
    },
};

pub fn rule_intersect() -> Rule {
    rule! {
        name: "intersect",
        pattern: [pred!(is_time_expr), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let lhs = get_time_expr(tokens.first()?)?;
            let rhs = get_time_expr(tokens.get(1)?)?;

            intersect_time_exprs(lhs, rhs)
        }
    }
}

pub fn rule_intersect_of() -> Rule {
    rule! {
        name: "intersect by \",\", \"of\", \"from\", \"'s\"",
        pattern: [pred!(is_time_expr), re!(r"(?i)of|from|for|'s|,"), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let lhs = get_time_expr(tokens.first()?)?;
            let rhs = get_time_expr(tokens.get(2)?)?;

            intersect_time_exprs(lhs, rhs)
        }
    }
}

pub fn rule_weekday_from_time() -> Rule {
    rule! {
        name: "<weekday> from|of <time>",
        pattern: [
            pred!(is_weekday_expr),
            re!(r"(?i)\s+(?:from|of)\s+"),
            pred!(is_time_expr),
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_expr(tokens.first()?)?;
            let time_expr = get_time_expr(tokens.get(2)?)?.clone();
            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

pub fn rule_time_possessive_weekday() -> Rule {
    rule! {
        name: "<time>'s <weekday>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)'s\s+"),
            pred!(is_weekday_expr),
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?.clone();
            let weekday = weekday_from_expr(tokens.get(2)?)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

pub fn rule_weekday_in_time_expr() -> Rule {
    rule! {
        name: "<weekday> <time>",
        pattern: [pred!(is_weekday_expr), re!(r"\s+"), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(2)?)?.clone();
            let weekday = weekday_from_expr(tokens.first()?)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

pub fn rule_intersect_year() -> Rule {
    rule! {
        name: "intersect by \",\", \"of\", \"from\" for year",
        pattern: [pred!(is_time_expr), re!(r"(?i)of|from|,"), re!(r"(\d{4})")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base = get_time_expr(tokens.first()?)?.clone();
            let year = regex_group_int_value(tokens.get(2)?, 1)? as i32;

            match base {
                TimeExpr::MonthDay { month, day } => Some(TimeExpr::Absolute {
                    year,
                    month,
                    day,
                    hour: None,
                    minute: None,
                }),
                TimeExpr::Intersect { constraint: Constraint::Month(month), expr } if matches!(*expr, TimeExpr::Reference) => {
                    Some(TimeExpr::Absolute {
                        year,
                        month,
                        day: 1,
                        hour: None,
                        minute: None,
                    })
                }
                _ => None,
            }
        }
    }
}

pub fn rule_absorb_on_day() -> Rule {
    rule! {
        name: "on <day>",
        pattern: [re!(r"(?i)on"), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            get_time_expr(tokens.get(1)?).cloned()
        }
    }
}

pub fn rule_absorb_on_a_dow() -> Rule {
    rule! {
        name: "on a <named-day>",
        pattern: [re!(r"(?i)on\s+a"), pred!(is_weekday_expr)],
        buckets: (BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            get_time_expr(tokens.get(1)?).cloned()
        }
    }
}

pub fn rule_absorb_in_month_year() -> Rule {
    rule! {
        name: "in|during <named-month>|year",
        pattern: [re!(r"(?i)in|during"), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let expr = get_time_expr(tokens.get(1)?)?;

            match expr {
                TimeExpr::Intersect { constraint: Constraint::Month(_), .. }
                | TimeExpr::StartOf { grain: Grain::Year, .. }
                | TimeExpr::IntervalOf { grain: Grain::Year, .. } => {
                    Some(expr.clone())
                }
                _ => None,
            }
        }
    }
}

pub fn rule_absorb_comma_tod() -> Rule {
    rule! {
        name: "absorption of , after named day",
        pattern: [pred!(is_weekday_expr), re!(r",")],
        buckets: (BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            get_time_expr(tokens.first()?).cloned()
        }
    }
}

pub fn rule_in_duration_at_time() -> Rule {
    rule! {
        name: "in <duration> at <time-of-day> (post-process intersect)",
        pattern: [pred!(|t: &Token| {
            // Match Intersect tokens where the expr is a Shift with Year or Month grain
            match &t.kind {
                TokenKind::TimeExpr(TimeExpr::Intersect { expr, constraint }) => {
                    matches!(&**expr, TimeExpr::Shift { grain: Grain::Year | Grain::Month | Grain::Week | Grain::Day, amount, .. } if *amount > 0)
                    && matches!(constraint, Constraint::TimeOfDay(_))
                }
                _ => false,
            }
        })],
        buckets: (BucketMask::HAS_COLON | BucketMask::HAS_AMPM | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Extract the existing Intersect
            let (shift_expr, time_constraint) = match &tokens.first()?.kind {
                TokenKind::TimeExpr(TimeExpr::Intersect { expr, constraint }) => (expr.as_ref(), constraint),
                _ => return None,
            };

            // Extract amount and grain from the Shift
            let (amount, grain) = match shift_expr {
                TimeExpr::Shift { amount, grain, .. } => (*amount, *grain),
                _ => return None,
            };

            // Extract time from constraint
            let time = match time_constraint {
                Constraint::TimeOfDay(t) => *t,
                _ => return None,
            };

            // Rebuild with proper base for Month/Year grains
            let base = match grain {
                Grain::Month | Grain::Year => TimeExpr::StartOf {
                    expr: Box::new(TimeExpr::Reference),
                    grain: Grain::Month,
                },
                Grain::Week | Grain::Day => TimeExpr::StartOf {
                    expr: Box::new(TimeExpr::Reference),
                    grain: Grain::Day,
                },
                _ => TimeExpr::Reference,
            };
            let shifted = shift_by_grain(base, amount, grain);
            Some(TimeExpr::Intersect {
                expr: Box::new(shifted),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_time_of_time() -> Rule {
    rule! {
        name: "<time> of <time>",
        pattern: [pred!(is_time_expr), re!(r"(?i)\s+of\s+"), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let lhs = get_time_expr(tokens.first()?)?;
            let rhs = get_time_expr(tokens.get(2)?)?;
            intersect_time_exprs(lhs, rhs)
        }
    }
}
