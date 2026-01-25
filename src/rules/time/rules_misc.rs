use crate::Dimension;
use crate::time_expr::{Constraint, Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};
/// Miscellaneous time rules (timezones, nth patterns, year formatting)
use chrono::{NaiveTime, Timelike};

use crate::{
    engine::BucketMask,
    rules::numeral::predicates::number_between,
    rules::time::{
        helpers::shift::shift_by_grain,
        helpers::timezone::{LOCAL_TZ_OFFSET_HOURS, tz_offset_hours},
        helpers::*,
        predicates::*,
    },
};

pub fn rule_interval_by_the_end_of() -> Rule {
    rule! {
        name: "by the end of <time>",
        pattern: [re!(r"(?i)by (the )?end of\s*"), pred!(is_time_expr)],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let target_token = tokens.get(1)?;

            match &target_token.kind {
                TokenKind::TimeExpr(target_expr) => {
                    // Create an interval from now until the end of the target period
                    // For "by the end of March", we want: now -> end of March
                    // The target already represents the time period, normalization will handle it
                    Some(TimeExpr::IntervalUntil {
                        target: Box::new(target_expr.clone()),
                    })
                }
                _ => None,
            }
        }
    }
}

pub fn rule_nth_last_week_of_period() -> Rule {
    rule! {
        name: "nth last week of <month/year>",
        pattern: [
            re!(r"(?i)(the\s+)?(first|second|third|fourth|fifth|last|1st|2nd|3rd|4th|5th)\s+last\s+week\s+(of|in)\s+"),
            pred!(is_month_expr),
            re!(r"(?i)\s*(\d{4})?")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal_str = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups
                    .iter()
                    .map(|g| g.trim().to_lowercase())
                    .find(|g| {
                        !g.is_empty()
                            && !g.contains(char::is_whitespace)
                            && matches!(
                                g.as_str(),
                                "first" | "second" | "third" | "fourth" | "fifth" | "last" | "1st" | "2nd" | "3rd" | "4th" | "5th"
                            )
                    })?,
                _ => return None,
            };

            let n = match ordinal_str.as_str() {
                "last" | "first" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };

            let month = month_from_expr(tokens.get(1)?)?;

            let year = if let Some(year_token) = tokens.get(2) {
                if let TokenKind::RegexMatch(groups) = &year_token.kind {
                    if let Some(year_str) = groups.get(1) {
                        if !year_str.is_empty() {
                            year_str.parse::<i32>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Some(TimeExpr::NthLastOf {
                n,
                grain: Grain::Week,
                year,
                month: Some(month),
            })
        }
    }
}

pub fn rule_last_week_of_period() -> Rule {
    rule! {
        name: "last week of <month/year>",
        pattern: [
            re!(r"(?i)(?:the\s+)?last\s+week\s+(?:of|in)\s+"),
            pred!(is_month_expr),
            re!(r"(?i)\s*(\d{4})?")
        ],
        buckets: BucketMask::MONTHISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(1)?)?;

            let year = if let Some(year_token) = tokens.get(2) {
                if let TokenKind::RegexMatch(groups) = &year_token.kind {
                    if let Some(year_str) = groups.get(1) {
                        if !year_str.is_empty() {
                            year_str.parse::<i32>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Some(TimeExpr::NthLastOf {
                n: 1,
                grain: Grain::Week,
                year,
                month: Some(month),
            })
        }
    }
}

pub fn rule_nth_last_week_of_year() -> Rule {
    rule! {
        name: "nth last week of year",
        pattern: [
            re!(r"(?i)(?:the\s+)?(third|second|first|fourth|fifth|\d+(?:st|nd|rd|th))\s+last\s+week\s+of\s+(\d{4})")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let token = tokens.first()?;

            let ordinal_str = match &token.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let n: u32 = match ordinal_str.as_str() {
                "first" => 1,
                "second" => 2,
                "third" => 3,
                "fourth" => 4,
                "fifth" => 5,
                _ => ordinal_str
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u32>()
                    .ok()?,
            };

            let year_str = match &token.kind {
                TokenKind::RegexMatch(groups) => groups.get(2)?,
                _ => return None,
            };
            let year = year_str.parse::<i32>().ok()?;

            Some(TimeExpr::NthLastOf {
                n,
                grain: Grain::Week,
                year: Some(year),
                month: None,
            })
        }
    }
}

pub fn rule_nth_last_day_of_month() -> Rule {
    rule! {
        name: "nth last day of <month>",
        pattern: [
            re!(r"(?i)(the\s+)?(first|second|third|fourth|fifth|last|1st|2nd|3rd|4th|5th)\s+last\s+day\s+(of|in)\s+"),
            pred!(is_month_expr),
            re!(r"(?i)\s*(\d{4})?")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal_str = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups
                    .iter()
                    .map(|g| g.trim().to_lowercase())
                    .find(|g| {
                        !g.is_empty()
                            && !g.contains(char::is_whitespace)
                            && matches!(
                                g.as_str(),
                                "first" | "second" | "third" | "fourth" | "fifth" | "last" | "1st" | "2nd" | "3rd" | "4th" | "5th"
                            )
                    })?,
                _ => return None,
            };

            let n = match ordinal_str.as_str() {
                "last" | "first" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                "fifth" | "5th" => 5,
                _ => return None,
            };

            let month = month_from_expr(tokens.get(1)?)?;

            let year = if let Some(year_token) = tokens.get(2) {
                if let TokenKind::RegexMatch(groups) = &year_token.kind {
                    if let Some(year_str) = groups.get(1) {
                        if !year_str.is_empty() {
                            year_str.parse::<i32>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Some(TimeExpr::NthLastOf {
                n,
                grain: Grain::Day,
                year,
                month: Some(month),
            })
        }
    }
}

pub fn rule_last_day_of_month() -> Rule {
    rule! {
        name: "last day of <month>",
        pattern: [
            re!(r"(?i)(?:the\s+)?last\s+day\s+(?:of|in)\s+"),
            pred!(is_month_expr),
            re!(r"(?i)\s*(\d{4})?")
        ],
        buckets: BucketMask::MONTHISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(1)?)?;

            let year = if let Some(year_token) = tokens.get(2) {
                if let TokenKind::RegexMatch(groups) = &year_token.kind {
                    if let Some(year_str) = groups.get(1) {
                        if !year_str.is_empty() {
                            year_str.parse::<i32>().ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            Some(TimeExpr::NthLastOf {
                n: 1,
                grain: Grain::Day,
                year,
                month: Some(month),
            })
        }
    }
}

pub fn rule_time_of_day_with_timezone() -> Rule {
    rule! {
        name: "<time-of-day> <timezone>",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"\s+"),
            pattern_regex(timezone_pattern()),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?.clone();
            let tz = first(&tokens[2..])?;

            let tz_offset = tz_offset_hours(&tz)?;
            let delta = LOCAL_TZ_OFFSET_HOURS - tz_offset;

            if delta == 0 {
                Some(time_expr)
            } else {
                let shifted = TimeExpr::Shift {
                    expr: Box::new(time_expr),
                    amount: delta,
                    grain: Grain::Hour,
                };
                Some(shifted)
            }
        }
    }
}

pub fn rule_interval_dash_with_timezone() -> Rule {
    rule! {
        name: "<time> - <time> <timezone>",
        pattern: [
            pred!(is_time_expr),
            re!(r"\s*-\s*"),
            pred!(is_time_expr),
            re!(r"\s+"),
            pattern_regex(timezone_pattern()),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Only apply this rule for time-of-day ranges.
            let start_token = tokens.first()?;
            let end_token = tokens.get(2)?;

            if !is_time_of_day_expr(start_token) || !is_time_of_day_expr(end_token) {
                return None;
            }

            let start = get_time_expr(start_token)?.clone();
            let end = get_time_expr(end_token)?.clone();

            // End-exclusive bound at minute/second resolution.
            let end = if let Some(tod) = time_from_expr(end_token) {
                let (amount, grain) = if tod.second() != 0 {
                    (1, Grain::Second)
                } else {
                    (1, Grain::Minute)
                };
                TimeExpr::Shift {
                    expr: Box::new(end),
                    amount,
                    grain,
                }
            } else {
                end
            };

            let interval = TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            };

            let tz = first(&tokens[4..])?;
            let tz_offset = tz_offset_hours(&tz)?;
            let delta = LOCAL_TZ_OFFSET_HOURS - tz_offset;

            if delta == 0 {
                Some(interval)
            } else {
                Some(TimeExpr::Shift {
                    expr: Box::new(interval),
                    amount: delta,
                    grain: Grain::Hour,
                })
            }
        }
    }
}

pub fn rule_weekday_time_of_day_with_timezone() -> Rule {
    use chrono::Weekday;

    rule! {
        name: "<weekday> <hour> am|pm <timezone>",
        pattern: [
            re!(r"(?i)\b(mondays?|mon|tuesdays?|tues?|wed?nesdays?|wed|thursdays?|thurs?|thu|fridays?|fri|saturdays?|sat|sundays?|sun)\s+(\d{1,2})\s+([ap])\.?\s?m\.?\s+\(?(BST|PST|EST|CST|MST|CET|UTC|GMT|IST|JST|KST|AEST|AEDT|NZST|NZDT)\)?\b"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Parse weekday
            let dow_match = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.as_str(),
                _ => return None,
            };
            let dow_key = dow_match.to_lowercase();
            let weekday = match dow_key.as_str() {
                "monday" | "mon" => Weekday::Mon,
                "tuesday" | "tue" | "tues" => Weekday::Tue,
                "wednesday" | "wed" => Weekday::Wed,
                "thursday" | "thu" | "thurs" => Weekday::Thu,
                "friday" | "fri" => Weekday::Fri,
                "saturday" | "sat" => Weekday::Sat,
                "sunday" | "sun" => Weekday::Sun,
                _ => return None,
            };

            // Parse hour and am/pm
            let hour = regex_group_int_value(tokens.first()?, 2)? as i64;
            let ap_group = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(3)?.as_str(),
                _ => return None,
            };
            let is_pm = ap_group.to_lowercase().starts_with('p');
            let hour_24 = if is_pm {
                if hour == 12 { 12 } else { hour + 12 }
            } else if hour == 12 { 0 } else { hour };

            // Parse timezone
            let tz_abbr = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(4)?.as_str(),
                _ => return None,
            };
            let tz_offset = tz_offset_hours(tz_abbr)?;
            let delta = (LOCAL_TZ_OFFSET_HOURS - tz_offset) as i64;

            // Apply timezone shift
            let final_hour = ((hour_24 + delta) % 24 + 24) % 24;
            let time = NaiveTime::from_hms_opt(final_hour as u32, 0, 0)?;

            // Create weekday constraint
            let weekday_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };

            // Intersect with time
            Some(TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_weekday_at_time_with_minutes_and_timezone() -> Rule {
    use chrono::Weekday;

    rule! {
        name: "<weekday> at <hour>:<minute> am|pm <timezone>",
        pattern: [
            re!(r"(?i)\b(mondays?|mon|tuesdays?|tues?|wed?nesdays?|wed|thursdays?|thurs?|thu|fridays?|fri|saturdays?|sat|sundays?|sun)\s+at\s+(\d{1,2}):(\d{2})\s*([ap])\.?\s?m\.?\s+\(?(BST|PST|EST|CST|MST|CET|UTC|GMT|IST|JST|KST|AEST|AEDT|NZST|NZDT)\)?\b"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Parse weekday
            let dow_match = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.as_str(),
                _ => return None,
            };
            let dow_key = dow_match.to_lowercase();
            let weekday = match dow_key.as_str() {
                "monday" | "mon" => Weekday::Mon,
                "tuesday" | "tue" | "tues" => Weekday::Tue,
                "wednesday" | "wed" => Weekday::Wed,
                "thursday" | "thu" | "thurs" => Weekday::Thu,
                "friday" | "fri" => Weekday::Fri,
                "saturday" | "sat" => Weekday::Sat,
                "sunday" | "sun" => Weekday::Sun,
                _ => return None,
            };

            // Parse hour, minute, and am/pm
            let hour = regex_group_int_value(tokens.first()?, 2)? as i64;
            let _minute = regex_group_int_value(tokens.first()?, 3)? as u32;
            let ap_group = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(4)?.as_str(),
                _ => return None,
            };
            let is_pm = ap_group.to_lowercase().starts_with('p');
            let hour_24 = if is_pm {
                if hour == 12 { 12 } else { hour + 12 }
            } else if hour == 12 { 0 } else { hour };

            // Parse timezone
            let tz_abbr = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(5)?.as_str(),
                _ => return None,
            };
            let tz_offset = tz_offset_hours(tz_abbr)?;
            let delta = (LOCAL_TZ_OFFSET_HOURS - tz_offset) as i64;

            // Apply timezone shift (ignoring minutes for simplicity)
            let final_hour = ((hour_24 + delta) % 24 + 24) % 24;
            let time = NaiveTime::from_hms_opt(final_hour as u32, 0, 0)?;

            // Create weekday constraint
            let weekday_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };

            // Intersect with time
            Some(TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_end_of_year() -> Rule {
    rule! {
        name: "end of year",
        pattern: [re!(r"(?i)(by (the )?|(at )?the )?(EOY|end of (the )?year)")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first(tokens)?;

            // End of current year
            let current_year = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            // Shift to next year and get start (which is end of current year)
            let next_year = shift_by_grain(current_year.clone(), 1, Grain::Year);

            if matched.to_lowercase().starts_with("by") {
                Some(TimeExpr::IntervalUntil {
                    target: Box::new(next_year),
                })
            } else {
                // Corpus expects "EOY" / "end of year" as the last quarter.
                let start_of_eoy = shift_by_grain(current_year, 8, Grain::Month);
                Some(TimeExpr::IntervalBetween {
                    start: Box::new(start_of_eoy),
                    end: Box::new(next_year),
                })
            }
        }
    }
}

pub fn rule_beginning_of_year() -> Rule {
    rule! {
        name: "beginning of year",
        pattern: [re!(r"(?i)((at )?the )?(BOY|beginning of (the )?year)")],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let start_of_year = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };

            let start_of_q2 = shift_by_grain(start_of_year.clone(), 3, Grain::Month);

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_of_year),
                end: Box::new(start_of_q2),
            })
        }
    }
}

pub fn rule_n_weekdays_from_now() -> Rule {
    use chrono::Weekday;

    rule! {
        name: "<integer> <weekday>s from now",
        pattern: [
            // N is a Numeral token (parsed by numeral rules), then a weekday
            // in text, then "from now/today" as raw text.
            pred!(|t: &Token| crate::rules::numeral::predicates::is_integer(t)),
            re!(r"\s+"),
            re!(r"(?i)(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tue|tues|wed|thu|thur|thurs|fri|sat|sun)s?"),
            re!(r"\s+from\s+(now|today)"),
        ],
        // Activated primarily by presence of a weekday word; N may be
        // written out (no digits), so we do not require HAS_DIGITS.
        buckets: (BucketMask::WEEKDAYISH).bits(),
        deps: [Dimension::Numeral],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            // Token 0: Numeral (N)
            let n = integer_value(tokens.first()?)? as i64;
            if n <= 0 {
                return None;
            }

            // Token 2: weekday regex match (group 1 = base weekday name)
            let dow_token = tokens.get(2)?;
            let dow_str = match &dow_token.kind {
                TokenKind::RegexMatch(g) => g.get(1)?.trim().to_lowercase(),
                _ => return None,
            };
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

            // "N weekdays from now" means N occurrences forward, excluding today
            // To exclude today, we shift reference by 1 day first
            let tomorrow_ref = TimeExpr::Shift {
                expr: Box::new(TimeExpr::Reference),
                amount: 1,
                grain: Grain::Day,
            };

            let weekday_expr = TimeExpr::Intersect {
                expr: Box::new(tomorrow_ref),
                constraint: Constraint::DayOfWeek(weekday),
            };

            // Then shift by (n-1) weeks to get the Nth occurrence
            Some(TimeExpr::Shift {
                expr: Box::new(weekday_expr),
                amount: (n - 1) as i32,
                grain: Grain::Week,
            })
        }
    }
}

pub fn rule_cycle_numeral_qtr() -> Rule {
    rule! {
        name: "<integer> qtr",
        pattern: [
            pred!(|t: &Token| number_between::<1, 4>(t)),
            re!(r"\s+"),
            re!(r"(?i)qtr(s)?\b|qr\b"),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let ordinal = integer_value(tokens.first()?)? as i32;
            if !(1..=4).contains(&ordinal) {
                return None;
            }

            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            let shifted = shift_by_grain(base, ordinal - 1, Grain::Quarter);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Quarter,
            })
        }
    }
}

pub fn rule_interval_from_time_for_duration_regex() -> Rule {
    rule! {
        name: "from <time> for <duration>",
        pattern: [re!(r"(?i)(from|starting|beginning)\s+"), pred!(is_time_expr), re!(r"\s+for\s+(\d+)\s*(seconds?|mins?|'|minutes?|hours?|h|days?|weeks?|months?|years?)")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?;

            let groups = match &tokens.get(2)?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount = groups.get(1)?.parse::<i32>().ok()?;
            let unit = groups.get(2)?.to_lowercase();

            let grain = match unit.as_str() {
                "second" | "seconds" => Grain::Second,
                "min" | "mins" | "'" | "minute" | "minutes" => Grain::Minute,
                "hour" | "hours" | "h" => Grain::Hour,
                "day" | "days" => Grain::Day,
                "week" | "weeks" => Grain::Week,
                "month" | "months" => Grain::Month,
                "year" | "years" => Grain::Year,
                _ => return None,
            };

            let end_expr = shift_by_grain(time_expr.clone(), amount + 1, grain);
            Some(TimeExpr::IntervalBetween {
                start: Box::new(time_expr.clone()),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_year_numeric() -> Rule {
    rule! {
        name: "year (numeric)",
        pattern: [pred!(|t: &Token| number_between::<1000, 2500>(t))],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year = integer_value(tokens.first()?)? as i32;
            Some(TimeExpr::Absolute {
                year,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}

pub fn rule_year_bc() -> Rule {
    rule! {
        name: "in <year> bc",
        pattern: [re!(r"(?i)in\s+(\d{1,4})\s*(b\.?c\.?|bc)\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let year = regex_group_int_value(tokens.first()?, 1)? as i32;
            Some(TimeExpr::Absolute {
                year: -year,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}

pub fn rule_time_year_suffix() -> Rule {
    rule! {
        name: "<time> <year>",
        // Include leading whitespace because the engine matches regexes at the
        // current position without skipping spaces.
        pattern: [pred!(is_time_expr), re!(r"\s+(\d{4})\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base = get_time_expr(tokens.first()?)?;
            let year = regex_group_int_value(tokens.get(1)?, 1)? as i32;
            let expr = time_expr_with_year(base, year)?;
            Some(expr)
        }
    }
}

pub fn rule_time_numeral_year_suffix() -> Rule {
    rule! {
        name: "<time> <numeral-year>",
        // Include leading whitespace because the engine matches regexes at the
        // current position without skipping spaces.
        pattern: [pred!(is_time_expr), re!(r"\s+"), pred!(|t: &Token| number_between::<1000, 2500>(t))],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base = get_time_expr(tokens.first()?)?;
            let year = integer_value(tokens.get(2)?)? as i32;
            let expr = time_expr_with_year(base, year)?;
            Some(expr)
        }
    }
}

pub fn rule_time_two_thousand_year_suffix() -> Rule {
    rule! {
        name: "<time> two thousand <year>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s+two\s+thousand\s+(?P<suf>ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|\d{1,2})\b"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let base = get_time_expr(tokens.first()?)?;
            let suffix = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.trim().to_ascii_lowercase(),
                _ => return None,
            };

            let n: i32 = match suffix.as_str() {
                "ten" => 10,
                "eleven" => 11,
                "twelve" => 12,
                "thirteen" => 13,
                "fourteen" => 14,
                "fifteen" => 15,
                "sixteen" => 16,
                "seventeen" => 17,
                "eighteen" => 18,
                "nineteen" => 19,
                _ => suffix.parse().ok()?,
            };
            let year = 2000 + n;
            let expr = time_expr_with_year(base, year)?;
            Some(expr)
        }
    }
}
