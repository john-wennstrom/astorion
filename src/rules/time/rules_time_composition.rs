//! Time composition rules (cycle + time, ordinal cycle of time, etc.)

use crate::engine::BucketMask;
use crate::rules::numeral::helpers::first_match_lower;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, Grain, TimeExpr};
use crate::{Rule, Token};

/// "<day-of-month> of <month>" (5th of March, 25 of December)
pub fn rule_dom_of_time_month() -> Rule {
    rule! {
        name: "<day-of-month> (ordinal or number) of <month>",
        pattern: [pred!(is_day_of_month_numeral), re!(r"(?i)of( the)?"), pred!(is_month_expr)],
        required_phrases: ["of"],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.first()?)?;
            let month = month_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

/// "<day-of-month> of <time> (month-like)" (20 of next month, 20th of the current month)
///
/// This composes an existing DayOfMonth expression with a month-level time
/// expression such as "this month" or "next month" produced by cycle
/// or modifier rules. The right-hand side must normalize to the start of a
/// specific month; normalization of the resulting Intersect will then use
/// `Constraint::DayOfMonth` logic to pick the requested day within that
/// month.
pub fn rule_dom_of_time_month_like() -> Rule {
    rule! {
        name: "<day-of-month> of <time> (month-like)",
        pattern: [
            pred!(is_day_of_month_numeral),
            re!(r"(?i)\s+(?:day\s+)?of( the)?\s+"),
            pred!(is_time_expr),
        ],
        // Rely on bucket detection (digits + month word via the underlying
        // time expression); no extra phrase gating.
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.first()?)?;
            let time_expr = get_time_expr(tokens.get(2)?)?;

            // Only handle month-like container expressions such as
            // "this month", "next month", "last month", etc., which are
            // represented as StartOf{ grain: Month, .. }.
            match time_expr {
                TimeExpr::StartOf { grain: Grain::Month, .. } => {
                    Some(TimeExpr::Intersect {
                        expr: Box::new(time_expr.clone()),
                        constraint: Constraint::DayOfMonth(day),
                    })
                }
                _ => None,
            }
        }
    }
}

