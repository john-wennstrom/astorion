//! Weekday and time modifiers (this/last/next weekday, around time, late last night, etc.)

use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, Grain, PartOfDay, TimeExpr};
use crate::{Rule, Token};

/// "this|next <day-of-week>" (this Monday, next Friday)
pub fn rule_next_dow() -> Rule {
    rule! {
        name: "this|next <day-of-week>",
        pattern: [re!(r"(?i)(this|next)\s+"), pred!(is_weekday_expr)],
        required_phrases: ["this", "next"],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let modifier = first(tokens)?.trim().to_lowercase();
            let weekday = weekday_from_expr(tokens.get(1)?)?;

            let amount = match modifier.as_str() {
                "this" => 0,
                "next" => 1,
                _ => return None,
            };

            let base_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };

            if amount == 0 {
                Some(base_expr)
            } else {
                Some(TimeExpr::Shift {
                    expr: Box::new(base_expr),
                    amount,
                    grain: Grain::Week,
                })
            }
        }
    }
}

/// "last <day-of-week>" (last Monday)
pub fn rule_last_dow() -> Rule {
    rule! {
        name: "last <day-of-week>",
        pattern: [re!(r"(?i)last\s+"), pred!(is_weekday_expr)],
        required_phrases: ["last"],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_expr(tokens.get(1)?)?;
            let base_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };
            Some(TimeExpr::Shift {
                expr: Box::new(base_expr),
                amount: -1,
                grain: Grain::Week,
            })
        }
    }
}

