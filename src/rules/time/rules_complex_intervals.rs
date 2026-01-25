//! Complex date and time interval patterns

use crate::time_expr::{Constraint, Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};
use chrono::{NaiveTime, Timelike};

use crate::{
    engine::BucketMask,
    rules::time::{
        helpers::timezone::{LOCAL_TZ_OFFSET_HOURS, tz_offset_hours},
        helpers::*,
        predicates::*,
    },
};

fn time_of_day_constraint(expr: &TimeExpr) -> Option<Constraint> {
    match expr {
        TimeExpr::Intersect { constraint: c @ Constraint::TimeOfDay(_), .. } => Some(c.clone()),
        TimeExpr::Shift { expr, amount: 0, .. } => time_of_day_constraint(expr),
        _ => None,
    }
}

pub fn rule_interval_month_day_range_regex() -> Rule {
    rule! {
        name: "<month> <dd> - <dd> (interval, regex)",
        pattern: [
            pred!(is_month_expr),
            re!(r"\s+"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.first()?)?;
            let d1 = regex_group_int_value(tokens.get(2)?, 1)? as u32;
            let d2 = regex_group_int_value(tokens.get(4)?, 1)? as u32;
            if !(1..=31).contains(&d1) || !(1..=31).contains(&d2) || d1 >= d2 {
                return None;
            }

            // Create interval from MonthDay start to MonthDay end+1 (exclusive)
            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_from_month_day_range_regex() -> Rule {
    rule! {
        name: "from <month> <dd> - <dd> (interval, regex)",
        pattern: [
            re!(r"(?i)from"),
            re!(r"\s+"),
            pred!(is_month_expr),
            re!(r"\s+"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(2)?)?;
            let d1 = regex_group_int_value(tokens.get(4)?, 1)? as u32;
            let d2 = regex_group_int_value(tokens.get(6)?, 1)? as u32;
            if !(1..=31).contains(&d1) || !(1..=31).contains(&d2) || d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_from_dd_range_month_regex() -> Rule {
    rule! {
        name: "from <dd> - <dd> <month> (interval, regex)",
        pattern: [
            re!(r"(?i)from( the)?"),
            re!(r"\s+"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"(?i)\s*(?:\-|to( the)?|th?ru|through|(un)?til(l)?)\s*"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"\s+"),
            pred!(is_month_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(6)?)?;
            let d1 = regex_group_int_value(tokens.get(2)?, 1)? as u32;
            let d2 = regex_group_int_value(tokens.get(4)?, 1)? as u32;
            if !(1..=31).contains(&d1) || !(1..=31).contains(&d2) || d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_from_dd_range_of_month_regex() -> Rule {
    rule! {
        name: "from <dd> - <dd> of <month> (interval, regex)",
        pattern: [
            re!(r"(?i)from( the)?"),
            re!(r"\s+"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"(?i)\s*(?:\-|to( the)?|th?ru|through|(un)?til(l)?)\s*"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"\s+"),
            re!(r"(?i)of"),
            re!(r"\s+"),
            pred!(is_month_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(8)?)?;
            let d1 = regex_group_int_value(tokens.get(2)?, 1)? as u32;
            let d2 = regex_group_int_value(tokens.get(4)?, 1)? as u32;
            if !(1..=31).contains(&d1) || !(1..=31).contains(&d2) || d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_month_day_sep_month_day_regex() -> Rule {
    rule! {
        name: "<month> <dd> - <month> <dd> (interval, regex)",
        pattern: [
            pred!(is_month_expr),
            re!(r"\s+"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            pred!(is_month_expr),
            re!(r"\s+"),
            re!(r"(?i)(\d{1,2})(?:st|nd|rd|th)?"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::MONTHISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let m1 = month_from_expr(tokens.first()?)?;
            let d1 = regex_group_int_value(tokens.get(2)?, 1)? as u32;
            let m2 = month_from_expr(tokens.get(4)?)?;
            let d2 = regex_group_int_value(tokens.get(6)?, 1)? as u32;
            if !(1..=31).contains(&d1) || !(1..=31).contains(&d2) {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month: m1, day: d1 };
            let end_expr = TimeExpr::MonthDay { month: m2, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_month_dd_dd() -> Rule {
    rule! {
        name: "<month> dd-dd (interval)",
        pattern: [
            pred!(is_month_expr),
            re!(r"\s+"),
            pred!(is_day_of_month_expr),
            re!(r"(?i)\-|to|th?ru|through|(un)?til(l)?"),
            pred!(is_day_of_month_expr)
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.first()?)?;
            let d1 = day_of_month_from_expr(tokens.get(2)?)?;
            let d2 = day_of_month_from_expr(tokens.get(4)?)?;

            if d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_dd_dd_month() -> Rule {
    rule! {
        name: "dd-dd <month> (interval)",
        pattern: [
            pred!(is_day_of_month_expr),
            re!(r"(?i)\-|to|th?ru|through|(un)?til(l)?"),
            pred!(is_day_of_month_expr),
            re!(r"\s+"),
            pred!(is_month_expr)
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(4)?)?;
            let d1 = day_of_month_from_expr(tokens.first()?)?;
            let d2 = day_of_month_from_expr(tokens.get(2)?)?;

            if d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_day_to_month_day() -> Rule {
    rule! {
        name: "dd-dd <day month> (interval)",
        pattern: [
            pred!(is_day_of_month_expr),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            pred!(is_month_day_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let d1 = day_of_month_from_expr(tokens.first()?)?;
            let (month, d2) = month_day_from_expr(tokens.get(2)?)?;

            if d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_base = TimeExpr::MonthDay { month, day: d2 };
            let end_expr = TimeExpr::Shift {
                expr: Box::new(end_base),
                amount: 1,
                grain: Grain::Day,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_from_month_dd_dd() -> Rule {
    rule! {
        name: "from <month> dd-dd (interval)",
        pattern: [
            re!(r"(?i)from"),
            re!(r"\s+"),
            pred!(is_month_expr),
            re!(r"\s+"),
            pred!(is_day_of_month_expr),
            re!(r"(?i)\-|to|th?ru|through|(un)?til(l)?"),
            pred!(is_day_of_month_expr)
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(2)?)?;
            let d1 = day_of_month_from_expr(tokens.get(4)?)?;
            let d2 = day_of_month_from_expr(tokens.get(6)?)?;

            if d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_from_dd_dd_month() -> Rule {
    rule! {
        name: "from the <day-of-month> (ordinal or number) to the <day-of-month> (ordinal or number) <named-month> (interval)",
        pattern: [
            re!(r"(?i)from( the)?"),
            re!(r"\s+"),
            pred!(is_day_of_month_expr),
            re!(r"\s+"),
            re!(r"(?i)\-|to( the)?|th?ru|through|(un)?til(l)?"),
            re!(r"\s+"),
            pred!(is_day_of_month_expr),
            re!(r"\s+"),
            pred!(is_month_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(8)?)?;
            let d1 = day_of_month_from_expr(tokens.get(2)?)?;
            let d2 = day_of_month_from_expr(tokens.get(6)?)?;

            if d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_from_dd_dd_of_month() -> Rule {
    rule! {
        name: "from the <day-of-month> (ordinal or number) to the <day-of-month> (ordinal or number) of <named-month> (interval)",
        pattern: [
            re!(r"(?i)from( the)?"),
            re!(r"\s+"),
            pred!(is_day_of_month_expr),
            re!(r"\s+"),
            re!(r"(?i)\-|to( the)?|th?ru|through|(un)?til(l)?"),
            re!(r"\s+"),
            pred!(is_day_of_month_expr),
            re!(r"\s+"),
            re!(r"(?i)of"),
            re!(r"\s+"),
            pred!(is_month_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::MONTHISH | BucketMask::ORDINALISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let month = month_from_expr(tokens.get(10)?)?;
            let d1 = day_of_month_from_expr(tokens.get(2)?)?;
            let d2 = day_of_month_from_expr(tokens.get(6)?)?;

            if d1 >= d2 {
                return None;
            }

            let start_expr = TimeExpr::MonthDay { month, day: d1 };
            let end_expr = TimeExpr::MonthDay { month, day: d2 + 1 };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_year_latent() -> Rule {
    rule! {
        name: "<year> (latent) - <year> (latent) (interval)",
        pattern: [
            re!(r"(?i)(\d{4})\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*(\d{4})")
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let y1 = regex_group_int_value(tokens.first()?, 1)? as i32;
            let y2 = regex_group_int_value(tokens.first()?, 2)? as i32;

            if y1 >= y2 {
                return None;
            }

            let start_expr = TimeExpr::Absolute {
                year: y1,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            };
            let end_expr = TimeExpr::Absolute {
                year: y2 + 1,
                month: 1,
                day: 1,
                hour: None,
                minute: None,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_slash() -> Rule {
    use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

    rule! {
        name: "<datetime>/<datetime> (interval)",
        pattern: [
            re!(r"(?i)(\d{4})-(0?[1-9]|1[0-2])-(3[01]|[12]\d|0?[1-9])\s+([01]?\d|2[0-3]):([0-5]\d):([0-5]\d)"),
            re!(r"/"),
            re!(r"(?i)(\d{4})-(0?[1-9]|1[0-2])-(3[01]|[12]\d|0?[1-9])\s+([01]?\d|2[0-3]):([0-5]\d):([0-5]\d)"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_year = regex_group_int_value(tokens.first()?, 1)? as i32;
            let start_month = regex_group_int_value(tokens.first()?, 2)? as u32;
            let start_day = regex_group_int_value(tokens.first()?, 3)? as u32;
            let start_hour = regex_group_int_value(tokens.first()?, 4)? as u32;
            let start_minute = regex_group_int_value(tokens.first()?, 5)? as u32;
            let start_second = regex_group_int_value(tokens.first()?, 6)? as u32;

            let end_year = regex_group_int_value(tokens.get(2)?, 1)? as i32;
            let end_month = regex_group_int_value(tokens.get(2)?, 2)? as u32;
            let end_day = regex_group_int_value(tokens.get(2)?, 3)? as u32;
            let end_hour = regex_group_int_value(tokens.get(2)?, 4)? as u32;
            let end_minute = regex_group_int_value(tokens.get(2)?, 5)? as u32;
            let end_second = regex_group_int_value(tokens.get(2)?, 6)? as u32;

            let start_date = NaiveDate::from_ymd_opt(start_year, start_month, start_day)?;
            let start_time = NaiveTime::from_hms_opt(start_hour, start_minute, start_second)?;
            let start_dt = NaiveDateTime::new(start_date, start_time);

            let end_date = NaiveDate::from_ymd_opt(end_year, end_month, end_day)?;
            let end_time = NaiveTime::from_hms_opt(end_hour, end_minute, end_second)?;
            let end_dt = NaiveDateTime::new(end_date, end_time) + Duration::seconds(1);

            Some(TimeExpr::IntervalBetween {
                start: Box::new(TimeExpr::At(start_dt)),
                end: Box::new(TimeExpr::At(end_dt)),
            })
        }
    }
}

pub fn rule_interval_tod_dash() -> Rule {
    rule! {
        name: "<time-of-day> - <time-of-day> (interval)",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            pred!(is_time_of_day_expr),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_expr = get_time_expr(tokens.first()?)?.clone();
            let mut end_expr = get_time_expr(tokens.get(2)?)?.clone();

            // Determine grain based on precision of the times
            let start_time = time_from_expr(tokens.first()?);
            let end_time = time_from_expr(tokens.get(2)?);

            // If end time is earlier than start time (e.g., "8am to 6" where 6 is interpreted as 6am),
            // adjust it to be in the afternoon/evening (add 12 hours)
            if let (Some(st), Some(et)) = (start_time, end_time) {
                if et < st && et.hour() < 12 {
                    // End time is earlier and is in AM, shift to PM
                    let adjusted_hour = et.hour() + 12;
                    if let Some(adjusted_time) = chrono::NaiveTime::from_hms_opt(adjusted_hour, et.minute(), et.second()) {
                        end_expr = TimeExpr::Intersect {
                            expr: Box::new(TimeExpr::Reference),
                            constraint: Constraint::TimeOfDay(adjusted_time),
                        };
                    }
                }
            }

            // Check for second-level precision
            let has_seconds = start_time.map(|t| t.second() != 0).unwrap_or(false)
                           || end_time.map(|t| t.second() != 0).unwrap_or(false);

            // Check for minute-level precision
            let has_minutes = start_time.map(|t| t.minute() != 0).unwrap_or(false)
                           || end_time.map(|t| t.minute() != 0).unwrap_or(false);

            let grain = if has_seconds {
                Grain::Second
            } else if has_minutes {
                Grain::Minute
            } else {
                Grain::Hour
            };

            let end_expr = TimeExpr::Shift {
                expr: Box::new(end_expr),
                amount: 1,
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
            })
        }
    }
}

pub fn rule_interval_tod_dash_tz() -> Rule {
    rule! {
        name: "<time-of-day> - <time-of-day> (interval) timezone",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            pred!(is_time_of_day_expr),
            re!(r"\s+"),
            pattern_regex(timezone_pattern()),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_expr = get_time_expr(tokens.first()?)?.clone();
            let end_expr = get_time_expr(tokens.get(2)?)?.clone();
            let tz = first(&tokens[4..])?;

            let tz_offset = tz_offset_hours(&tz)?;
            let delta = LOCAL_TZ_OFFSET_HOURS - tz_offset;

            let start_shifted = if delta == 0 {
                start_expr
            } else {
                TimeExpr::Shift {
                    expr: Box::new(start_expr),
                    amount: delta,
                    grain: Grain::Hour,
                }
            };
            let end_shifted = if delta == 0 {
                end_expr
            } else {
                TimeExpr::Shift {
                    expr: Box::new(end_expr),
                    amount: delta,
                    grain: Grain::Hour,
                }
            };
            let end_shifted = TimeExpr::Shift {
                expr: Box::new(end_shifted),
                amount: 1,
                grain: Grain::Minute,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_shifted),
                end: Box::new(end_shifted),
            })
        }
    }
}

pub fn rule_interval_tod_tz_dash_tod_tz() -> Rule {
    rule! {
        name: "<time-of-day> <tz> - <time-of-day> <tz> (interval)",
        pattern: [
            pred!(is_time_of_day_expr),
            re!(r"\s+"),
            pattern_regex(timezone_pattern()),
            re!(r"(?i)\s*(?:\-|:|to|th?ru|through|(un)?til(l)?)\s*"),
            pred!(is_time_of_day_expr),
            re!(r"\s+"),
            pattern_regex(timezone_pattern()),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_expr = get_time_expr(tokens.first()?)?.clone();
            let start_tz = first(&tokens[2..])?;
            let end_expr = get_time_expr(tokens.get(4)?)?.clone();
            let end_tz = first(&tokens[6..])?;

            // Verify both timezones are the same
            if start_tz.to_lowercase() != end_tz.to_lowercase() {
                return None;
            }

            let tz_offset = tz_offset_hours(&start_tz)?;
            let delta = LOCAL_TZ_OFFSET_HOURS - tz_offset;

            let start_shifted = if delta == 0 {
                start_expr
            } else {
                TimeExpr::Shift {
                    expr: Box::new(start_expr),
                    amount: delta,
                    grain: Grain::Hour,
                }
            };
            let end_shifted = if delta == 0 {
                end_expr
            } else {
                TimeExpr::Shift {
                    expr: Box::new(end_expr),
                    amount: delta,
                    grain: Grain::Hour,
                }
            };
            let end_shifted = TimeExpr::Shift {
                expr: Box::new(end_shifted),
                amount: 1,
                grain: Grain::Minute,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_shifted),
                end: Box::new(end_shifted),
            })
        }
    }
}

pub fn rule_interval_tod_dash_on_weekday() -> Rule {
    rule! {
        name: "from <time-of-day> - <time-of-day> on <weekday>",
        pattern: [
            re!(r"(?i)(from\s+)?"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s*(?:\-|to|th?ru|through|(un)?til(l)?)\s*"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+on\s+"),
            pred!(is_weekday_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_tod_expr = get_time_expr(tokens.get(1)?)?.clone();
            let end_tod_expr = get_time_expr(tokens.get(3)?)?.clone();
            let weekday_expr = get_time_expr(tokens.get(5)?)?.clone();

            // Extract the TimeOfDay constraints from the time expressions
            let start_constraint = time_of_day_constraint(&start_tod_expr)?;
            let end_constraint = time_of_day_constraint(&end_tod_expr)?;

            // Create start: <weekday> at <time>
            let start = TimeExpr::Intersect {
                expr: Box::new(weekday_expr.clone()),
                constraint: start_constraint,
            };

            // Determine grain based on time precision
            let start_time = time_from_expr(tokens.get(1)?);
            let end_time = time_from_expr(tokens.get(3)?);
            let has_seconds = start_time.map(|t| t.second() != 0).unwrap_or(false)
                           || end_time.map(|t| t.second() != 0).unwrap_or(false);
            let has_minutes = start_time.map(|t| t.minute() != 0).unwrap_or(false)
                           || end_time.map(|t| t.minute() != 0).unwrap_or(false);
            let grain = if has_seconds { Grain::Second } else if has_minutes { Grain::Minute } else { Grain::Hour };

            // Create end: <weekday> at <time> + 1 unit
            let end_base = TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: end_constraint,
            };

            let end = TimeExpr::Shift {
                expr: Box::new(end_base),
                amount: 1,
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_between_tod_and_tod_on_weekday() -> Rule {
    rule! {
        name: "between <time-of-day> and <time-of-day> on <weekday>",
        pattern: [
            re!(r"(?i)between\s+"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+and\s+"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+on\s+"),
            pred!(is_weekday_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_tod_expr = get_time_expr(tokens.get(1)?)?.clone();
            let end_tod_expr = get_time_expr(tokens.get(3)?)?.clone();
            let weekday_expr = get_time_expr(tokens.get(5)?)?.clone();

            // Extract the TimeOfDay constraints from the time expressions
            let start_constraint = time_of_day_constraint(&start_tod_expr)?;
            let end_constraint = time_of_day_constraint(&end_tod_expr)?;

            // Create start: <weekday> at <time>
            let start = TimeExpr::Intersect {
                expr: Box::new(weekday_expr.clone()),
                constraint: start_constraint,
            };

            // Determine grain based on time precision
            let start_time = time_from_expr(tokens.get(1)?);
            let end_time = time_from_expr(tokens.get(3)?);
            let has_seconds = start_time.map(|t| t.second() != 0).unwrap_or(false)
                           || end_time.map(|t| t.second() != 0).unwrap_or(false);
            let has_minutes = start_time.map(|t| t.minute() != 0).unwrap_or(false)
                           || end_time.map(|t| t.minute() != 0).unwrap_or(false);
            let grain = if has_seconds { Grain::Second } else if has_minutes { Grain::Minute } else { Grain::Hour };

            // Create end: <weekday> at <time> + 1 unit
            let end_base = TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: end_constraint,
            };

            let end = TimeExpr::Shift {
                expr: Box::new(end_base),
                amount: 1,
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_later_than_tod_but_before_tod() -> Rule {
    rule! {
        name: "later than <time-of-day> but before <time-of-day>",
        pattern: [
            re!(r"(?i)later\s+than\s+"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+but\s+before\s+"),
            pred!(is_time_of_day_expr),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_expr = get_time_expr(tokens.get(1)?)?.clone();
            let end_expr = get_time_expr(tokens.get(3)?)?.clone();

            let grain = grain_of_time_expr(&end_expr);
            let end = TimeExpr::Shift {
                expr: Box::new(end_expr),
                amount: 1,
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_later_than_tod_but_before_tod_on_weekday() -> Rule {
    rule! {
        name: "later than <time-of-day> but before <time-of-day> on <weekday>",
        pattern: [
            re!(r"(?i)later\s+than\s+"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+but\s+before\s+"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+on\s+"),
            pred!(is_weekday_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_tod_expr = get_time_expr(tokens.get(1)?)?.clone();
            let end_tod_expr = get_time_expr(tokens.get(3)?)?.clone();
            let weekday_expr = get_time_expr(tokens.get(5)?)?.clone();

            // Extract the TimeOfDay constraints from the time expressions
            let start_constraint = time_of_day_constraint(&start_tod_expr)?;
            let end_constraint = time_of_day_constraint(&end_tod_expr)?;

            // Create start: <weekday> at <time>
            let start = TimeExpr::Intersect {
                expr: Box::new(weekday_expr.clone()),
                constraint: start_constraint,
            };

            // Determine grain based on time precision
            let start_time = time_from_expr(tokens.get(1)?);
            let end_time = time_from_expr(tokens.get(3)?);
            let has_seconds = start_time.map(|t| t.second() != 0).unwrap_or(false)
                           || end_time.map(|t| t.second() != 0).unwrap_or(false);
            let has_minutes = start_time.map(|t| t.minute() != 0).unwrap_or(false)
                           || end_time.map(|t| t.minute() != 0).unwrap_or(false);
            let grain = if has_seconds { Grain::Second } else if has_minutes { Grain::Minute } else { Grain::Hour };

            // Create end: <weekday> at <time> + 1 unit
            let end_base = TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: end_constraint,
            };

            let end = TimeExpr::Shift {
                expr: Box::new(end_base),
                amount: 1,
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_weekday_from_tod_to_tod() -> Rule {
    rule! {
        name: "<weekday> from <time-of-day> to <time-of-day>",
        pattern: [
            pred!(is_weekday_expr),
            re!(r"(?i)\s+from\s+"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+(?:to|(?:un)?til(?:l)?)\s+"),
            pred!(is_time_of_day_expr),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday_expr = get_time_expr(tokens.first()?)?.clone();
            let start_tod_expr = get_time_expr(tokens.get(2)?)?.clone();
            let end_tod_expr = get_time_expr(tokens.get(4)?)?.clone();

            // Extract the TimeOfDay constraints from the time expressions
            let start_constraint = time_of_day_constraint(&start_tod_expr)?;
            let end_constraint = time_of_day_constraint(&end_tod_expr)?;

            // Create start: <weekday> at <time>
            let start = TimeExpr::Intersect {
                expr: Box::new(weekday_expr.clone()),
                constraint: start_constraint,
            };

            // Determine grain based on time precision
            let start_time = time_from_expr(tokens.get(2)?);
            let end_time = time_from_expr(tokens.get(4)?);
            let has_seconds = start_time.map(|t| t.second() != 0).unwrap_or(false)
                           || end_time.map(|t| t.second() != 0).unwrap_or(false);
            let has_minutes = start_time.map(|t| t.minute() != 0).unwrap_or(false)
                           || end_time.map(|t| t.minute() != 0).unwrap_or(false);
            let grain = if has_seconds { Grain::Second } else if has_minutes { Grain::Minute } else { Grain::Hour };

            // Create end: <weekday> at <time> + 1 unit
            let end_base = TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: end_constraint,
            };

            let end = TimeExpr::Shift {
                expr: Box::new(end_base),
                amount: 1,
                grain,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_hour_dash_hour_ampm() -> Rule {
    rule! {
        name: "<hour>-<hour> am|pm",
        pattern: [
            re!(r"(?i)(?:(?:from|around)\s+)?(\d{1,2})\s*(?:\-|to)\s*(\d{1,2})\s*(?:in\s+the\s+)?([ap])\.?m?\.?"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_hour = regex_group_int_value(tokens.first()?, 1)? as i64;
            let end_hour = regex_group_int_value(tokens.first()?, 2)? as i64;

            let ap_group = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(3).cloned(),
                _ => None,
            }?;

            let is_pm = ap_group.to_lowercase().starts_with('p');

            let start_hour_24 = if is_pm && start_hour < 12 {
                start_hour + 12
            } else if !is_pm && start_hour == 12 {
                0
            } else {
                start_hour
            };

            let end_hour_24 = if is_pm && end_hour < 12 {
                end_hour + 12
            } else if !is_pm && end_hour == 12 {
                0
            } else {
                end_hour
            };

            let start_time = NaiveTime::from_hms_opt(start_hour_24 as u32, 0, 0)?;
            // For hour-only intervals, the end extends through the entire hour,
            // so we add 1 hour to make it inclusive
            let end_time = NaiveTime::from_hms_opt(((end_hour_24 + 1) % 24) as u32, 0, 0)?;

            let start = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(start_time),
            };

            let end = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(end_time),
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_weekday_hour_dash_hour_ampm() -> Rule {
    rule! {
        name: "<weekday> <hour>-<hour> am|pm",
        pattern: [
            pred!(is_weekday_expr),
            re!(r"(?i)\s+(?:(?:from|around)\s+)?(\d{1,2})\s*(?:\-|to)\s*(\d{1,2})\s*(?:in\s+the\s+)?([ap])\.?m?\.?"),
        ],
        buckets: (BucketMask::HAS_DIGITS | BucketMask::HAS_COLON | BucketMask::WEEKDAYISH).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let weekday_expr = get_time_expr(tokens.first()?)?.clone();

            let start_hour = regex_group_int_value(tokens.get(1)?, 1)? as i64;
            let end_hour = regex_group_int_value(tokens.get(1)?, 2)? as i64;

            let ap_group = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(3).cloned(),
                _ => None,
            }?;

            let is_pm = ap_group.to_lowercase().starts_with('p');

            let start_hour_24 = if is_pm && start_hour < 12 {
                start_hour + 12
            } else if !is_pm && start_hour == 12 {
                0
            } else {
                start_hour
            };

            let end_hour_24 = if is_pm && end_hour < 12 {
                end_hour + 12
            } else if !is_pm && end_hour == 12 {
                0
            } else {
                end_hour
            };

            let start_time = NaiveTime::from_hms_opt(start_hour_24 as u32, 0, 0)?;
            // For hour-only intervals, the end extends through the entire hour
            let end_time = NaiveTime::from_hms_opt(((end_hour_24 + 1) % 24) as u32, 0, 0)?;

            let start = TimeExpr::Intersect {
                expr: Box::new(weekday_expr.clone()),
                constraint: Constraint::TimeOfDay(start_time),
            };

            let end = TimeExpr::Intersect {
                expr: Box::new(weekday_expr),
                constraint: Constraint::TimeOfDay(end_time),
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start),
                end: Box::new(end),
            })
        }
    }
}

pub fn rule_interval_tod_to_word_hour_ampm() -> Rule {
    rule! {
        name: "<time-of-day> to <word-hour> am|pm",
        pattern: [
            re!(r"(?i)(?:from\s+)?"),
            pred!(is_time_of_day_expr),
            re!(r"(?i)\s+(?:to|(?:un)?til(?:l)?)\s+(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+([ap])\.?\s?m?\.?"),
        ],
        buckets: (BucketMask::HAS_COLON).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let start_expr = get_time_expr(tokens.get(1)?)?.clone();

            let (hour_word, am_pm) = match &tokens.get(2)?.kind {
                TokenKind::RegexMatch(groups) => (groups.get(1)?, groups.get(2)?),
                _ => return None,
            };

            let hour = parse_integer_text(hour_word)? as i64;
            let is_pm = am_pm.to_lowercase().starts_with('p');

            let hour_24 = if is_pm {
                if hour == 12 { 12 } else { hour + 12 }
            } else if hour == 12 { 0 } else { hour };

            let end_time = NaiveTime::from_hms_opt(hour_24 as u32, 0, 0)?;
            let end_base = TimeExpr::Intersect {
                expr: Box::new(TimeExpr::Reference),
                constraint: Constraint::TimeOfDay(end_time),
            };

            let end = TimeExpr::Shift {
                expr: Box::new(end_base),
                amount: 1,
                grain: Grain::Minute,
            };

            Some(TimeExpr::IntervalBetween {
                start: Box::new(start_expr),
                end: Box::new(end),
            })
        }
    }
}
