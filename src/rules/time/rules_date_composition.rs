//! Date/month/day combinations, ordinal patterns, and formatting

use crate::time_expr::{Constraint, TimeExpr};
use crate::{Pattern, Rule, Token, TokenKind};

use crate::{
    engine::BucketMask,
    rules::time::{helpers::*, predicates::*},
};

pub fn rule_at_word_hour_minute() -> Rule {
    rule! {
        name: "at <word-hour> <word-minute>",
        pattern: [re!(r"(?i)at\s+(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+(ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fifty)")],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (hour_word, minute_word) = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => (groups.get(1)?, groups.get(2)?),
                _ => return None,
            };
            let hour = parse_integer_text(hour_word)? as u32;
            let minute = parse_integer_text(minute_word)? as u32;

            // Use AmbiguousTime which will be resolved to next occurrence during normalization
            Some(TimeExpr::AmbiguousTime { hour, minute })
        }
    }
}
pub fn rule_word_hour_minute() -> Rule {
    rule! {
        name: "<word-hour> <word-minute>",
        pattern: [re!(r"(?i)\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)(?:\s+|-)\b(ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fifty)\b")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (hour_word, minute_word) = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => (groups.get(1)?, groups.get(2)?),
                _ => return None,
            };
            let hour = parse_integer_text(hour_word)? as u32;
            let minute = parse_integer_text(minute_word)? as u32;

            // Use AmbiguousTime which will be resolved to next occurrence during normalization
            Some(TimeExpr::AmbiguousTime { hour, minute })
        }
    }
}
pub fn rule_word_hour_minute_tens_units() -> Rule {
    rule! {
        name: "<word-hour> <word-minute-tens> <word-minute-units>",
        pattern: [re!(r"(?i)\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)(?:\s+|-)(twenty|thirty|forty|fifty)(?:\s+|-)(one|two|three|four|five|six|seven|eight|nine)\b")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (hour_word, tens_word, units_word) = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => (groups.get(1)?, groups.get(2)?, groups.get(3)?),
                _ => return None,
            };

            let hour = parse_integer_text(hour_word)? as u32;
            let minute_tens = parse_integer_text(tens_word)? as u32;
            let minute_units = parse_integer_text(units_word)? as u32;
            let minute = minute_tens + minute_units;

            Some(TimeExpr::AmbiguousTime { hour, minute })
        }
    }
}

pub fn rule_word_hour_zero_minute() -> Rule {
    rule! {
        name: "<word-hour> zero/oh <minute-unit>",
        pattern: [re!(r"(?i)\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)(?:\s+|-)\b(zero|o|oh|ou)(?:\s+|-)\b(one|two|three|four|five|six|seven|eight|nine)\b")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (hour_word, zero_word, unit_word) = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => (groups.get(1)?, groups.get(2)?, groups.get(3)?),
                _ => return None,
            };
            let hour = parse_integer_text(hour_word)? as i64;
            let _zero = parse_integer_text(zero_word)? as i64; // forces zero/o/oh/ou to be recognized
            let units = parse_integer_text(unit_word)? as i64;
            let minute = units; // "zero three" -> 03
            time_expr_with_minutes(hour, minute, false)
        }
    }
}

pub fn rule_time_expr_at_time_of_day() -> Rule {
    rule! {
        name: "<time> <time-of-day>",
        pattern: [pred!(is_time_expr), re!(r"(?i)\s+"), pred!(is_time_of_day_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?.clone();
            let time_of_day = time_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::TimeOfDay(time_of_day),
            })
        }
    }
}

pub fn rule_time_expr_explicit_at_time_of_day() -> Rule {
    use crate::rules::time::predicates::is_time;

    rule! {
        name: "<time> at <time-of-day>",
        pattern: [pred!(is_time), re!(r"(?i)\s+at\s+"), pred!(is_time_of_day_expr)],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Get time expression - works with both TimeExpr and TimeData
            let time_expr = get_time_expr(tokens.first()?)?. clone();
            let time_of_day = time_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::TimeOfDay(time_of_day),
            })
        }
    }
}

pub fn rule_at_time_on_time() -> Rule {
    use crate::rules::time::predicates::is_time;

    rule! {
        name: "at <time-of-day> <time>",
        pattern: [re!(r"(?i)at\s+"), pred!(is_time_of_day_expr), re!(r"(?i)\s+"), pred!(is_time)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_of_day = time_from_expr(tokens.get(1)?)?;
            let time_expr = get_time_expr(tokens.get(3)?)?.clone();

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::TimeOfDay(time_of_day),
            })
        }
    }
}