/// "the <cycle> after|before <time>" (the year after 2020, the month before Christmas)
pub fn rule_cycle_the_after_before_time() -> Rule {
    rule! {
        name: "the <cycle> after|before <time>",
        pattern: [re!(r"(?i)the\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)(after|before)\s+"), pred!(is_time_expr)],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let grain = first_match_lower(&tokens[1..])?;
            let relation = first_match_lower(&tokens[2..])?;
            let time_expr = get_time_expr(tokens.get(3)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let amount = if relation.trim() == "before" { -1 } else { 1 };

            let base = shift_by_grain(time_expr.clone(), amount, grain);
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

/// "<cycle> after|before <time>" (year after 2020, month before Christmas)
pub fn rule_cycle_after_before_time() -> Rule {
    rule! {
        name: "<cycle> after|before <time>",
        pattern: [re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)(after|before)\s+"), pred!(is_time_expr)],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let grain = first_match_lower(tokens)?;
            let relation = first_match_lower(&tokens[1..])?;
            let time_expr = get_time_expr(tokens.get(2)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let amount = if relation.trim() == "before" { -1 } else { 1 };

            let base = shift_by_grain(time_expr.clone(), amount, grain);
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

/// "<ordinal> <cycle> of <time>" (first week of January, third day of the month)
pub fn rule_cycle_ordinal_of_time() -> Rule {
    rule! {
        name: "<ordinal> <cycle> of <time>",
        pattern: [re!(r"(?i)(first|second|third|fourth|fifth|\d+(st|nd|rd|th))\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)(?:of|in|from)\s+"), pred!(is_time_expr)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = ordinal_value(tokens.first()?)?;
            let grain = first_match_lower(&tokens[1..])?;
            let time_expr = get_time_expr(tokens.get(3)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let container_grain = container_grain_for_expr(time_expr);
            let base = TimeExpr::StartOf {
                expr: Box::new(time_expr.clone()),
                grain: container_grain,
            };
            let shifted = shift_by_grain(base, ordinal - 1, grain);
            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "<ordinal> last <cycle> of <time>" (second last day of month)
pub fn rule_cycle_last_ordinal_of_time() -> Rule {
    rule! {
        name: "<ordinal> last <cycle> of <time>",
        pattern: [re!(r"(?i)(first|second|third|fourth|fifth|\d+(st|nd|rd|th))\s+"), re!(r"(?i)last\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)(?:of|in|from)\s+"), pred!(is_time_expr)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = ordinal_value(tokens.first()?)?;
            let grain = first_match_lower(&tokens[2..])?;
            let time_expr = get_time_expr(tokens.get(4)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let container_grain = container_grain_for_expr(time_expr);
            let base = TimeExpr::StartOf {
                expr: Box::new(time_expr.clone()),
                grain: container_grain,
            };
            let end = shift_by_grain(base, 1, container_grain);
            let shifted = shift_by_grain(end, -ordinal, grain);

            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "the <ordinal> <cycle> of <time>" (the first week of January)
pub fn rule_cycle_the_ordinal_of_time() -> Rule {
    rule! {
        name: "the <ordinal> <cycle> of <time>",
        pattern: [re!(r"(?i)the\s+"), re!(r"(?i)(first|second|third|fourth|fifth|\d+(st|nd|rd|th))\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)(?:of|in|from)\s+"), pred!(is_time_expr)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = ordinal_value(tokens.get(1)?)?;
            let grain = first_match_lower(&tokens[2..])?;
            let time_expr = get_time_expr(tokens.get(4)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let container_grain = container_grain_for_expr(time_expr);
            let base = TimeExpr::StartOf {
                expr: Box::new(time_expr.clone()),
                grain: container_grain,
            };
            let shifted = shift_by_grain(base, ordinal - 1, grain);
            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "the <ordinal> last <cycle> of <time>" (the second last day of month)
pub fn rule_cycle_the_last_ordinal_of_time() -> Rule {
    rule! {
        name: "the <ordinal> last <cycle> of <time>",
        pattern: [re!(r"(?i)the\s+"), re!(r"(?i)(first|second|third|fourth|fifth|\d+(st|nd|rd|th))\s+"), re!(r"(?i)last\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)(?:of|in|from)\s+"), pred!(is_time_expr)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = ordinal_value(tokens.get(1)?)?;
            let grain = first_match_lower(&tokens[3..])?;
            let time_expr = get_time_expr(tokens.get(5)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let container_grain = container_grain_for_expr(time_expr);
            let base = TimeExpr::StartOf {
                expr: Box::new(time_expr.clone()),
                grain: container_grain,
            };
            let end = shift_by_grain(base, 1, container_grain);
            let shifted = shift_by_grain(end, -ordinal, grain);

            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "the <cycle> of the <time grain>" (the week of the month, the day of the year)
pub fn rule_cycle_the_of_time_grain() -> Rule {
    rule! {
        name: "the <cycle> of the <time grain>",
        pattern: [re!(r"(?i)the\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)of( the)?\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let cycle = first_match_lower(&tokens[1..])?;
            let time_grain = first_match_lower(&tokens[3..])?;

            let cycle_grain = grain_from_cycle(cycle.trim())?;
            let time_grain = grain_from_cycle(time_grain.trim())?;
            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: time_grain,
            };

            let expr = if cycle_grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(base),
                    grain: cycle_grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(base),
                    grain: cycle_grain,
                }
            };

            Some(expr)
        }
    }
}

/// "the <cycle> of <time>" (the week of Christmas, the month of next year)
pub fn rule_cycle_the_of_time() -> Rule {
    rule! {
        name: "the <cycle> of <time>",
        pattern: [re!(r"(?i)the\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)of\s+"), pred!(is_time_expr)],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let cycle = first_match_lower(&tokens[1..])?;
            let time_expr = get_time_expr(tokens.get(3)?)?;

            let cycle_grain = grain_from_cycle(cycle.trim())?;
            let container_grain = container_grain_for_expr(time_expr);

            let expr = if cycle_grain == Grain::Week {
                if container_grain == Grain::Day {
                    let shifted = shift_by_grain(time_expr.clone(), 1, Grain::Day);
                    TimeExpr::StartOf {
                        expr: Box::new(shifted),
                        grain: Grain::Week,
                    }
                } else {
                    let base = TimeExpr::StartOf {
                        expr: Box::new(time_expr.clone()),
                        grain: container_grain,
                    };
                    TimeExpr::IntervalOf {
                        expr: Box::new(base),
                        grain: Grain::Week,
                    }
                }
            } else {
                let base = TimeExpr::StartOf {
                    expr: Box::new(time_expr.clone()),
                    grain: container_grain,
                };
                TimeExpr::StartOf {
                    expr: Box::new(base),
                    grain: cycle_grain,
                }
            };

            Some(expr)
        }
    }
}

/// "<ordinal> <cycle> after <time>" (first week after Christmas, second day after tomorrow)
pub fn rule_cycle_ordinal_after_time() -> Rule {
    rule! {
        name: "<ordinal> <cycle> after <time>",
        pattern: [re!(r"(?i)(first|second|third|fourth|fifth|\d+(st|nd|rd|th))\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)after\s+"), pred!(is_time_expr)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = ordinal_value(tokens.first()?)?;
            let grain = first_match_lower(&tokens[1..])?;
            let time_expr = get_time_expr(tokens.get(3)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let shifted = shift_by_grain(time_expr.clone(), ordinal, grain);
            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "the <ordinal> <cycle> after <time>" (the first week after Christmas)
pub fn rule_cycle_the_ordinal_after_time() -> Rule {
    rule! {
        name: "the <ordinal> <cycle> after <time>",
        pattern: [re!(r"(?i)the\s+"), re!(r"(?i)(first|second|third|fourth|fifth|\d+(st|nd|rd|th))\s+"), re!(r"(?i)\b(year|quarter|month|week|day)\b\s+"), re!(r"(?i)after\s+"), pred!(is_time_expr)],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = ordinal_value(tokens.get(1)?)?;
            let grain = first_match_lower(&tokens[2..])?;
            let time_expr = get_time_expr(tokens.get(4)?)?;

            let grain = grain_from_cycle(grain.trim())?;
            let shifted = shift_by_grain(time_expr.clone(), ordinal, grain);
            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}
