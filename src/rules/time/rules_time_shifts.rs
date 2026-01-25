//! Time shift rules (in X duration, X ago/hence, duration after/before time)

use crate::engine::BucketMask;
use crate::rules::numeral::helpers::first_match_lower;
use crate::rules::numeral::predicates::number_between;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::Constraint;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};

/// "in a week" (7 days from now, rounded to day boundary)
pub fn rule_in_a_week() -> Rule {
    rule! {
        name: "in a week",
        pattern: [re!(r"(?i)in\s+a\s+week")],
        required_phrases: ["in", "week"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, 1, Grain::Week);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "a/one week ago" (7 days ago, rounded to day boundary)
pub fn rule_a_week_ago() -> Rule {
    rule! {
        name: "a/one week ago",
        pattern: [re!(r"(?i)(a|one)\s+week\s+ago")],
        required_phrases: ["week", "ago"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, -1, Grain::Week);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "a/one week hence" (7 days from now, rounded to day boundary)
pub fn rule_a_week_hence() -> Rule {
    rule! {
        name: "a/one week hence",
        pattern: [re!(r"(?i)(a|one)\s+week\s+hence")],
        required_phrases: ["week", "hence"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, 1, Grain::Week);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Day,
            })
        }
    }
}

/// "a fortnight ago" (2 weeks ago)
pub fn rule_fortnight_ago() -> Rule {
    rule! {
        name: "a fortnight ago",
        pattern: [re!(r"(?i)a\s+fortnight\s+ago")],
            required_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, -2, Grain::Week);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Hour,
            })
        }
    }
}

/// "a fortnight hence" (2 weeks from now)
pub fn rule_fortnight_hence() -> Rule {
    rule! {
        name: "a fortnight hence",
        pattern: [re!(r"(?i)a\s+fortnight\s+hence")],
            required_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let shifted = shift_by_grain(TimeExpr::Reference, 2, Grain::Week);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Hour,
            })
        }
    }
}