pub fn rule_ordinal_words_day_of_month() -> Rule {
    rule! {
        name: "ordinal words (day of month)",
        pattern: [re!(r"(?i)\b(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth|thirteenth|fourteenth|fifteenth|sixteenth|seventeenth|eighteenth|nineteenth|twentieth|twenty-first|twenty-second|twenty-third|twenty-fourth|twenty-fifth|twenty-sixth|twenty-seventh|twenty-eighth|twenty-ninth|thirtieth|thirty-first)\b")],
        buckets: (BucketMask::HAS_COLON | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1).or_else(|| groups.first())?.to_lowercase(),
                _ => return None,
            };

            let day = match ordinal.as_str() {
                "first" => 1,
                "second" => 2,
                "third" => 3,
                "fourth" => 4,
                "fifth" => 5,
                "sixth" => 6,
                "seventh" => 7,
                "eighth" => 8,
                "ninth" => 9,
                "tenth" => 10,
                "eleventh" => 11,
                "twelfth" => 12,
                "thirteenth" => 13,
                "fourteenth" => 14,
                "fifteenth" => 15,
                "sixteenth" => 16,
                "seventeenth" => 17,
                "eighteenth" => 18,
                "nineteenth" => 19,
                "twentieth" => 20,
                "twenty-first" => 21,
                "twenty-second" => 22,
                "twenty-third" => 23,
                "twenty-fourth" => 24,
                "twenty-fifth" => 25,
                "twenty-sixth" => 26,
                "twenty-seventh" => 27,
                "twenty-eighth" => 28,
                "twenty-ninth" => 29,
                "thirtieth" => 30,
                "thirty-first" => 31,
                _ => return None,
            };

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfMonth(day),
            })
        }
    }
}

