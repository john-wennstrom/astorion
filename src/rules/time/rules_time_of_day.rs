//! Time-of-day rules (HAS_COLON, HAS_AMPM buckets)

use crate::engine::BucketMask;
use crate::rules::numeral::helpers::first_match_lower;
use crate::rules::numeral::predicates::number_between;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::Constraint;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};
use chrono::Timelike;
// Already imported above

fn tod_expr_with_precision(time: chrono::NaiveTime, precision: Option<Grain>) -> Option<TimeExpr> {
    let base = TimeExpr::Intersect { expr: Box::new(TimeExpr::Reference), constraint: Constraint::TimeOfDay(time) };
    let expr = match precision {
        Some(grain) => TimeExpr::Shift { expr: Box::new(base), amount: 0, grain },
        None => base,
    };

    Some(expr)
}

/// hh:mm time-of-day (e.g., "3:45" or "3:45pm")
pub fn rule_hhmm_time() -> Rule {
    rule! {
        name: "hh:mm (time-of-day)",
        pattern: [
            re!(r"(?i)(\d{1,2}):(\d{2})(?:\s*(am?|pm?))?")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)? as u32;
            let minute = regex_group_int_value(tokens.first()?, 2)? as u32;

            // Check for am/pm
            let am_pm = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(3).map(|s: &String| s.to_lowercase()),
                _ => None,
            };

            let hour_24 = match am_pm.as_deref() {
                Some("pm") | Some("p") => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                },
                Some("am") | Some("a") => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                },
                None => {
                    // No AM/PM specified, default to afternoon for 1-11
                    if hour > 23 {
                        return None;
                    }
                    match hour {
                        0 => hour,
                        1..=11 => hour + 12,
                        _ => hour,
                    }
                },
                _ => return None,
            };

            if minute > 59 {
                return None;
            }

            let time = chrono::NaiveTime::from_hms_opt(hour_24, minute, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

/// hh am/pm (e.g., "3pm")
pub fn rule_hh_time() -> Rule {
    rule! {
        name: "hh (time-of-day)",
        pattern: [
            re!(r"(?i)(\d{1,2})\s*(am?|pm?)")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_AMPM).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)? as u32;
            let am_pm = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(2)?.to_lowercase(),
                _ => return None,
            };

            let hour_24 = match am_pm.as_str() {
                "pm" | "p" => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                }
                "am" | "a" => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                }
                _ => return None,
            };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

