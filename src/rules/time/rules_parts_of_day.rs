//! Part of day rules (morning, afternoon, evening, night, tonight, etc.)

use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::Constraint;
use crate::time_expr::{Grain, PartOfDay, TimeExpr};
use crate::{Rule, Token};

/// "morning", "afternoon", "evening", "night"
pub fn rule_part_of_days() -> Rule {
    rule! {
        name: "part of days",
        pattern: [re!(r"(?i)(morning|afternoon|evening|night|tonight)")],
        optional_phrases: ["morning", "afternoon", "evening", "night", "tonight"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let part_of_day = part_of_day_from_token(tokens.first()?)?;
            Some(TimeExpr::PartOfDay(part_of_day))
        }
    }
}

/// "this morning", "this afternoon", "this evening"
pub fn rule_this_part_of_day() -> Rule {
    rule! {
        name: "this <part-of-day>",
        pattern: [
            re!(r"(?i)this\s+"),
            re!(r"(?i)(morning|afternoon|evening|night)")
        ],
        required_phrases: ["this"],
        optional_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let part_of_day = part_of_day_from_token(tokens.get(1)?)?;

            let today = TimeExpr::Reference;
            Some(TimeExpr::Intersect {
                expr: Box::new(today),
                constraint: crate::time_expr::Constraint::PartOfDay(part_of_day),
            })
        }
    }
}

/// "tonight" (standalone)
pub fn rule_tonight() -> Rule {
    rule! {
        name: "tonight",
        pattern: [re!(r"(?i)tonight")],
        required_phrases: ["tonight"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let today = TimeExpr::Reference;
            Some(TimeExpr::Intersect {
                expr: Box::new(today),
                constraint: crate::time_expr::Constraint::PartOfDay(PartOfDay::Night),
            })
        }
    }
}

/// "late tonight" (21:00-00:00)
pub fn rule_late_tonight() -> Rule {
    rule! {
        name: "late tonight",
        pattern: [re!(r"(?i)late\s+toni(ght|gth|te)s?")],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let today = TimeExpr::Reference;
            Some(TimeExpr::Intersect {
                expr: Box::new(today),
                constraint: crate::time_expr::Constraint::PartOfDay(PartOfDay::LateTonight),
            })
        }
    }
}

/// "tomorrow morning", "tomorrow afternoon", etc.
pub fn rule_tomorrow_part_of_day() -> Rule {
    rule! {
        name: "tomorrow <part-of-day>",
        pattern: [
            re!(r"(?i)tomorrow\s+"),
            re!(r"(?i)(morning|afternoon|evening|night)")
        ],
        optional_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let part_of_day = part_of_day_from_token(tokens.get(1)?)?;

            let tomorrow = shift_by_grain(TimeExpr::Reference, 1, Grain::Day);
            Some(TimeExpr::Intersect {
                expr: Box::new(tomorrow),
                constraint: crate::time_expr::Constraint::PartOfDay(part_of_day),
            })
        }
    }
}

/// "yesterday morning", "yesterday afternoon", etc.
pub fn rule_yesterday_part_of_day() -> Rule {
    rule! {
        name: "yesterday <part-of-day>",
        pattern: [
            re!(r"(?i)yesterday\s+"),
            re!(r"(?i)(morning|afternoon|evening|night)")
        ],
        optional_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let part_of_day = part_of_day_from_token(tokens.get(1)?)?;

            let yesterday = shift_by_grain(TimeExpr::Reference, -1, Grain::Day);
            Some(TimeExpr::Intersect {
                expr: Box::new(yesterday),
                constraint: crate::time_expr::Constraint::PartOfDay(part_of_day),
            })
        }
    }
}

/// "<integer> in the morning/afternoon/evening" - produces time-of-day
pub fn rule_integer_in_part_of_day() -> Rule {
    use crate::rules::numeral::predicates::number_between;

    rule! {
        name: "<integer> in the <part-of-day>",
        pattern: [
            pred!(|t: &Token| number_between::<0, 24>(t)),
            re!(r"(?i)\s+in\s+the\s+"),
            re!(r"(?i)(morning|afternoon|evening|night)")
        ],
        optional_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = integer_value(tokens.first()?)? as u32;
            let part_of_day = part_of_day_from_token(tokens.get(2)?)?;

            if hour > 24 {
                return None;
            }

            let hour_24 = if hour == 24 { 0 } else { hour };
            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part_of_day);

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

/// "<time> in the morning/afternoon/evening"
pub fn rule_time_in_part_of_day() -> Rule {
    rule! {
        name: "<time> in the <part-of-day>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s+in\s+the\s+"),
            re!(r"(?i)(morning|afternoon|evening|night)")
        ],
        optional_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?.clone();
            let part_of_day = part_of_day_from_token(tokens.get(2)?)?;

            // Special handling: if the time_expr is already a TimeOfDay constraint,
            // adjust the hour based on the part of day instead of creating an interval
            match &time_expr {
                TimeExpr::Intersect {
                    expr,
                    constraint: Constraint::TimeOfDay(time),
                } => {
                    let adjusted_time = adjust_time_for_part_of_day(*time, part_of_day);
                    Some(TimeExpr::Intersect {
                        expr: expr.clone(),
                        constraint: Constraint::TimeOfDay(adjusted_time),
                    })
                }
                _ => {
                    // For other time expressions, apply PartOfDay constraint
                    Some(TimeExpr::Intersect {
                        expr: Box::new(time_expr),
                        constraint: crate::time_expr::Constraint::PartOfDay(part_of_day),
                    })
                }
            }
        }
    }
}

