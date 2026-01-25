//! Advanced time-of-day patterns including latent times and precision

use crate::time_expr::{Constraint, Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};

use crate::{
    engine::BucketMask,
    rules::numeral::helpers::first_match_lower,
    rules::numeral::predicates::number_between,
    rules::time::{helpers::shift::shift_by_grain, helpers::*, predicates::*},
};

pub fn rule_mid_day() -> Rule {
    rule! {
        name: "Mid-day",
        pattern: [re!(r"(?i)(the )?mid(\s)?day")],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let time = chrono::NaiveTime::from_hms_opt(12, 0, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        },
    }
}

pub fn rule_precision_tod() -> Rule {
    rule! {
        name: "about|exactly <time-of-day>",
        pattern: [
            re!(r"(?i)(?:at\s+)?(about|around|approximately|exactly)"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let expr = get_time_expr(tokens.get(1)?)?.clone();
            Some(expr)
        }
    }
}

pub fn rule_tod_latent() -> Rule {
    rule! {
        name: "time-of-day (latent)",
        pattern: [pred!(|t: &Token| number_between::<0, 23>(t))],
        optional_phrases: ["at", "morning", "afternoon", "evening", "night", "tonight"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n = integer_value(tokens.first()?)?;
            let time = chrono::NaiveTime::from_hms_opt(n as u32, 0, 0)?;
            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_hod_half() -> Rule {
    rule! {
        name: "<hour-of-day> half",
        pattern: [pred!(is_time_of_day_expr), re!(r"(?i)half")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.first()?, 30) }
    }
}

pub fn rule_hod_quarter() -> Rule {
    rule! {
        name: "<hour-of-day> quarter",
        pattern: [pred!(is_time_of_day_expr), re!(r"(?i)(a|one)?\s?quarter")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.first()?, 15) }
    }
}

pub fn rule_numeral_to_hod() -> Rule {
    rule! {
        name: "<integer> to|till|before <hour-of-day>",
        pattern: [pred!(|t: &Token| number_between::<1, 59>(t)), re!(r"(?i)\s*(to|till|before|of)\s+"), pred!(is_time_of_day_expr)],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let minutes = integer_value(tokens.first()?)?;
            time_expr_minutes_offset(tokens.get(2)?, -minutes)
        }
    }
}

pub fn rule_minutes_to_hod() -> Rule {
    rule! {
        name: "<integer> minutes to|till|before <hour-of-day>",
        pattern: [pred!(|t: &Token| number_between::<1, 59>(t)), re!(r"(?i)\s*minutes?\s*"), re!(r"(?i)(to|till|before|of)\s+"), pred!(is_time_of_day_expr)],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let minutes = integer_value(tokens.first()?)?;
            time_expr_minutes_offset(tokens.get(3)?, -minutes)
        }
    }
}

pub fn rule_minutes_after_hod() -> Rule {
    rule! {
        name: "<integer> minutes after|past <hour-of-day>",
        pattern: [pred!(|t: &Token| number_between::<1, 59>(t)), re!(r"(?i)\s*minutes?\s*"), re!(r"(?i)(after|past)\s+"), pred!(is_time_of_day_expr)],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let minutes = integer_value(tokens.first()?)?;
            time_expr_minutes_offset(tokens.get(3)?, minutes)
        }
    }
}

pub fn rule_numeral_after_hod() -> Rule {
    rule! {
        name: "integer after|past <hour-of-day>",
        pattern: [pred!(|t: &Token| number_between::<1, 59>(t)), re!(r"(?i)\s*(after|past)\s+"), pred!(is_time_of_day_expr)],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let minutes = integer_value(tokens.first()?)?;
            time_expr_minutes_offset(tokens.get(2)?, minutes)
        }
    }
}

pub fn rule_half_hod() -> Rule {
    rule! {
        name: "half <integer> (UK style hour-of-day)",
        pattern: [re!(r"(?i)half"), pred!(is_time_of_day_expr)],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> { time_expr_minutes_offset(tokens.get(1)?, 30) }
    }
}