pub fn rule_hh_in_the_ampm() -> Rule {
    rule! {
        name: "hh in the am|pm",
        pattern: [re!(r"(?i)\b(\d{1,2})\b\s+in\s+the\s+([ap])\.?m\.?\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_AMPM).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)? as u32;
            let ap = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(2)?.to_lowercase(),
                _ => return None,
            };

            let hour_24 = match ap.as_str() {
                "p" => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                }
                "a" => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                }
                _ => return None,
            };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

pub fn rule_hh_oclock_ampm() -> Rule {
    rule! {
        name: "hh o'clock am|pm",
        pattern: [re!(r"(?i)\b(\d{1,2})\s*o'?clock\s*(am?|pm?)\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_AMPM).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)? as u32;
            let am_pm = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(2)?.to_lowercase(),
                _ => return None,
            };

            let hour_24 = match am_pm.as_str() {
                "pm" | "p" => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                }
                "am" | "a" => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                }
                _ => return None,
            };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

pub fn rule_hh_oclock() -> Rule {
    rule! {
        name: "hh o'clock",
        pattern: [re!(r"(?i)\b(\d{1,2})\s*o'?clock\b")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)? as u32;

            if hour > 24 {
                return None;
            }

            // Use hour as-is (0-24 format)
            let hour_24 = if hour == 24 { 0 } else { hour };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

pub fn rule_numeral_ampm() -> Rule {
    rule! {
        name: "<integer> am|pm",
        pattern: [pred!(|t: &Token| number_between::<1, 12>(t)), re!(r"(?i)\s*(am?|pm?)\b")],
        buckets: BucketMask::HAS_AMPM.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = integer_value(tokens.first()?)? as u32;
            let am_pm = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let hour_24 = match am_pm.as_str() {
                "pm" | "p" => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                }
                "am" | "a" => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                }
                _ => return None,
            };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

pub fn rule_at_numeral_ampm() -> Rule {
    rule! {
        name: "at <integer> am|pm",
        pattern: [re!(r"(?i)at\s+"), pred!(|t: &Token| number_between::<1, 12>(t)), re!(r"(?i)\s*(am?|pm?)\b")],
        buckets: BucketMask::HAS_AMPM.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = integer_value(tokens.get(1)?)? as u32;
            let am_pm = match &tokens.get(2)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let hour_24 = match am_pm.as_str() {
                "pm" | "p" => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                }
                "am" | "a" => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                }
                _ => return None,
            };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

/// hh (0-24 hour format)
pub fn rule_hh() -> Rule {
    rule! {
        name: "hh",
        pattern: [
            re!(r"(?i)\b(0?[0-9]|1[0-9]|2[0-4])\b")
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour = regex_group_int_value(tokens.first()?, 1)? as u32;

            if hour > 24 {
                return None;
            }

            let time = chrono::NaiveTime::from_hms_opt(hour, 0, 0)?;
            tod_expr_with_precision(time, Some(Grain::Hour))
        }
    }
}

/// <time-of-day> am|pm
pub fn rule_tod_ampm() -> Rule {
    rule! {
        name: "<time-of-day> am|pm",
        pattern: [pred!(is_time_of_day_expr), re!(r"(?i)\s*(in the )?([ap])(\s|\.)?(m?)\.?")],
        buckets: BucketMask::HAS_AMPM.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time = time_from_expr(tokens.first()?)?;
            let precision_marker = match &tokens.first()?.kind {
                TokenKind::TimeExpr(TimeExpr::Shift {
                    amount: 0,
                    grain,
                    ..
                }) => Some(grain),
                _ => None,
            };
            let period_token = tokens.get(1)?;
            let ap = first_match_lower(std::slice::from_ref(period_token))?;

            let hour = time.hour() as i64;
            let adjusted_hour = match ap.as_str() {
                "p" => match hour {
                    12 => 12,
                    0..=11 => hour + 12,
                    _ => return None,
                },
                "a" => match hour {
                    12 => 0,
                    0..=11 => hour,
                    _ => return None,
                },
                _ => return None,
            };

            let t = chrono::NaiveTime::from_hms_opt(
                adjusted_hour as u32,
                time.minute(),
                time.second(),
            )?;
            match precision_marker {
                Some(Grain::Second) => tod_expr_with_precision(t, Some(Grain::Second)),
                Some(Grain::Hour) => tod_expr_with_precision(t, Some(Grain::Hour)),
                _ => tod_expr_with_precision(t, None),
            }
        }
    }
}

/// <ambiguous-time> am|pm (e.g., "seven thirty p.m.")
pub fn rule_ambiguous_tod_ampm() -> Rule {
    rule! {
        name: "<ambiguous-time> am|pm",
        pattern: [pred!(is_ambiguous_time_expr), re!(r"(?i)\s*(in the )?([ap])(\s|\.)?(m?)\.?" )],
        buckets: BucketMask::HAS_AMPM.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (hour, minute) = match &tokens.first()?.kind {
                TokenKind::TimeExpr(TimeExpr::AmbiguousTime { hour, minute }) => (hour, minute),
                _ => return None,
            };

            let period_token = tokens.get(1)?;
            let ap = first_match_lower(std::slice::from_ref(period_token))?;
            let ap = ap.trim().to_lowercase();

            let hour = *hour;
            let minute = *minute;
            let hour_24 = match ap.as_str() {
                s if s.contains('p') => {
                    if hour == 12 { 12 } else { hour + 12 }
                }
                s if s.contains('a') => {
                    if hour == 12 { 0 } else { hour }
                }
                _ => return None,
            };

            let time = chrono::NaiveTime::from_hms_opt(hour_24, minute, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}