/// "<part-of-day> of <time>"
pub fn rule_part_of_day_of_time() -> Rule {
    rule! {
        name: "<part-of-day> of <time>",
        pattern: [
            re!(r"(?i)(morning|afternoon|evening|night)"),
            re!(r"(?i)\s+of\s+(?:the\s+)?(?:this\s+)?"),
            pred!(is_time_expr)
        ],
        optional_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let part_of_day = part_of_day_from_token(tokens.first()?)?;
            let time_expr = get_time_expr(tokens.get(2)?)?.clone();

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: crate::time_expr::Constraint::PartOfDay(part_of_day),
            })
        }
    }
}

/// "last night"
pub fn rule_last_night() -> Rule {
    rule! {
        name: "last night",
        pattern: [re!(r"(?i)last\s+night")],
        required_phrases: ["last", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let yesterday = shift_by_grain(TimeExpr::Reference, -1, Grain::Day);
            Some(TimeExpr::Intersect {
                expr: Box::new(yesterday),
                constraint: crate::time_expr::Constraint::PartOfDay(PartOfDay::Night),
            })
        }
    }
}

pub fn rule_relative_day_part_of_day() -> Rule {
    rule! {
        name: "<relative-day> <part-of-day>",
        pattern: [
            re!(r"(?i)(today|tomorrow|yesterday)"),
            re!(r"(?i)\s+(?:at\s+)?"),
            re!(r"(?i)(?:early\s+morning|early\s+in\s+the\s+morning|early\s+hours\s+of\s+the\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = first(tokens)?;
            let pod = first(&tokens[2..])?;
            let part = part_of_day_from_text(pod.as_str())?;

            let base = match day.as_str() {
                "today" => TimeExpr::StartOf {
                    expr: Box::new(TimeExpr::Reference),
                    grain: Grain::Day,
                },
                "tomorrow" => {
                    let shifted = shift_by_grain(TimeExpr::Reference, 1, Grain::Day);
                    TimeExpr::StartOf {
                        expr: Box::new(shifted),
                        grain: Grain::Day,
                    }
                }
                "yesterday" => {
                    let shifted = shift_by_grain(TimeExpr::Reference, -1, Grain::Day);
                    TimeExpr::StartOf {
                        expr: Box::new(shifted),
                        grain: Grain::Day,
                    }
                }
                _ => return None,
            };

            Some(TimeExpr::Intersect {
                expr: Box::new(base),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

pub fn rule_weekday_part_of_day() -> Rule {
    rule! {
        name: "<weekday> <part-of-day>",
        pattern: [
            pred!(is_weekday_expr),
            re!(r"\s+"),
            re!(r"(?i)(?:early\s+morning|early\s+in\s+the\s+morning|early\s+hours\s+of\s+the\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday_expr = get_time_expr(tokens.first()?)?.clone();
            let pod = first(&tokens[2..])?;
            let part = part_of_day_from_text(pod.as_str())?;

            Some(TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

pub fn rule_weekday_in_the_part_of_day() -> Rule {
    rule! {
        name: "<weekday> in|during the <part-of-day>",
        pattern: [
            pred!(is_weekday_expr),
            re!(r"\s+"),
            re!(r"(?i)(in|during)( the)?"),
            re!(r"\s+"),
            re!(r"(?i)(?:early\s+morning|early\s+in\s+the\s+morning|early\s+hours\s+of\s+the\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday_expr = get_time_expr(tokens.first()?)?.clone();
            let pod = first(&tokens[4..])?;
            let part = part_of_day_from_text(pod.as_str())?;

            Some(TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

pub fn rule_date_in_the_part_of_day() -> Rule {
    rule! {
        name: "<date> in|during the <part-of-day>",
        pattern: [
            pred!(is_time_expr),
            re!(r"\s+"),
            re!(r"(?i)(in|during)( the)?"),
            re!(r"\s+"),
            re!(r"(?i)(?:early\s+morning|early\s+in\s+the\s+morning|early\s+hours\s+of\s+the\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let date_expr = get_time_expr(tokens.first()?)?.clone();
            let pod = first(&tokens[4..])?;
            let part = part_of_day_from_text(pod.as_str())?;

            Some(TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}