pub fn rule_ordinal_day_of_month_expr() -> Rule {
    rule! {
        name: "<day-of-month> of <month>",
        pattern: [
            pred!(is_day_of_month_numeral),
            re!(r"(?i)\s+of\s+"),
            pred!(is_month_expr)
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.first()?)?;
            let month = month_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_day_of_month_of_time() -> Rule {
    rule! {
        name: "<day-of-month> of <time>",
        pattern: [
            pred!(is_day_of_month_numeral),
            re!(r"(?i)\s+(?:day\s+)?of( the)?\s+"),
            pred!(is_time_expr),
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {

            let day = day_of_month_from_expr(tokens.first()?)?;
            let time_expr = get_time_expr(tokens.get(2)?)?.clone();

            Some(TimeExpr::Intersect {
                expr: Box::new(time_expr),
                constraint: Constraint::DayOfMonth(day),
            })
        }
    }
}

pub fn rule_the_ordinal_day_of_month() -> Rule {
    rule! {
        name: "the <day-of-month> of <month>",
        pattern: [
            re!(r"(?i)the\s+"),
            pred!(is_day_of_month_numeral),
            re!(r"(?i)\s+of\s+"),
            pred!(is_month_expr)
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.get(1)?)?;
            let month = month_from_expr(tokens.get(3)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_month_the_ordinal_day() -> Rule {
    rule! {
        name: "<month> the <day-of-month>",
        pattern: [
            pred!(is_month_expr),
            re!(r"(?i)\s+the\s+"),
            pred!(is_day_of_month_numeral)
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.first()?)?;
            let day = day_of_month_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_ordinal_day_month() -> Rule {
    rule! {
        name: "<day-of-month> <month>",
        pattern: [pred!(is_day_of_month_numeral), re!(r"\s+"), pred!(is_month_expr)],
        buckets: (BucketMask::ORDINALISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.first()?)?;
            let month = month_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_day_month_no_space() -> Rule {
    rule! {
        name: "<day><month> (no space)",
        pattern: [pred!(is_day_of_month_numeral), pred!(is_month_expr)],
        buckets: (BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = day_of_month_from_expr(tokens.first()?)?;
            let month = month_from_expr(tokens.get(1)?)?;

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_dd_month_no_space_regex() -> Rule {
    rule! {
        name: "ddmonth (no space, regex)",
        pattern: [re!(r"(?i)([1-9]|[12]\d|3[01])(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = regex_group_int_value(tokens.first()?, 1)? as u32;
            let month_name = match tokens.first() {
                Some(Token { kind: TokenKind::RegexMatch(groups), .. }) => groups.get(2).cloned(),
                _ => None,
            }?;

            let month = match month_name.to_lowercase().as_str() {
                "january" | "jan" => 1,
                "february" | "feb" => 2,
                "march" | "mar" => 3,
                "april" | "apr" => 4,
                "may" => 5,
                "june" | "jun" => 6,
                "july" | "jul" => 7,
                "august" | "aug" => 8,
                "september" | "sep" | "sept" => 9,
                "october" | "oct" => 10,
                "november" | "nov" => 11,
                "december" | "dec" => 12,
                _ => return None,
            };

            if !(1..=31).contains(&day) {
                return None;
            }

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_month_day_no_space_regex() -> Rule {
    rule! {
        name: "monthdd (no space, regex)",
        pattern: [re!(r"(?i)(january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)([1-9]|[12]\d|3[01])")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let day = regex_group_int_value(tokens.first()?, 2)? as u32;
            let month_name = match tokens.first() {
                Some(Token { kind: TokenKind::RegexMatch(groups), .. }) => groups.get(1).cloned(),
                _ => None,
            }?;

            let month = match month_name.to_lowercase().as_str() {
                "january" | "jan" => 1,
                "february" | "feb" => 2,
                "march" | "mar" => 3,
                "april" | "apr" => 4,
                "may" => 5,
                "june" | "jun" => 6,
                "july" | "jul" => 7,
                "august" | "aug" => 8,
                "september" | "sep" | "sept" => 9,
                "october" | "oct" => 10,
                "november" | "nov" => 11,
                "december" | "dec" => 12,
                _ => return None,
            };

            Some(TimeExpr::MonthDay { month, day })
        }
    }
}

pub fn rule_weekday_comma_month_day() -> Rule {
    rule! {
        name: "<weekday>, <month-day>",
        pattern: [pred!(is_weekday_expr), re!(r",\s*"), pred!(is_month_day_expr)],
        buckets: (BucketMask::HAS_COLON | BucketMask::WEEKDAYISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_expr(tokens.first()?)?;
            let (month, day) = month_day_from_expr(tokens.get(2)?)?;

            // Create an Intersect expression with DayOfWeek constraint
            // This will be normalized to find the next occurrence of month/day that matches the weekday
            let month_day_expr = TimeExpr::MonthDay { month, day };
            Some(TimeExpr::Intersect {
                expr: Box::new(month_day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

pub fn rule_weekday_comma_month_day_no_space() -> Rule {
    rule! {
        name: "<weekday>, <month><day>",
        pattern: [re!(r"(?i)(monday|mon|tuesday|tue|tues|wednesday|wed|thursday|thu|thurs|friday|fri|saturday|sat|sunday|sun),\s*(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)(\d{1,2})")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };
            let weekday_key = groups.get(1)?.to_lowercase();
            let month_key = groups.get(2)?.to_lowercase();
            let day = groups.get(3)?.parse::<u32>().ok()?;

            let weekday = match weekday_key.as_str() {
                "monday" | "mon" => chrono::Weekday::Mon,
                "tuesday" | "tue" | "tues" => chrono::Weekday::Tue,
                "wednesday" | "wed" => chrono::Weekday::Wed,
                "thursday" | "thu" | "thurs" => chrono::Weekday::Thu,
                "friday" | "fri" => chrono::Weekday::Fri,
                "saturday" | "sat" => chrono::Weekday::Sat,
                "sunday" | "sun" => chrono::Weekday::Sun,
                _ => return None,
            };

            let month = match month_key.as_str() {
                "january" | "jan" => 1,
                "february" | "feb" => 2,
                "march" | "mar" => 3,
                "april" | "apr" => 4,
                "may" => 5,
                "june" | "jun" => 6,
                "july" | "jul" => 7,
                "august" | "aug" => 8,
                "september" | "sep" | "sept" => 9,
                "october" | "oct" => 10,
                "november" | "nov" => 11,
                "december" | "dec" => 12,
                _ => return None,
            };

            let month_day_expr = TimeExpr::MonthDay { month, day };
            Some(TimeExpr::Intersect {
                expr: Box::new(month_day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

pub fn rule_weekday_month_day() -> Rule {
    rule! {
        name: "<weekday> <month-day> (no comma)",
        pattern: [pred!(is_weekday_expr), pred!(is_month_day_expr)],
        buckets: (BucketMask::HAS_COLON | BucketMask::WEEKDAYISH | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_expr(tokens.first()?)?;
            let (month, day) = month_day_from_expr(tokens.get(1)?)?;

            let month_day_expr = TimeExpr::MonthDay { month, day };
            Some(TimeExpr::Intersect {
                expr: Box::new(month_day_expr),
                constraint: Constraint::DayOfWeek(weekday),
            })
        }
    }
}

pub fn rule_month() -> Rule {
    rule! {
        name: "named-month",
        // Shared month regex pattern constant
        pattern: [
            Pattern::Regex(&MONTH_PATTERN_REGEX),
        ],
        buckets: (BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month_match = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.first()?.as_str(),
                _ => return None,
            };
            let month_key = month_match.to_lowercase();
            let month = MONTH_NAME.get(month_key.as_str())?;

            // Represent a month reference as the Reference intersected with the month,
            // which normalizes to the start of that month.
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::Month(*month),
            })
        }
    }
}