pub fn rule_half_hod_words() -> Rule {
    rule! {
        name: "half <word-hour>",
        pattern: [re!(r"(?i)half\s+(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let hour_word = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let hour = parse_integer_text(hour_word)? as i64;
            let adjusted_hour = if hour < 12 { hour + 12 } else { hour };
            time_expr_with_minutes(adjusted_hour, 30, false)
        }
    }
}

pub fn rule_hhmm() -> Rule {
    rule! {
        name: "hh:mm",
        pattern: [re!(r"(?i)((?:[01]?\d)|(?:2[0-3]))[:.]([0-5]\d)")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let token = tokens.first()?;
            let h = regex_group_int_value(token, 1)?;
            let m = regex_group_int_value(token, 2)?;

            time_expr_with_minutes(h, m, false)
        }
    }
}

pub fn rule_hhhmm() -> Rule {
    rule! {
        name: "hhhmm",
        pattern: [re!(r"(?i)\b((?:[01]?\d)|(?:2[0-3]))h(([0-5]\d)?)\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let token = tokens.first()?;
            let h = regex_group_int_value(token, 1)?;
            let m = regex_group_int_value(token, 2).unwrap_or(0);

            // Special-case: avoid treating plain "1h" / "1h00" as a
            // clock-time. This lets duration rules like "in 1h" be
            // interpreted as a relative offset instead of 01:00.
            if h == 1 && m == 0 {
                return None;
            }

            time_expr_with_minutes(h, m, false)
        }
    }
}

pub fn rule_hhmm_latent() -> Rule {
    rule! {
        name: "hhmm (latent)",
        // Avoid matching 4-digit years like "2017" as 20:17.
        // Accept 3-digit times (e.g. 930) and 4-digit times only when
        // the first digit is 0 or 1 (i.e. 0000..1959).
        pattern: [re!(r"(?i)\b(?:([0-9])([0-5]\d)|([01]\d)([0-5]\d))\b")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let token = tokens.first()?;
            let h = regex_group_int_value(token, 1)
                .or_else(|| regex_group_int_value(token, 3))?;
            let m = regex_group_int_value(token, 2)
                .or_else(|| regex_group_int_value(token, 4))?;

            time_expr_with_minutes(h, m, false)
        }
    }
}

/// hhmm-ish approximate clock time (e.g., "150ish", "0930ish")
pub fn rule_hhmm_ish() -> Rule {
    rule! {
        name: "hhmm-ish",
        // Reuse the latent hhmm shape, but require an "ish" suffix.
        // Accept 3-digit times (e.g. 150ish -> 01:50) and 4-digit times only when
        // the first digit is 0 or 1 (i.e. 0000..1959), to avoid most years.
        pattern: [re!(r"(?i)\b(?:([0-9])([0-5]\d)|([01]\d)([0-5]\d))\s*ish\b")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let token = tokens.first()?;
            let h = regex_group_int_value(token, 1)
                .or_else(|| regex_group_int_value(token, 3))?;
            let m = regex_group_int_value(token, 2)
                .or_else(|| regex_group_int_value(token, 4))?;

            time_expr_with_minutes(h, m, false)
        }
    }
}

pub fn rule_hhmmss() -> Rule {
    rule! {
        name: "hh:mm:ss",
        pattern: [re!(r"(?i)((?:[01]?\d)|(?:2[0-3]))[:.]([0-5]\d)[:.]([0-5]\d)")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let token = tokens.first()?;
            let h = regex_group_int_value(token, 1)?;
            let m = regex_group_int_value(token, 2)?;
            let s = regex_group_int_value(token, 3)?;

            if !(0..24).contains(&h) || !(0..60).contains(&m) || !(0..60).contains(&s) {
                return None;
            }

            let time = chrono::NaiveTime::from_hms_opt(h as u32, m as u32, s as u32)?;
            Some(TimeExpr::Shift {
                expr: Box::new(TimeExpr::Intersect {
                    expr: Box::new(TimeExpr::Reference),
                    constraint: Constraint::TimeOfDay(time),
                }),
                amount: 0,
                grain: Grain::Second,
            })
        }
    }
}

pub fn rule_military_ampm() -> Rule {
    rule! {
        name: "hhmm (military) am|pm",
        pattern: [re!(r"(?i)((?:1[012]|0?\d))([0-5]\d)"), re!(r"(?i)([ap])\.?m?\.?")],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_token = tokens.first()?;
            let period_token = tokens.get(1)?;

            let h = regex_group_int_value(time_token, 1)?;
            let m = regex_group_int_value(time_token, 2)?;
            let ap = first_match_lower(std::slice::from_ref(period_token))?;

            let hour = if ap.contains('a') {
                if h == 12 { 0 } else { h }
            } else if h == 12 {
                12
            } else {
                h + 12
            };

            time_expr_with_minutes(hour, m, false)
        }
    }
}

pub fn rule_time_in_duration() -> Rule {
    rule! {
        name: "<time> in <duration>",
        pattern: [pred!(is_time_expr), re!(r"(?i)\s+in\s+"), re!(r"(\d+|an?)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)")],
        required_phrases: ["in"],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?;

            // Parse duration from token 2
            let groups = match &tokens.get(2)?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount_str = groups.get(1)?.to_lowercase();
            let amount = if amount_str == "a" || amount_str == "an" {
                1
            } else {
                amount_str.parse::<i32>().ok()?
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

            if let TimeExpr::Intersect { expr: inner_expr, constraint: Constraint::TimeOfDay(time) } = time_expr {
                if matches!(**inner_expr, TimeExpr::Reference) {
                    let base = match grain {
                        Grain::Month | Grain::Year => TimeExpr::StartOf {
                            expr: Box::new(TimeExpr::Reference),
                            grain: Grain::Month,
                        },
                        Grain::Week | Grain::Day => TimeExpr::StartOf {
                            expr: Box::new(TimeExpr::Reference),
                            grain: Grain::Day,
                        },
                        _ => TimeExpr::Reference,
                    };
                    let shifted = shift_by_grain(base, amount, grain);
                    Some(TimeExpr::Intersect {
                        expr: Box::new(shifted),
                        constraint: Constraint::TimeOfDay(*time),
                    })
                } else {
                    Some(TimeExpr::Shift {
                        expr: Box::new(time_expr.clone()),
                        amount,
                        grain,
                    })
                }
            } else {
                Some(TimeExpr::Shift {
                    expr: Box::new(time_expr.clone()),
                    amount,
                    grain,
                })
            }
        }
    }
}

pub fn rule_pod_this() -> Rule {
    rule! {
        name: "this <part-of-day>",
        pattern: [
            re!(r"(?i)this"),
            re!(r"(?i)\s*(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
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

pub fn rule_tod_this_pod_phrase() -> Rule {
    rule! {
        name: "<time-of-day> this <part-of-day> phrase",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s*this\s+(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
        ],
        buckets: BucketMask::empty().bits(),
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

pub fn rule_pod_at_tod() -> Rule {
    rule! {
        name: "<part-of-day> at <time-of-day>",
        pattern: [
            re!(r"(?i)(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
            re!(r"(?i)\s*(?:at|@)\s*"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let pod = first(tokens)?;
            let part = part_of_day_from_text(pod.as_str())?;
            let time = time_from_expr(tokens.get(2)?)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

pub fn rule_relative_day_pod_at_tod() -> Rule {
    rule! {
        name: "<relative-day> <part-of-day> at <time-of-day>",
        pattern: [
            re!(r"(?i)(today|tomorrow|yesterday)"),
            re!(r"(?i)\s+(early\s+morning|morning|afternoon|lunch|evening|night)\s*(?:at|@)\s*"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let rel = first(tokens)?;
            let day_expr = match rel.as_str() {
                "tomorrow" => shift_by_grain(TimeExpr::Reference, 1, Grain::Day),
                "yesterday" => shift_by_grain(TimeExpr::Reference, -1, Grain::Day),
                "today" => TimeExpr::Reference,
                _ => return None,
            };

            let pod_text = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let part = part_of_day_from_text(pod_text.as_str())?;

            let time = time_from_expr(tokens.get(2)?)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);

            Some(TimeExpr::Intersect {
                expr: Box::new(day_expr),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

pub fn rule_weekday_pod_at_tod() -> Rule {
    rule! {
        name: "<weekday> <part-of-day> at <time-of-day>",
        pattern: [
            pred!(is_weekday_name),
            re!(r"(?i)\s+(early\s+morning|morning|afternoon|lunch|evening|night)\s*(?:at|@)\s*"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday = weekday_from_name(tokens.first()?)?;

            let pod_text = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let part = part_of_day_from_text(pod_text.as_str())?;

            let time = time_from_expr(tokens.get(2)?)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);

            let weekday_expr = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::DayOfWeek(weekday),
            };

            Some(TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

pub fn rule_pod_intersect_tod_latent() -> Rule {
    rule! {
        name: "<part-of-day> <latent-time-of-day> (latent)",
        pattern: [
            re!(r"(?i)(?:early\s+morning|morning|afternoon|lunch|evening|night)"),
            pred!(is_time_of_day_expr),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let pod = first(tokens)?;
            let part = part_of_day_from_text(pod.as_str())?;
            let time = time_from_expr(tokens.get(1)?)?;
            let adjusted_time = adjust_time_for_part_of_day(time, part);

            Some(TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(adjusted_time),
            })
        }
    }
}

pub fn rule_tod_on_date() -> Rule {
    rule! {
        name: "<time-of-day> on <date>",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+on\s+"),
            pred!(is_time_expr),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time = time_from_expr(tokens.first()?)?;
            let date_expr = get_time_expr(tokens.get(2)?)?.clone();

            // Don't combine if the date already has a time-of-day constraint
            if matches!(date_expr, TimeExpr::Intersect { constraint: Constraint::TimeOfDay(_), .. }) {
                return None;
            }

            Some(TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_tod_date() -> Rule {
    rule! {
        name: "<time-of-day> <date>",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+"),
            pred!(is_time_expr),
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time = time_from_expr(tokens.first()?)?;
            let date_expr = get_time_expr(tokens.get(2)?)?.clone();

            // Don't combine if the date already has a time-of-day constraint.
            if matches!(
                date_expr,
                TimeExpr::Intersect {
                    constraint: Constraint::TimeOfDay(_),
                    ..
                }
            ) {
                return None;
            }

            Some(TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_on_date_for_tod() -> Rule {
    rule! {
        name: "on <date> for <time-of-day>",
        pattern: [
            re!(r"(?i)on\s+"),
            pred!(is_time_expr),
            re!(r"(?i)\s+(?:for|at)\s+"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let date_expr = get_time_expr(tokens.get(1)?)?.clone();
            let time = time_from_expr(tokens.get(3)?)?;

            // Don't combine if the date already has a time-of-day constraint
            if matches!(date_expr, TimeExpr::Intersect { constraint: Constraint::TimeOfDay(_), .. }) {
                return None;
            }

            Some(TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_absolute_date_tod() -> Rule {
    rule! {
        name: "<absolute-date> <time-of-day>",
        pattern: [
            pred!(is_time_expr),
            re!(r"(?i)\s+"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let date_expr = get_time_expr(tokens.first()?)?.clone();
            let TimeExpr::Absolute { hour: None, minute: None, .. } = date_expr else {
                return None;
            };

            let time = time_from_expr(tokens.get(2)?)?;

            Some(TimeExpr::Intersect {
                expr: Box::new(date_expr),
                constraint: Constraint::TimeOfDay(time),
            })
        }
    }
}

pub fn rule_at_numeral_after_hod() -> Rule {
    rule! {
        name: "at <integer> after|past <hour-of-day>",
        pattern: [
            re!(r"(?i)at\s+"),
            pred!(|t: &Token| number_between::<1, 59>(t)),
            re!(r"(?i)\s*(after|past)\s+"),
            pred!(is_time_of_day_expr),
        ],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let minutes = integer_value(tokens.get(1)?)?;
            time_expr_minutes_offset(tokens.get(3)?, minutes)
        }
    }
}

pub fn rule_one_hour_short_as_duration() -> Rule {
    rule! {
        name: "1h as duration (one hour)",
        pattern: [re!(r"(?i)\b1h\b")],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let expr = shift_by_grain(TimeExpr::Reference, 1, Grain::Hour);
            Some(expr)
        }
    }
}