/// "this|current|coming <year|month|week|day>" (this year, current month, coming week)
pub fn rule_this_time() -> Rule {
    rule! {
        name: "this <time>",
        pattern: [re!(r"(?i)(this|current|coming)\s+"), re!(r"(?i)(year|quarter|month|week|day)\b")],
        optional_phrases: ["this", "current", "coming", "year", "quarter", "month", "week", "day"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let qualifier = first(tokens)?;
            let cycle = first(&tokens[1..])?;

            let grain = grain_from_cycle(cycle.trim())?;

            let amount = match qualifier.trim() {
                "this" | "current" => 0,
                "coming" => 1,
                _ => return None,
            };

            let base = if amount == 0 {
                TimeExpr::Reference
            } else {
                shift_by_grain(TimeExpr::Reference, amount, grain)
            };

            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(base),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(base),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "next <year|month|week|day>" (next year, next month)
pub fn rule_next_time() -> Rule {
    rule! {
        name: "next <time>",
        pattern: [re!(r"(?i)next\s+"), re!(r"(?i)(year|quarter|month|week|day)\b")],
        // Activate when we see "next"; the specific cycle word
        // (year/quarter/month/week/day) is enforced by the pattern itself.
        optional_phrases: ["next"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let _qualifier = first(tokens)?;
            let cycle = first(&tokens[1..])?;

            let grain = grain_from_cycle(cycle.trim())?;

            let base = shift_by_grain(TimeExpr::Reference, 1, grain);

            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(base),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(base),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "the following <week>" ~= "next week"
pub fn rule_following_week() -> Rule {
    rule! {
        name: "the following week",
        pattern: [re!(r"(?i)the\s+following\s+week")],
        optional_phrases: ["following", "week"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let base = shift_by_grain(TimeExpr::Reference, 1, Grain::Week);
            Some(TimeExpr::IntervalOf {
                expr: Box::new(base),
                grain: Grain::Week,
            })
        }
    }
}

/// "next <time>" (next Christmas, next July)
pub fn rule_next_time_expr() -> Rule {
    rule! {
        name: "next <time>",
        pattern: [re!(r"(?i)next\s+"), pred!(is_time_expr)],
        required_phrases: ["next"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?;

            // Don't match simple weekday expressions - those are handled by rule_next_dow
            if matches!(
                time_expr,
                TimeExpr::Intersect {
                    expr,
                    constraint: Constraint::DayOfWeek(_)
                } if matches!(**expr, TimeExpr::Reference)
            ) {
                return None;
            }

            // Don't match expressions with wrong structure: Intersect(Intersect(Reference, TimeOfDay), DayOfWeek)
            // This comes from <weekday> <time> rule which puts TimeOfDay first
            if matches!(
                time_expr,
                TimeExpr::Intersect {
                    expr,
                    constraint: Constraint::DayOfWeek(_)
                } if matches!(
                    **expr,
                    TimeExpr::Intersect {
                        expr: ref inner,
                        constraint: Constraint::TimeOfDay(_)
                    } if matches!(**inner, TimeExpr::Reference)
                )
            ) {
                return None;
            }

            // For supported date-like expressions, the base `time_expr` already
            // represents the next occurrence from the reference time
            // (e.g., Month / MonthDay pick the upcoming one). Adding an extra
            // shift would overshoot by a year, which breaks cases like
            // "next March" that are expected to resolve to the upcoming March.
            let supported = matches!(
                time_expr,
                TimeExpr::Intersect { constraint: Constraint::Month(_), .. }
                    | TimeExpr::MonthDay { .. }
            );

            if !supported {
                return None;
            }

            Some(time_expr.clone())
        }
    }
}

/// "this past|last|previous <year|month|week|day>" (last year, previous month)
pub fn rule_last_time() -> Rule {
    rule! {
        name: "last <time>",
        pattern: [re!(r"(?i)(this\s+past|last|previous|past)\s+"), re!(r"(?i)(year|quarter|month|week|day)\b")],
        // Activate when any of the qualifiers is present; the specific
        // cycle word (year/month/week/..) is matched by the pattern itself.
        optional_phrases: ["this", "past", "last", "previous", "week", "month", "year", "quarter"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let qualifier_raw = first(tokens)?;
            let qualifier = qualifier_raw.trim();
            let cycle = first(&tokens[1..])?;

            let grain = grain_from_cycle(cycle.trim())?;

            let amount = match qualifier {
                "last" | "previous" | "this past" | "past" => -1,
                _ => return None,
            };

            let base = shift_by_grain(TimeExpr::Reference, amount, grain);

            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(base),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(base),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "around <time>" (around 3pm, around tomorrow) - just passes through the time
pub fn rule_around_time() -> Rule {
    rule! {
        name: "around <time>",
        pattern: [re!(r"(?i)around\s+"), pred!(is_time_expr)],
        required_phrases: ["around"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // "around" is just a modifier that doesn't change the time
            let time_expr = get_time_expr(tokens.get(1)?)?;
            Some(time_expr.clone())
        }
    }
}

/// "<time> before last|after next" (Monday before last, Christmas after next)
pub fn rule_time_before_last_after_next() -> Rule {
    rule! {
        name: "<time> before last|after next",
        pattern: [pred!(is_time_expr), re!(r"(?i)\s+(before last|after next)")],
        optional_phrases: ["before last", "after next"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?;
            let qualifier = first(&tokens[1..])?.trim().to_lowercase();

            // Determine the grain based on the time expression
            let grain = match time_expr {
                TimeExpr::Intersect { constraint: Constraint::DayOfWeek(_), .. } => Grain::Week,
                TimeExpr::Intersect { constraint: Constraint::Month(_), .. } => Grain::Year,
                _ => Grain::Week, // default
            };

            // For weekdays, "after next" means 1 week after the next occurrence
            // For months, "after next" means 1 year after the next occurrence
            let amount = match qualifier.as_str() {
                "after next" => 1,
                "before last" => -1,
                _ => return None,
            };

            Some(TimeExpr::Shift {
                expr: Box::new(time_expr.clone()),
                amount,
                grain,
            })
        }
    }
}

/// "late last night" (21:00-00:00 yesterday)
pub fn rule_late_last_night() -> Rule {
    rule! {
        name: "late last night",
        pattern: [re!(r"(?i)late\s+last\s+night")],
        required_phrases: ["late", "last", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            // "late last night" = yesterday + late tonight period (21:00-00:00)
            let yesterday = TimeExpr::Shift {
                expr: Box::new(TimeExpr::Reference),
                amount: -1,
                grain: Grain::Day,
            };
            Some(TimeExpr::Intersect {
                expr: Box::new(yesterday),
                constraint: Constraint::PartOfDay(PartOfDay::LateTonight),
            })
        }
    }
}

/// "yesterday evening|night"
pub fn rule_yesterday_evening() -> Rule {
    rule! {
        name: "yesterday evening",
        pattern: [re!(r"(?i)yesterday\s+(evening|night)")],
        required_phrases: ["yesterday", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            // Same as "last night"
            let yesterday = TimeExpr::Shift {
                expr: Box::new(TimeExpr::Reference),
                amount: -1,
                grain: Grain::Day,
            };
            Some(TimeExpr::Intersect {
                expr: Box::new(yesterday),
                constraint: Constraint::PartOfDay(PartOfDay::Evening),
            })
        }
    }
}
