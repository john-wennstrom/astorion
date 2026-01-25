//! Time of day combination rules (tod + pod, special times like noon/midnight)

use crate::engine::BucketMask;
use crate::rules::numeral::helpers::first_match_lower;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, Grain, PartOfDay, TimeExpr};
use crate::{Rule, Token};

/// "noon", "midnight", "EOD", "end of day"
pub fn rule_noon_midnight_eod() -> Rule {
    rule! {
        name: "noon|midnight|EOD|end of day",
        pattern: [re!(r"(?i)(noon|midni(ght|te)|(the )?(eod|end of (the )?day))")],
        optional_phrases: ["noon", "midnight", "midnite", "eod", "end"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let hour = if matched.trim() == "noon" { 12 } else { 0 };
            let time = chrono::NaiveTime::from_hms_opt(hour, 0, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

/// "mid-day", "midday"
pub fn rule_mid_day() -> Rule {
    rule! {
        name: "Mid-day",
        pattern: [re!(r"(?i)(the )?mid(\s)?day")],
        required_phrases: ["mid", "day"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let time = chrono::NaiveTime::from_hms_opt(12, 0, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

/// "early morning"
pub fn rule_early_morning() -> Rule {
    rule! {
        name: "early morning",
        pattern: [re!(r"(?i)early ((in|hours of) the )?morning")],
        required_phrases: ["early", "morning"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Day,
            };
            let start_expr = base.clone();
            let end_expr = shift_by_grain(base, 9, Grain::Hour);
            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

/// "in the morning", "during the evening"
pub fn rule_pod_in() -> Rule {
    rule! {
        name: "in|during the <part-of-day>",
        pattern: [
            re!(r"(?i)(in|during)( the)?"),
            re!(r"(?i)\s*(?:at\s+)?(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        required_phrases: ["in", "during", "morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(&tokens[1..])?;
            let part = part_of_day_from_text(matched.as_str())?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

/// "tonight <time>", "late tonight 9pm"
pub fn rule_tonight_time_of_day() -> Rule {
    rule! {
        name: "tonight <time-of-day>",
        pattern: [
            re!(r"(?i)(late )?toni(ght|gth|te)s?"),
            re!(r"\s+"),
            pred!(is_time_of_day_expr),
        ],
        // Phrase gating: require only the canonical spelling "tonight".
        // The regex still accepts common misspellings (tonigth/tonite),
        // but trigger-based activation currently only tracks "tonight".
        // Using all variants here would AND them, effectively disabling
        // this rule under phrase gating.
        required_phrases: ["tonight"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let part = if matched.trim_start().starts_with("late") {
                PartOfDay::LateTonight
            } else {
                PartOfDay::Tonight
            };
            let time = time_from_expr(tokens.get(2)?)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

/// "<time> tonight"
pub fn rule_time_of_day_tonight() -> Rule {
    rule! {
        name: "<time-of-day> tonight",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+(late )?toni(ght|gth|te)s?"),
        ],
        // Same rationale as `rule_tonight_time_of_day` above: only gate on
        // the canonical "tonight" spelling so this rule participates in
        // trigger-based activation.
        required_phrases: ["tonight"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(&tokens[1..])?;
            let part = if matched.trim_start().starts_with("late") {
                PartOfDay::LateTonight
            } else {
                PartOfDay::Tonight
            };
            let time = time_from_expr(tokens.first()?)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

/// "after lunch", "after work", "after school"
pub fn rule_after_partofday() -> Rule {
    rule! {
        name: "after lunch/work/school",
        pattern: [re!(r"(?i)after[\s-]?(lunch|work|school)")],
        required_phrases: ["after"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(tokens)?;
            let part = if matched.contains("work") || matched.contains("school") {
                PartOfDay::AfterWork
            } else {
                PartOfDay::AfterLunch
            };

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

/// "<time> <part-of-day>"
pub fn rule_time_pod() -> Rule {
    rule! {
        name: "<time> <part-of-day>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s*(?:at\s+)?(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        required_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?.clone();
            let matched = first(&tokens[1..])?;
            let part = part_of_day_from_text(matched.as_str())?;

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

/// "<time-of-day> this <part-of-day>"
pub fn rule_tod_this_pod() -> Rule {
    rule! {
        name: "<time-of-day> this <part-of-day>",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s*this\s+"),
            re!(r"(?i)\s*(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        required_phrases: ["this", "morning", "afternoon", "evening", "night"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time = time_from_expr(tokens.first()?)?;
            let matched = first(&tokens[2..])?;
            let part = part_of_day_from_text(matched.as_str())?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

/// "<part-of-day> of <time>"
pub fn rule_pod_of_time() -> Rule {
    rule! {
        name: "<part-of-day> of <time>",
        pattern: [
            re!(r"(?i)\s*(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
            re!(r"\s+"),
            re!(r"(?i)of"),
            re!(r"\s+"),
            pred!(is_time_expr),
        ],
        required_phrases: ["morning", "afternoon", "evening", "night", "of"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(tokens)?;
            let part = part_of_day_from_text(matched.as_str())?;
            let time_expr = get_time_expr(tokens.get(4)?)?.clone();

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::PartOfDay(part),
            })
        }
    }
}

/// "<time-of-day> sharp|exactly"
pub fn rule_tod_precision() -> Rule {
    rule! {
        name: "<time-of-day> sharp|exactly",
        pattern: [pred!(is_time_of_day_expr), re!(r"(?i)(sharp|exactly|-?ish|approximately)")],
        required_phrases: ["sharp", "exactly", "ish", "approximately"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let expr = get_time_expr(tokens.first()?)?.clone();
            Some(expr)
        }
    }
}

/// "<time-of-day> <part-of-day>" (e.g., "3pm in the afternoon")
pub fn rule_tod_pod() -> Rule {
    rule! {
        name: "<time-of-day> <part-of-day>",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s*(?:in\s+(?:the\s+)?)?(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        required_phrases: ["morning", "afternoon", "evening", "night"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time = time_from_expr(tokens.first()?)?;
            let matched = first(&tokens[1..])?;
            let part = part_of_day_from_text(matched.as_str())?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}
