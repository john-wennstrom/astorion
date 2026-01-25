//! Weekday-based rules (WEEKDAYISH bucket)

use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Constraint, Grain, TimeExpr};
use crate::{Dimension, Rule, Token, TokenKind};

/// "last/next Monday", "this Tuesday"
pub fn rule_last_next_weekday() -> Rule {
    rule! {
        name: "last/next <weekday>",
        pattern: [
            re!(r"(?i)(this|next|last|coming|past|previous)\s+"),
            pred!(is_weekday_name)
        ],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let modifier = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let weekday = weekday_from_name(tokens.get(1)?)?;

            let expr = match modifier.as_str() {
                "this" => TimeExpr::Intersect {
                    expr: Box::new(TimeExpr::Reference),
                    constraint: Constraint::DayOfWeek(weekday),
                },
                "next" | "coming" => {
                    // Anchor to the start of *next* week (next Monday), then pick the
                    // requested weekday within that week.
                    let next_week_start = TimeExpr::Intersect {
                        expr: Box::new(TimeExpr::Reference),
                        constraint: Constraint::DayOfWeek(chrono::Weekday::Mon),
                    };
                    TimeExpr::Intersect {
                        expr: Box::new(next_week_start),
                        constraint: Constraint::DayOfWeek(weekday),
                    }
                }
                "last" | "past" | "previous" => {
                    let current_ref = TimeExpr::Reference;
                    let shifted = shift_by_grain(current_ref.clone(), -1, Grain::Week);
                    TimeExpr::Intersect {
                        expr: Box::new(shifted),
                        constraint: Constraint::DayOfWeek(weekday),
                    }
                }
                _ => return None,
            };

            Some(expr)
        }
    }
}

