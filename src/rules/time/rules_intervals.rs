use crate::Dimension;
/// Interval-based rules (from/to, between, dash ranges)
use crate::engine::BucketMask;
use crate::rules::time::predicates::*;
use crate::time_expr::Grain;
use crate::time_expr::TimeExpr;
use crate::{Rule, Token, TokenKind};
use chrono::Timelike;

fn time_of_day_time(expr: &TimeExpr) -> Option<chrono::NaiveTime> {
    let mut current = expr;
    loop {
        match current {
            TimeExpr::Intersect { constraint: crate::time_expr::Constraint::TimeOfDay(tod), .. } => return Some(*tod),
            TimeExpr::Shift { expr, amount, .. } if *amount == 0 => {
                current = expr;
            }
            _ => return None,
        }
    }
}

fn replace_time_of_day(expr: &TimeExpr, new_tod: chrono::NaiveTime) -> Option<TimeExpr> {
    match expr {
        TimeExpr::Shift { expr: inner, amount, grain } if *amount == 0 => {
            let mapped = replace_time_of_day(inner, new_tod)?;
            Some(TimeExpr::Shift { expr: Box::new(mapped), amount: *amount, grain: *grain })
        }
        TimeExpr::Intersect { expr, constraint: crate::time_expr::Constraint::TimeOfDay(_) } => {
            Some(TimeExpr::Intersect {
                expr: expr.clone(),
                constraint: crate::time_expr::Constraint::TimeOfDay(new_tod),
            })
        }
        TimeExpr::Intersect { .. } => None,
        _ => None,
    }
}

fn maybe_disambiguate_end_time_of_day(start: &TimeExpr, end: TimeExpr) -> TimeExpr {
    let Some(start_tod) = time_of_day_time(start) else {
        return end;
    };
    let Some(end_tod) = time_of_day_time(&end) else {
        return end;
    };

    if end_tod > start_tod {
        return end;
    }

    // Heuristic for inputs like "8am until 6": if the end is a bare hour
    // (no minutes/seconds) and would be before the start, interpret it as PM.
    if end_tod.minute() == 0 && end_tod.second() == 0 && end_tod.hour() < 12 {
        let new_hour = end_tod.hour() + 12;
        if let Some(new_tod) = chrono::NaiveTime::from_hms_opt(new_hour, 0, 0) {
            if new_tod > start_tod {
                if let Some(mapped) = replace_time_of_day(&end, new_tod) {
                    return mapped;
                }
            }
        }
    }

    end
}

fn finest_precision(a: Grain, b: Grain) -> Grain {
    match (a, b) {
        (Grain::Second, _) | (_, Grain::Second) => Grain::Second,
        (Grain::Minute, _) | (_, Grain::Minute) => Grain::Minute,
        _ => Grain::Hour,
    }
}

fn time_of_day_precision(expr: &TimeExpr) -> Option<Grain> {
    let mut current = expr;
    let mut forced_precision: Option<Grain> = None;
    loop {
        match current {
            TimeExpr::Intersect { constraint: crate::time_expr::Constraint::TimeOfDay(tod), .. } => {
                return Some(match forced_precision {
                    Some(Grain::Second) => Grain::Second,
                    Some(Grain::Hour) => Grain::Hour,
                    Some(Grain::Minute) => Grain::Minute,
                    _ => {
                        if tod.second() != 0 {
                            Grain::Second
                        } else {
                            Grain::Minute
                        }
                    }
                });
            }
            TimeExpr::Shift { expr, amount, grain } => {
                if *amount == 0 {
                    forced_precision = Some(*grain);
                }
                current = expr;
            }
            _ => return None,
        }
    }
}

fn end_exclusive_grain(start: &TimeExpr, end: &TimeExpr) -> Option<Grain> {
    let end_precision = time_of_day_precision(end)?;
    let start_precision = time_of_day_precision(start);
    Some(match start_precision {
        Some(sp) => finest_precision(sp, end_precision),
        None => end_precision,
    })
}