/// "in <decimal> hours/minutes/seconds" (in 2.5 hours)
pub fn rule_in_decimal_duration() -> Rule {
    rule! {
        name: "in <decimal> hours/minutes/seconds",
        pattern: [re!(r"(?i)in\s+(\d+(?:\.\d+)?)\s*(hours?|hrs?|minutes?|mins?|seconds?|secs?)")],
        required_phrases: ["in"],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount_str = groups.get(1)?;
            let amount_f64: f64 = amount_str.parse().ok()?;
            let unit = groups.get(2)?.to_lowercase();

            let grain = if unit.starts_with("h") {
                Grain::Hour
            } else if unit.starts_with("m") {
                Grain::Minute
            } else if unit.starts_with("s") {
                Grain::Second
            } else {
                return None;
            };

            // Convert to minutes for finer precision
            let total_minutes = match grain {
                Grain::Hour => (amount_f64 * 60.0).round() as i32,
                Grain::Minute => amount_f64.round() as i32,
                Grain::Second => (amount_f64 / 60.0).round() as i32,
                _ => return None,
            };

            let expr = shift_by_grain(TimeExpr::Reference, total_minutes, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in <number> and a/an half hours" (in 2 and a half hours)
pub fn rule_in_integer_and_half_duration() -> Rule {
    rule! {
        name: "in <number> and a/an half hours",
        pattern: [re!(r"(?i)in\s+(\d+)\s+and\s+(?:a|an)\s+half\s+(hours?|hrs?|minutes?|mins?)")],
        required_phrases: ["in", "and", "half"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let whole_part: i32 = groups.get(1)?.parse().ok()?;
            let unit = groups.get(2)?.to_lowercase();

            let grain = if unit.starts_with("h") {
                Grain::Hour
            } else if unit.starts_with("m") {
                Grain::Minute
            } else {
                return None;
            };

            // Calculate total minutes
            let total_minutes = match grain {
                Grain::Hour => whole_part * 60 + 30,  // Add 30 minutes for half hour
                Grain::Minute => whole_part + whole_part / 2,  // Add half the minutes
                _ => return None,
            };

            let expr = shift_by_grain(TimeExpr::Reference, total_minutes, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in <number>" (implicit minutes, 0-61)
pub fn rule_in_numeral() -> Rule {
    rule! {
        name: "in <number> (implicit minutes)",
        pattern: [re!(r"(?i)in"), pred!(|t: &Token| number_between::<0, 61>(t))],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let minutes = i32::try_from(integer_value(tokens.get(1)?)?).ok()?;

            let expr = shift_by_grain(TimeExpr::Reference, minutes, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in <integer> h(ours)" (in 2h, in 5 hours)
pub fn rule_in_hours_short() -> Rule {
    rule! {
        name: "in <integer> h(ours)",
        pattern: [re!(r"(?i)in\s+(\d+)\s*h(?:ours?)?(?:\s+from\s+now)?")],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hours = regex_group_int_value(tokens.first()?, 1)? as i32;
            let expr = shift_by_grain(TimeExpr::Reference, hours, Grain::Hour);
            Some(expr)
        }
    }
}

/// "in a quarter/half of an hour" (in 15/30 minutes)
pub fn rule_in_quarter_half_hour() -> Rule {
    rule! {
        name: "in a quarter/half of an hour",
        pattern: [re!(r"(?i)in\s+(?:about\s+)?(?:a\s+)?(quarter|half|three-quarters)\s+(?:of\s+)?(?:an\s+)?hour")],
        required_phrases: ["in", "hour"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let fraction = groups.get(1)?.to_lowercase();
            let minutes = match fraction.as_str() {
                "quarter" => 15,
                "half" => 30,
                "three-quarters" => 45,
                _ => return None,
            };

            let expr = shift_by_grain(TimeExpr::Reference, minutes, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in 1/4h or 1/2h or 3/4h"
pub fn rule_in_fractional_hour() -> Rule {
    rule! {
        name: "in 1/4h or 1/2h or 3/4h",
        pattern: [re!(r"(?i)in\s+(1/4|1/2|3/4)\s*h")],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let fraction = groups.get(1)?;
            let minutes = match fraction.as_str() {
                "1/4" => 15,
                "1/2" => 30,
                "3/4" => 45,
                _ => return None,
            };

            let expr = shift_by_grain(TimeExpr::Reference, minutes, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in <number>\"" (in 30" means 30 seconds)
pub fn rule_in_number_seconds_quote() -> Rule {
    rule! {
        name: "in <number>\" (seconds)",
        pattern: [re!(r#"(?i)in\s+(\d+)\s*""#)],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount = groups.get(1)?.parse::<i32>().ok()?;
            let expr = shift_by_grain(TimeExpr::Reference, amount, Grain::Second);
            Some(expr)
        }
    }
}

/// "in <number>'" (in 30' means 30 minutes)
pub fn rule_in_number_minutes_quote() -> Rule {
    rule! {
        name: "in <number>' (minutes)",
        pattern: [re!(r"(?i)in\s+(\d+)\s*'")],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount = groups.get(1)?.parse::<i32>().ok()?;
            let expr = shift_by_grain(TimeExpr::Reference, amount, Grain::Minute);
            Some(expr)
        }
    }
}

/// "<duration> after|before|from|past <time>" (2 hours after 3pm, 1 day before Christmas)
pub fn rule_duration_after_before_time() -> Rule {
    rule! {
        name: "<duration> after|before|from|past <time>",
        pattern: [pattern_regex(duration_pattern()), re!(r"(?i)\s*(after|before|from|past)\s+"), pred!(is_time_expr)],
        optional_phrases: ["after", "before", "from", "past"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (amount, grain) = parse_duration(tokens.first()?)?;
            let relation = first_match_lower(&tokens[1..])?;
            let time_expr = get_time_expr(tokens.get(2)?)?;

            let amount = if relation.trim() == "before" { -amount } else { amount };
            let expr = shift_by_grain(time_expr.clone(), amount, grain);

            Some(expr)
        }
    }
}

/// "<text-duration> after|before|from <time>" (two hours after 3pm)
pub fn rule_text_duration_after_before_time() -> Rule {
    use crate::rules::time::predicates::is_time;

    rule! {
        name: "<text-duration> after|before|from <time>",
        pattern: [pattern_regex(text_duration_pattern()), re!(r"(?i)\s*(after|before|from)\s+"), pred!(is_time)],
        required_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (amount, grain) = parse_text_duration(tokens.first()?)?;
            let relation = first_match_lower(&tokens[1..])?;
            let time_expr = get_time_expr(tokens.get(2)?)?;

            let amount = if relation.trim() == "before" { -amount } else { amount };
            let expr = shift_by_grain(time_expr.clone(), amount, grain);

            Some(expr)
        }
    }
}

/// "<text-number> <duration> hence|ago" (two hours hence, three weeks ago)
pub fn rule_text_number_duration_hence() -> Rule {
    rule! {
        name: "<text-number> <duration> hence|ago",
        pattern: [re!(r"(?i)(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)\s+(hence|ago)")],
        required_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let number = groups.get(1)?.to_lowercase();
            let amount = match number.as_str() {
                "one" => 1,
                "two" => 2,
                "three" => 3,
                "four" => 4,
                "five" => 5,
                "six" => 6,
                "seven" => 7,
                "eight" => 8,
                "nine" => 9,
                "ten" => 10,
                "eleven" => 11,
                "twelve" => 12,
                _ => return None,
            };

            let unit = groups.get(2)?.to_lowercase();
            let grain = match unit.as_str() {
                "second" | "seconds" => Grain::Second,
                "minute" | "minutes" => Grain::Minute,
                "hour" | "hours" => Grain::Hour,
                "day" | "days" => Grain::Day,
                "week" | "weeks" => Grain::Week,
                "month" | "months" => Grain::Month,
                "year" | "years" => Grain::Year,
                _ => return None,
            };

            let relation = groups.get(3)?.trim().to_lowercase();
            let signed_amount = if relation == "ago" { -amount } else { amount };

            let shifted = shift_by_grain(TimeExpr::Reference, signed_amount, grain);
            let expr = match grain {
                Grain::Second | Grain::Minute | Grain::Hour => shifted,
                Grain::Year => TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain: Grain::Month,
                },
                _ => TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain: Grain::Day,
                },
            };
            Some(expr)
        }
    }
}

/// "<duration> hence|ago" (2 hours hence, 3 days ago)
pub fn rule_duration_hence_ago() -> Rule {
    rule! {
        name: "<duration> hence|ago",
        pattern: [pattern_regex(duration_pattern()), re!(r"(?i)\s*(hence|ago)")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (amount, grain) = parse_duration(tokens.first()?)?;
            let relation = first_match_lower(&tokens[1..])?;
            let relation = relation.trim();

            let signed_amount = if relation == "ago" { -amount } else { amount };
            let expr = shift_by_grain(TimeExpr::Reference, signed_amount, grain);
            Some(expr)
        }
    }
}

/// "a/an/one <duration> from now" (a day from now, one hour from now)
pub fn rule_a_duration_from_now() -> Rule {
    rule! {
        name: "a/an/one <duration> from now",
        pattern: [re!(r"(?i)(a|an|one)\s+(sec|second|seconds|minute|minutes|hour|hours|day|days|week|weeks|month|months|year|years)\s+from\s+now")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let unit = groups.get(2)?.to_lowercase();
            let grain = match unit.as_str() {
                "sec" | "second" | "seconds" => Grain::Second,
                "minute" | "minutes" => Grain::Minute,
                "hour" | "hours" => Grain::Hour,
                "day" | "days" => Grain::Day,
                "week" | "weeks" => Grain::Week,
                "month" | "months" => Grain::Month,
                "year" | "years" => Grain::Year,
                _ => return None,
            };

            // For day or larger grains, use start-of to round down
            let expr = if matches!(grain, Grain::Day | Grain::Week | Grain::Month | Grain::Year) {
                let shifted = shift_by_grain(TimeExpr::Reference, 1, grain);
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain: Grain::Hour,
                }
            } else {
                shift_by_grain(TimeExpr::Reference, 1, grain)
            };

            Some(expr)
        }
    }
}

/// "a/an/one <duration> from right now" (preserves exact time, no rounding)
pub fn rule_a_duration_from_right_now() -> Rule {
    rule! {
        name: "a/an/one <duration> from right now (preserve exact time)",
        pattern: [re!(r"(?i)(a|an|one)\s+(sec|second|seconds|minute|minutes|hour|hours|day|days|week|weeks|month|months|year|years)\s+from\s+right\s+now")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let unit = groups.get(2)?.to_lowercase();
            let grain = match unit.as_str() {
                "sec" | "second" | "seconds" => Grain::Second,
                "minute" | "minutes" => Grain::Minute,
                "hour" | "hours" => Grain::Hour,
                "day" | "days" => Grain::Day,
                "week" | "weeks" => Grain::Week,
                "month" | "months" => Grain::Month,
                "year" | "years" => Grain::Year,
                _ => return None,
            };

            // "right now" means preserve exact time, no rounding
            let expr = shift_by_grain(TimeExpr::Reference, 1, grain);
            Some(expr)
        }
    }
}

/// "<day> <duration> hence|ago|from now" (Monday 2 hours ago, tomorrow 3 days hence)
pub fn rule_day_duration_hence_ago() -> Rule {
    rule! {
        name: "<day> <duration> hence|ago",
        pattern: [pred!(is_time_expr), pattern_regex(duration_pattern()), re!(r"(?i)\s*(from now|hence|ago)")],
        optional_phrases: ["from now", "hence", "ago"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base = get_time_expr(tokens.first()?)?.clone();
            let (amount, grain) = parse_duration(tokens.get(1)?)?;
            let relation = first_match_lower(&tokens[2..])?;
            let relation = relation.trim();

            let signed_amount = if relation == "ago" { -amount } else { amount };
            let expr = shift_by_grain(base, signed_amount, grain);
            Some(expr)
        }
    }
}

/// "<day> in <duration>" (Monday in 2 hours, tomorrow in 3 days)
pub fn rule_day_in_duration() -> Rule {
    rule! {
        name: "<day> in <duration>",
        pattern: [pred!(is_time_expr), re!(r"(?i)\s+in\s+"), pattern_regex(duration_pattern())],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base = get_time_expr(tokens.first()?)?.clone();
            let (amount, grain) = parse_duration(tokens.get(2)?)?;

            let expr = shift_by_grain(base, amount, grain);
            Some(expr)
        }
    }
}

/// "<integer> <named-day> ago|back" (2 Mondays ago, 3 Fridays back)
pub fn rule_n_dow_ago() -> Rule {
    use chrono::Weekday;

    rule! {
        name: "<integer> <named-day> ago|back",
        pattern: [
            re!(r"(?i)(\d+)\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tue|tues|wed|thu|thur|thurs|fri|sat|sun)s?\s+(ago|back)")
        ],
        required_phrases: [],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let value = groups.get(1)?.parse::<i32>().ok()?;
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

            let base_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };

            Some(TimeExpr::Shift {
                expr: Box::new(base_expr),
                amount: -value,
                grain: Grain::Week,
            })
        }
    }
}