/// Just "Monday", "Tuesday", etc (standalone weekday)
pub fn rule_weekday() -> Rule {
    rule! {
        name: "<weekday>",
        pattern: [
            // Match standalone weekday names and common abbreviations
            re!(r"(?i)\b(monday|mon|tuesday|tues?|wednesday|wed|thursday|thu|thurs|friday|fri|saturday|sat|sunday|sun)\b")
        ],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.first()?)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

/// "<weekday> <time-of-day>"
pub fn rule_weekday_time() -> Rule {
    rule! {
        name: "<weekday> <time-of-day>",
        pattern: [re!(r"(?i)\b(monday|mon|tuesday|tues?|wednesday|wed|thursday|thu|thurs|friday|fri|saturday|sat|sunday|sun)\b\s*(?:at\s*)?(\d{1,2})\s*(?:[:h])\s*(\d{2})\b")],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let m = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let weekday_text = m.get(1)?.to_lowercase();
            let weekday = match weekday_text.as_str() {
                "monday" | "mon" => chrono::Weekday::Mon,
                "tuesday" | "tue" | "tues" => chrono::Weekday::Tue,
                "wednesday" | "wed" => chrono::Weekday::Wed,
                "thursday" | "thu" | "thurs" => chrono::Weekday::Thu,
                "friday" | "fri" => chrono::Weekday::Fri,
                "saturday" | "sat" => chrono::Weekday::Sat,
                "sunday" | "sun" => chrono::Weekday::Sun,
                _ => return None,
            };

            let hour: u32 = m.get(2)?.parse().ok()?;
            let minute: u32 = m.get(3)?.parse().ok()?;
            if hour > 23 || minute > 59 {
                return None;
            }

            let time = chrono::NaiveTime::from_hms_opt(hour, minute, 0)?;

            let weekday_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };

            Some(TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

/// "<time>'s <weekday>" or "<time> on <weekday>"
pub fn rule_time_poss_weekday() -> Rule {
    rule! {
        name: "<time>'s <weekday>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s*('s|on)\s+"),
            pred!(is_weekday_name)
        ],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base_expr = get_time_expr(tokens.first()?)?.clone();
            let weekday = weekday_from_name(tokens.get(2)?)?;

            Some(TimeExpr::Intersect {
                expr: Box::new(base_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

/// "<weekday> <day-of-month>"
pub fn rule_weekday_day_of_month() -> Rule {
    rule! {
        name: "<weekday> <day-of-month>",
        pattern: [pred!(is_weekday_name), re!(r"\s+"), pred!(is_day_of_month_numeral)],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.first()?)?;
            let day = day_of_month_from_expr(tokens.get(2)?)?;

            let day_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfMonth(day),
            };

            Some(TimeExpr::Intersect {
                expr: Box::new(day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

/// "last Monday of month"
pub fn rule_last_weekday_of_month() -> Rule {
    rule! {
        name: "last <weekday> of <month>",
        pattern: [
            re!(r"(?i)last\s+"),
            pred!(is_weekday_name),
            re!(r"(?i)\s+(of|in)\s+"),
            pred!(is_month_expr)
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.get(1)?)?;
            let month = month_from_expr(tokens.get(3)?)?;

            Some(TimeExpr::LastWeekdayOfMonth {
                year: None,
                month,
                weekday,
            })
        }
    }
}

/// "nth Monday of month" (e.g., "first Monday of March")
pub fn rule_nth_weekday_of_month() -> Rule {
    rule! {
        name: "nth <weekday> of <month>",
        pattern: [
            re!(r"(?i)(first|second|third|fourth|fifth|1st|2nd|3rd|4th|5th)\s+"),
            pred!(is_weekday_name),
            re!(r"\s+of\s+"),
            pred!(is_month_expr)
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n_text = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let n = match n_text.to_lowercase().as_str() {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };
            let weekday = weekday_from_name(tokens.get(1)?)?;
            let month = month_from_expr(tokens.get(3)?)?;

            Some(TimeExpr::NthWeekdayOfMonth {
                n,
                year: None,
                month,
                weekday,
            })
        }
    }
}

/// "nth Monday of month year" (e.g., "first Monday of March 2024")
pub fn rule_nth_weekday_of_month_year() -> Rule {
    rule! {
        name: "nth <weekday> of <month> <year>",
        pattern: [
            re!(r"(?i)(first|second|third|fourth|fifth|1st|2nd|3rd|4th|5th)\s+"),
            pred!(is_weekday_name),
            re!(r"\s+of\s+"),
            pred!(is_month_expr),
            re!(r"\s+(\d{4})")
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH | BucketMask::ORDINALISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n_text = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let n = match n_text.to_lowercase().as_str() {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };
            let weekday = weekday_from_name(tokens.get(1)?)?;
            let month = month_from_expr(tokens.get(3)?)?;
            let year = regex_group_int_value(tokens.get(4)?, 1)? as i32;

            Some(TimeExpr::NthWeekdayOfMonth {
                n,
                year: Some(year),
                month,
                weekday,
            })
        }
    }
}

/// "nth Monday in/of (this|last|next) month" (e.g., "the first Monday in this month")
pub fn rule_nth_weekday_of_relative_month() -> Rule {
    use chrono::Weekday;

    rule! {
        name: "nth <weekday> in/of <relative-month>",
        pattern: [
            re!(r"(?i)(?:the\s+)?(first|second|third|fourth|fifth|1st|2nd|3rd|4th|5th)\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tue|tues|wed|thu|thur|thurs|fri|sat|sun)\s+(?:of|in)\s+(?:(this|last|next)\s+)?(?:the\s+)?month\b")
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let n_text = groups.get(1)?.to_lowercase();
            let n: i32 = match n_text.as_str() {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };

            let dow_str = groups.get(2)?.trim().to_lowercase();
            let weekday = match dow_str.as_str() {
                "monday" | "mon" => Weekday::Mon,
                "tuesday" | "tue" | "tues" => Weekday::Tue,
                "wednesday" | "wed" => Weekday::Wed,
                "thursday" | "thu" | "thur" | "thurs" => Weekday::Thu,
                "friday" | "fri" => Weekday::Fri,
                "saturday" | "sat" => Weekday::Sat,
                "sunday" | "sun" => Weekday::Sun,
                _ => return None,
            };

            let month_delta: i32 = match groups.get(3).map(|s| s.trim().to_lowercase()) {
                Some(s) if s == "last" => -1,
                Some(s) if s == "next" => 1,
                Some(s) if s == "this" => 0,
                None => 0,
                _ => return None,
            };

            let this_month = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Month,
            };

            let base_month = if month_delta == 0 {
                this_month
            } else {
                shift_by_grain(this_month, month_delta, Grain::Month)
            };

            let first_weekday = TimeExpr::Intersect {
                expr: Box::new(base_month),
                constraint: Constraint::DayOfWeek(weekday),
            };

            let expr = if n <= 1 {
                first_weekday
            } else {
                shift_by_grain(first_weekday, n - 1, Grain::Week)
            };

            Some(expr)
        }
    }
}

pub fn rule_nth_weekday_after_time() -> Rule {
    rule! {
        name: "nth <weekday> after <time>",
        pattern: [
            re!(r"(?i)(first|second|third|fourth|fifth|1st|2nd|3rd|4th|5th)\s+"),
            pred!(is_weekday_name),
            re!(r"(?i)\s+after\s+"),
            pred!(is_time_expr)
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n_text = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };

            let n: i32 = match n_text.to_lowercase().as_str() {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };

            let weekday = weekday_from_name(tokens.get(1)?)?;
            let time_expr = get_time_expr(tokens.get(3)?)?;

            let first_after = TimeExpr::Intersect {
                expr: Box::new(shift_by_grain(time_expr.clone(), 1, Grain::Day)),
                constraint: Constraint::DayOfWeek(weekday),
            };

            let expr = shift_by_grain(first_after, n - 1, Grain::Week);
            Some(expr)
        }
    }
}

/// "last Monday of month year"
pub fn rule_last_weekday_of_month_year() -> Rule {
    rule! {
        name: "last <weekday> of <month> <year>",
        pattern: [
            re!(r"(?i)last\s+"),
            pred!(is_weekday_name),
            re!(r"(?i)\s+(of|in)\s+"),
            pred!(is_month_expr),
            re!(r"\s+(\d{4})")
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.get(1)?)?;
            let month = month_from_expr(tokens.get(3)?)?;
            let year = regex_group_int_value(tokens.get(4)?, 1)? as i32;

            Some(TimeExpr::LastWeekdayOfMonth {
                year: Some(year),
                month,
                weekday,
            })
        }
    }
}

/// "first Monday of month"
pub fn rule_first_weekday_of_month() -> Rule {
    rule! {
        name: "first <weekday> of <month>",
        pattern: [
            re!(r"(?i)first\s+"),
            pred!(is_weekday_name),
            re!(r"\s+of\s+"),
            pred!(is_month_expr)
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.get(1)?)?;
            let month = month_from_expr(tokens.get(3)?)?;

            Some(TimeExpr::FirstWeekdayOfMonth {
                year: None,
                month,
                weekday,
            })
        }
    }
}

/// "<weekday>, <month> <day>"
pub fn rule_weekday_comma_month_day() -> Rule {
    rule! {
        name: "<weekday>, <month> <day>",
        pattern: [
            pred!(is_weekday_name),
            re!(r",\s*"),
            pred!(is_month_expr),
            re!(r"\s+"),
            pred!(is_day_of_month_numeral)
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.first()?)?;
            let month = month_from_expr(tokens.get(2)?)?;
            let day = day_of_month_from_expr(tokens.get(4)?)?;

            let month_day_expr = TimeExpr::MonthDay { month, day };

            Some(TimeExpr::Intersect {
                expr: Box::new(month_day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

/// "<weekday> <month> <day>"
pub fn rule_weekday_month_day() -> Rule {
    rule! {
        name: "<weekday> <month> <day>",
        pattern: [
            pred!(is_weekday_name),
            re!(r"\s+"),
            pred!(is_month_expr),
            re!(r"\s+"),
            pred!(is_day_of_month_numeral)
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.first()?)?;
            let month = month_from_expr(tokens.get(2)?)?;
            let day = day_of_month_from_expr(tokens.get(4)?)?;

            let month_day_expr = TimeExpr::MonthDay { month, day };

            Some(TimeExpr::Intersect {
                expr: Box::new(month_day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

/// "<month> <day>, <weekday>"
pub fn rule_month_day_comma_weekday() -> Rule {
    rule! {
        name: "<month> <day>, <weekday>",
        pattern: [
            pred!(is_month_expr),
            re!(r"\s+"),
            pred!(is_day_of_month_numeral),
            re!(r",\s*"),
            // Match weekday name or common abbreviation (used as regex token
            // so we can convert it to a chrono::Weekday in production).
            re!(r"(?i)\b(monday|mon|tuesday|tues?|wednesday|wed|thursday|thu|thurs|friday|fri|saturday|sat|sunday|sun)\b")
        ],
        buckets: (BucketMask::MONTHISH | BucketMask::HAS_DIGITS | BucketMask::WEEKDAYISH).bits(),
        // Depends on Numeral so that day-of-month expressions from numerals
        // (including ordinals like "18th") are available before this runs.
        deps: [Dimension::Numeral],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.first()?)?;
            let day = day_of_month_from_expr(tokens.get(2)?)?;
            let weekday = weekday_from_name(tokens.get(4)?)?;

            let month_day_expr = TimeExpr::MonthDay { month, day };

            Some(TimeExpr::Intersect {
                expr: Box::new(month_day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

/// "the closest Monday to Oct 5th"
pub fn rule_closest_weekday_to_month_day() -> Rule {
    rule! {
        name: "closest <weekday> to <month-day>",
        pattern: [
            re!(r"(?i)\b(the\s+)?closest\s+"),
            pred!(is_weekday_name),
            re!(r"(?i)\s+to\s+"),
            pred!(is_month_day_expr),
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.get(1)?)?;
            let target = get_time_expr(tokens.get(3)?)?.clone();
            Some(TimeExpr::ClosestWeekdayTo {
                n: 1,
                weekday,
                target: Box::new(target),
            })
        }
    }
}

/// "the second closest Mon to October fifth"
pub fn rule_nth_closest_weekday_to_month_day() -> Rule {
    rule! {
        name: "nth closest <weekday> to <month-day>",
        pattern: [
            re!(r"(?i)\b(the\s+)?(second|third|fourth|fifth|\d+)(?:st|nd|rd|th)?\s+closest\s+"),
            pred!(is_weekday_name),
            re!(r"(?i)\s+to\s+"),
            pred!(is_month_day_expr),
        ],
        buckets: (BucketMask::WEEKDAYISH | BucketMask::MONTHISH | BucketMask::ORDINALISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n_raw = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(2)?.to_lowercase(),
                _ => return None,
            };

            let n: u32 = match n_raw.as_str() {
                "second" => 2,
                "third" => 3,
                "fourth" => 4,
                "fifth" => 5,
                _ => n_raw.parse().ok()?,
            };

            if n < 2 {
                return None;
            }

            let weekday = weekday_from_name(tokens.get(1)?)?;
            let target = get_time_expr(tokens.get(3)?)?.clone();
            Some(TimeExpr::ClosestWeekdayTo {
                n,
                weekday,
                target: Box::new(target),
            })
        }
    }
}