/// "from <time> to <time>"
pub fn rule_interval_from_to() -> Rule {
    rule! {
        name: "from <time> to <time>",
        pattern: [
            re!(r"(?i)from\s+"),
            pred!(is_time_expr),
            re!(r"\s+to\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["from", "to"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.get(1)?)?.clone();
            let end_token = tokens.get(3)?;
            let end = get_time_expr(end_token)?.clone();

            let end = if let Some(grain) = end_exclusive_grain(&start, &end) {
                TimeExpr::Shift {
                    expr: Box::new(end),
                    amount: 1,
                    grain,
                }
            } else {
                end
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "from <time>"
pub fn rule_interval_from_open() -> Rule {
    rule! {
        name: "from <time>",
        pattern: [
            re!(r"(?i)from\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["from"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.get(1)?)?.clone();
            Some(TimeExpr::After(Box::new(start)))
        }
    }
}

/// "between <time> and <time>"
pub fn rule_interval_between_and() -> Rule {
    rule! {
        name: "between <time> and <time>",
        pattern: [
            re!(r"(?i)between\s+"),
            pred!(is_time_expr),
            re!(r"\s+and\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["between"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.get(1)?)?.clone();
            let end_token = tokens.get(3)?;
            let end = get_time_expr(end_token)?.clone();

            let end = if let Some(grain) = end_exclusive_grain(&start, &end) {
                TimeExpr::Shift {
                    expr: Box::new(end),
                    amount: 1,
                    grain,
                }
            } else {
                end
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "<time> - <time>" (dash interval)
pub fn rule_interval_dash() -> Rule {
    rule! {
        name: "<time> - <time>",
        pattern: [
            pred!(is_time_expr),
            re!(r"\s*-\s*"),
            pred!(is_time_expr)
        ],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.first()?)?.clone();
            let end_token = tokens.get(2)?;
            let end = get_time_expr(end_token)?.clone();

            // Duckling-style semantics: treat end as inclusive at the token's
            // resolution (minute or second), and convert to an end-exclusive
            // interval bound by shifting by one unit.
            let end = if let Some(grain) = end_exclusive_grain(&start, &end) {
                TimeExpr::Shift {
                    expr: Box::new(end),
                    amount: 1,
                    grain,
                }
            } else {
                end
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "<time-of-day>-<time-of-day> <date>" (e.g. "1pm-2pm tomorrow")
pub fn rule_interval_dash_on_date() -> Rule {
    fn reapply_zero_shifts(original: &TimeExpr, inner: TimeExpr) -> TimeExpr {
        match original {
            TimeExpr::Shift { expr, amount, grain } if *amount == 0 => {
                TimeExpr::Shift { expr: Box::new(reapply_zero_shifts(expr, inner)), amount: *amount, grain: *grain }
            }
            _ => inner,
        }
    }

    rule! {
        name: "<time-of-day> - <time-of-day> <date>",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"\s*-\s*"),
            pred!(is_time_of_day_expr),
            re!(r"\s+"),
            pred!(is_future_shift_expr)
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let date_expr = get_time_expr(tokens.get(4)?)?.clone();

            let start_time = time_from_expr(tokens.first()?)?;
            let end_time = time_from_expr(tokens.get(2)?)?;

            let start_inner = TimeExpr::Intersect {
                expr: Box::new(date_expr.clone()),
                constraint: crate::time_expr::Constraint::TimeOfDay(start_time),
            };
            let start = match &tokens.first()?.kind {
                TokenKind::TimeExpr(expr) => reapply_zero_shifts(expr, start_inner),
                _ => start_inner,
            };

            let end_inner = TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: crate::time_expr::Constraint::TimeOfDay(end_time),
            };
            let mut end = match &tokens.get(2)?.kind {
                TokenKind::TimeExpr(expr) => reapply_zero_shifts(expr, end_inner),
                _ => end_inner,
            };

            if let Some(grain) = end_exclusive_grain(&start, &end) {
                end = TimeExpr::Shift {
                    expr: Box::new(end),
                    amount: 1,
                    grain,
                };
            }

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "<time> through <time>", "<time> thru <time>"
pub fn rule_interval_through() -> Rule {
    rule! {
        name: "<time> through <time>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s+(through|thru)\s+"),
            pred!(is_time_expr)
        ],
        optional_phrases: ["through", "thru"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.first()?)?.clone();
            let end_token = tokens.get(2)?;
            let end = maybe_disambiguate_end_time_of_day(&start, get_time_expr(end_token)?.clone());

            let end = if let Some(grain) = end_exclusive_grain(&start, &end) {
                TimeExpr::Shift {
                    expr: Box::new(end),
                    amount: 1,
                    grain,
                }
            } else {
                end
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "through <time>", "thru <time>"
pub fn rule_interval_through_open() -> Rule {
    rule! {
        name: "through <time>",
        pattern: [
            re!(r"(?i)(through|thru)\s+"),
            pred!(is_time_expr)
        ],
        optional_phrases: ["through", "thru"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let end = get_time_expr(tokens.get(1)?)?.clone();
            Some(TimeExpr::Before(Box::new(end)))
        }
    }
}

/// "<time> until <time>"
pub fn rule_interval_until() -> Rule {
    rule! {
        name: "<time> until <time>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s+until\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["until"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.first()?)?.clone();
            let end_token = tokens.get(2)?;
            let end = maybe_disambiguate_end_time_of_day(&start, get_time_expr(end_token)?.clone());

            let end = if let Some(grain) = end_exclusive_grain(&start, &end) {
                TimeExpr::Shift {
                    expr: Box::new(end),
                    amount: 1,
                    grain,
                }
            } else {
                end
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

/// "until <time>"
pub fn rule_interval_until_open() -> Rule {
    rule! {
        name: "until <time>",
        pattern: [
            re!(r"(?i)until\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["until"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let end = get_time_expr(tokens.get(1)?)?.clone();
            Some(TimeExpr::Before(Box::new(end)))
        }
    }
}

/// "before <time>"
pub fn rule_interval_before() -> Rule {
    rule! {
        name: "before <time>",
        pattern: [
            re!(r"(?i)before\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["before"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let end = get_time_expr(tokens.get(1)?)?.clone();

            Some(TimeExpr::Before(Box::new(end)))
        }
    }
}

/// "after <time>"
pub fn rule_interval_after() -> Rule {
    rule! {
        name: "after <time>",
        pattern: [
            re!(r"(?i)after\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["after"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.get(1)?)?.clone();

            Some(TimeExpr::After(Box::new(start)))
        }
    }
}

/// "since <time>"
pub fn rule_interval_since() -> Rule {
    rule! {
        name: "since <time>",
        pattern: [
            re!(r"(?i)since\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["since"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start = get_time_expr(tokens.get(1)?)?.clone();

            Some(TimeExpr::After(Box::new(start)))
        }
    }
}

/// "by <time>"
pub fn rule_interval_by() -> Rule {
    rule! {
        name: "by <time>",
        pattern: [
            re!(r"(?i)by\s+"),
            pred!(is_time_expr)
        ],
        required_phrases: ["by"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let end = get_time_expr(tokens.get(1)?)?.clone();

            // "by <time>" means an interval from now until that time.
            Some(TimeExpr::IntervalBetween {
                start: Box::new(TimeExpr::Reference),
                end: Box::new(end),
            })
        }
    }
}

/// "for <duration>"
pub fn rule_interval_for_duration() -> Rule {
    rule! {
        name: "for <duration>",
        pattern: [
            re!(r"(?i)for\s+"),
            pred!(is_duration_expr)
        ],
        required_phrases: ["for"],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Time],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let duration = get_duration_expr(tokens.get(1)?)?.clone();

            Some(TimeExpr::Duration(Box::new(duration)))
        }
    }
}
