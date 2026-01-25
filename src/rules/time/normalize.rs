use crate::time_expr::{Constraint, Grain, Holiday, MonthPart, PartOfDay, Season, TimeExpr, TimeValue};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::rules::time::helpers::boundaries::{interval_of, start_of};
use crate::rules::time::helpers::shift::shift_datetime_by_grain;

pub fn normalize(expr: &TimeExpr, reference: NaiveDateTime) -> Option<TimeValue> {
    match expr {
        TimeExpr::Reference => Some(TimeValue::Instant(reference)),
        TimeExpr::At(dt) => Some(TimeValue::Instant(*dt)),
        TimeExpr::Interval { start, end } => Some(TimeValue::Interval { start: *start, end: *end }),
        TimeExpr::Shift { expr, amount, grain } => {
            if *amount == 0 {
                return normalize(expr.as_ref(), reference);
            }
            if *amount == -1 && *grain == Grain::Week {
                if let TimeExpr::Intersect { expr: inner_expr, constraint: Constraint::DayOfWeek(target_dow) } =
                    expr.as_ref()
                {
                    if matches!(**inner_expr, TimeExpr::Reference) && reference.weekday() == *target_dow {
                        let date = reference.date().checked_sub_signed(Duration::days(7))?;
                        let dt = date.and_hms_opt(0, 0, 0)?;
                        return Some(TimeValue::Instant(dt));
                    }
                }
            }
            if *amount == 1 && *grain == Grain::Week {
                if let TimeExpr::Intersect { expr: inner_expr, constraint: Constraint::DayOfWeek(target_dow) } =
                    expr.as_ref()
                {
                    if matches!(**inner_expr, TimeExpr::Reference) && reference.weekday() == *target_dow {
                        return normalize(expr, reference);
                    }
                }
            }

            // Special handling for Shift of NthWeekdayOfMonth/LastWeekdayOfMonth by Month/Year grain
            // For holidays, "thanksgiving in 9 months" should mean "the thanksgiving ~9 months from now",
            // not "shift thanksgiving date by 9 months". So we shift the reference and find the holiday.
            if matches!(*grain, Grain::Month | Grain::Year) {
                match expr.as_ref() {
                    TimeExpr::Holiday { .. }
                    | TimeExpr::NthWeekdayOfMonth { .. }
                    | TimeExpr::LastWeekdayOfMonth { .. } => {
                        // Shift the reference time by the amount, then find the holiday
                        let shifted_reference = shift_datetime_by_grain(reference, *amount, *grain);
                        return normalize(expr, shifted_reference);
                    }
                    _ => {}
                }
            }

            // Special handling for Shift of NthWeekdayOfMonth/LastWeekdayOfMonth by Year grain
            // We need to adjust the year BEFORE normalization, not shift the result after
            if *grain == Grain::Year {
                match expr.as_ref() {
                    TimeExpr::NthWeekdayOfMonth { n, year, month, weekday } => {
                        if *amount == -1 && year.is_none() {
                            // Two cases:
                            // 1. "last <holiday>" means most recent past occurrence
                            // 2. "<holiday> of last year" means reference.year() - 1
                            // We'll use a heuristic: if the base year's occurrence is in the future,
                            // it's likely "last <holiday>" (use previous year)
                            // If it's in the past, it's likely "of last year" (use previous year unconditionally)
                            // Actually, both cases want previous year, so just do reference.year() - 1

                            // Wait, "last MLK day" should be most recent PAST, not always previous year
                            // Let me check: reference is Feb 12, 2013. MLK 2013 is Jan 21 (past).
                            // "last MLK day" should be Jan 21, 2013 (most recent past)
                            // "MLK day of last year" should be Jan 16, 2012 (previous year)

                            // To distinguish, I need more info. Let me use a different marker.
                            // For now, let's make "last" mean most recent past:
                            let current_year_expr = TimeExpr::NthWeekdayOfMonth {
                                n: *n,
                                year: Some(reference.year()),
                                month: *month,
                                weekday: *weekday,
                            };
                            if let Some(TimeValue::Instant(dt)) = normalize(&current_year_expr, reference) {
                                if dt.date() < reference.date() {
                                    // Current year's occurrence is in the past, use it
                                    return Some(TimeValue::Instant(dt));
                                } else {
                                    // Current year's occurrence is in the future, use previous year
                                    let prev_year_expr = TimeExpr::NthWeekdayOfMonth {
                                        n: *n,
                                        year: Some(reference.year() - 1),
                                        month: *month,
                                        weekday: *weekday,
                                    };
                                    return normalize(&prev_year_expr, reference);
                                }
                            }
                        } else {
                            // Regular year shift: adjust the year and normalize
                            let new_year = year.map(|y| y + amount).or_else(|| Some(reference.year() + amount));
                            let adjusted_expr =
                                TimeExpr::NthWeekdayOfMonth { n: *n, year: new_year, month: *month, weekday: *weekday };
                            return normalize(&adjusted_expr, reference);
                        }
                    }
                    TimeExpr::LastWeekdayOfMonth { year, month, weekday } => {
                        let new_year = year.map(|y| y + amount).or_else(|| Some(reference.year() + amount));
                        let adjusted_expr =
                            TimeExpr::LastWeekdayOfMonth { year: new_year, month: *month, weekday: *weekday };
                        return normalize(&adjusted_expr, reference);
                    }
                    _ => {}
                }
            }

            match normalize(expr, reference)? {
                TimeValue::Instant(dt) => Some(TimeValue::Instant(shift_datetime_by_grain(dt, *amount, *grain))),
                TimeValue::Interval { start, end } => Some(TimeValue::Interval {
                    start: shift_datetime_by_grain(start, *amount, *grain),
                    end: shift_datetime_by_grain(end, *amount, *grain),
                }),
                TimeValue::OpenAfter(dt) => Some(TimeValue::OpenAfter(shift_datetime_by_grain(dt, *amount, *grain))),
                TimeValue::OpenBefore(dt) => Some(TimeValue::OpenBefore(shift_datetime_by_grain(dt, *amount, *grain))),
            }
        }
        TimeExpr::StartOf { expr, grain } => match normalize(expr, reference)? {
            TimeValue::Instant(dt) => Some(TimeValue::Instant(start_of(*grain, dt))),
            TimeValue::Interval { start, .. } => Some(TimeValue::Instant(start_of(*grain, start))),
            TimeValue::OpenAfter(dt) => Some(TimeValue::OpenAfter(start_of(*grain, dt))),
            TimeValue::OpenBefore(dt) => Some(TimeValue::OpenBefore(start_of(*grain, dt))),
        },
        TimeExpr::IntervalOf { expr, grain } => match normalize(expr, reference)? {
            TimeValue::Instant(dt) => Some(interval_of(*grain, dt)),
            TimeValue::Interval { start, .. } => Some(interval_of(*grain, start)),
            TimeValue::OpenAfter(dt) => Some(interval_of(*grain, dt)),
            TimeValue::OpenBefore(dt) => Some(interval_of(*grain, dt)),
        },
        TimeExpr::Intersect { expr, constraint } => {
            // Special case: MonthDay + DayOfWeek constraint
            // We need to find the next year where month/day falls on the target weekday
            if let (TimeExpr::MonthDay { month, day }, Constraint::DayOfWeek(target_dow)) = (expr.as_ref(), constraint)
            {
                return normalize_month_day_with_weekday(*month, *day, *target_dow, reference);
            }
            if let (
                TimeExpr::Intersect { expr: inner_expr, constraint: Constraint::DayOfMonth(day) },
                Constraint::DayOfWeek(target_dow),
            ) = (expr.as_ref(), constraint)
            {
                if matches!(**inner_expr, TimeExpr::Reference) {
                    return normalize_day_of_month_with_weekday(*day, *target_dow, reference);
                }
            }
            if let (
                TimeExpr::Intersect { expr: inner_expr, constraint: Constraint::DayOfWeek(target_dow) },
                Constraint::DayOfMonth(day),
            ) = (expr.as_ref(), constraint)
            {
                if matches!(**inner_expr, TimeExpr::Reference) {
                    return normalize_day_of_month_with_weekday(*day, *target_dow, reference);
                }
            }

            let base_value = normalize(expr, reference)?;
            apply_constraint(base_value, constraint, reference)
        }
        TimeExpr::MonthPart { month, part } => {
            let target_month = month.unwrap_or_else(|| reference.month());
            month_part_interval(target_month, *part, reference)
        }
        TimeExpr::IntervalUntil { target } => {
            // Create an interval from the reference time (now) until the target time
            let target_value = normalize(target, reference)?;
            match target_value {
                TimeValue::Instant(end_dt) => Some(TimeValue::Interval { start: reference, end: end_dt }),
                TimeValue::Interval { end, .. } => {
                    // If the target is an interval, use its end as our end
                    Some(TimeValue::Interval { start: reference, end })
                }
                TimeValue::OpenAfter(end_dt) | TimeValue::OpenBefore(end_dt) => {
                    Some(TimeValue::Interval { start: reference, end: end_dt })
                }
            }
        }
        TimeExpr::IntervalBetween { start, end } => {
            // Special handling for year-crossing MonthDay intervals
            // e.g., "this winter" = Dec 21 to Mar 21 crosses years
            if let (
                TimeExpr::MonthDay { month: start_month, day: start_day },
                TimeExpr::MonthDay { month: end_month, day: end_day },
            ) = (start.as_ref(), end.as_ref())
            {
                // If start month > end month, this crosses a year boundary
                if start_month > end_month {
                    // For year-crossing intervals, start should be in the past year if it hasn't occurred yet
                    let mut start_year = reference.year();
                    let start_candidate = NaiveDate::from_ymd_opt(start_year, *start_month, *start_day)?;

                    // If the start date hasn't passed yet this year, use last year for start
                    if start_candidate >= reference.date() {
                        start_year -= 1;
                    }

                    // End is always in the following year from start
                    let end_year = start_year + 1;

                    let start_date = NaiveDate::from_ymd_opt(start_year, *start_month, *start_day)?;
                    let end_date = NaiveDate::from_ymd_opt(end_year, *end_month, *end_day)?;

                    return Some(TimeValue::Interval {
                        start: NaiveDateTime::new(start_date, chrono::NaiveTime::from_hms_opt(0, 0, 0)?),
                        end: NaiveDateTime::new(end_date, chrono::NaiveTime::from_hms_opt(0, 0, 0)?),
                    });
                }
            }

            // Create an interval between two time expressions
            let start_value = normalize(start, reference)?;
            let end_value = normalize(end, reference)?;

            let start_dt = match start_value {
                TimeValue::Instant(dt) => dt,
                TimeValue::Interval { start, .. } => start,
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => dt,
            };

            let end_dt = match end_value {
                TimeValue::Instant(dt) => dt,
                TimeValue::Interval { end, .. } => end,
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => dt,
            };

            Some(TimeValue::Interval { start: start_dt, end: end_dt })
        }
        TimeExpr::OpenAfter { expr } => {
            let value = normalize(expr, reference)?;
            match value {
                TimeValue::Instant(dt) => Some(TimeValue::OpenAfter(dt)),
                TimeValue::Interval { start, .. } => Some(TimeValue::OpenAfter(start)),
                TimeValue::OpenAfter(dt) => Some(TimeValue::OpenAfter(dt)),
                TimeValue::OpenBefore(dt) => Some(TimeValue::OpenAfter(dt)),
            }
        }
        TimeExpr::OpenBefore { expr } => {
            let value = normalize(expr, reference)?;
            match value {
                TimeValue::Instant(dt) => Some(TimeValue::OpenBefore(dt)),
                TimeValue::Interval { end, .. } => Some(TimeValue::OpenBefore(end)),
                TimeValue::OpenAfter(dt) => Some(TimeValue::OpenBefore(dt)),
                TimeValue::OpenBefore(dt) => Some(TimeValue::OpenBefore(dt)),
            }
        }
        TimeExpr::MonthDay { month, day } => {
            // Pick the next occurrence of this month/day
            let mut year = reference.year();
            let mut candidate = NaiveDate::from_ymd_opt(year, *month, *day)?;

            // If the date has passed this year, use next year
            if candidate < reference.date() {
                year += 1;
                candidate = NaiveDate::from_ymd_opt(year, *month, *day)?;
            }

            Some(TimeValue::Instant(NaiveDateTime::new(candidate, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)))
        }
        TimeExpr::ClosestWeekdayTo { n, weekday, target } => {
            let n = (*n).max(1) as i64;

            let target_dt = match normalize(target.as_ref(), reference)? {
                TimeValue::Instant(dt) => dt,
                TimeValue::Interval { start, .. } => start,
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => dt,
            };

            let target_date = target_dt.date();

            // Find the target weekday on-or-before the target date.
            let mut base = target_date;
            for _ in 0..7 {
                if base.weekday() == *weekday {
                    break;
                }
                base = base.pred_opt()?;
            }
            if base.weekday() != *weekday {
                return None;
            }

            // Generate enough candidates around the target date, then pick the nth by distance.
            let mut candidates: Vec<NaiveDate> = Vec::new();
            let span_weeks = n + 2;
            for k in 0..=span_weeks {
                let days = Duration::days(7 * k);
                if let Some(d) = base.checked_sub_signed(days) {
                    candidates.push(d);
                }
                if let Some(d) = base.checked_add_signed(days) {
                    candidates.push(d);
                }
            }
            candidates.sort();
            candidates.dedup();

            candidates.sort_by_key(|d| {
                let offset = (*d - target_date).num_days();
                let abs = offset.abs();
                // Tie-break: prefer future (offset >= 0) over past.
                (abs, if offset >= 0 { 0 } else { 1 }, offset)
            });

            let idx = (n - 1) as usize;
            let chosen = *candidates.get(idx)?;
            Some(TimeValue::Instant(NaiveDateTime::new(chosen, NaiveTime::from_hms_opt(0, 0, 0)?)))
        }
        TimeExpr::Absolute { year, month, day, hour, minute } => {
            let date = NaiveDate::from_ymd_opt(*year, *month, *day)?;
            let time = chrono::NaiveTime::from_hms_opt(hour.unwrap_or(0), minute.unwrap_or(0), 0)?;
            Some(TimeValue::Instant(NaiveDateTime::new(date, time)))
        }
        TimeExpr::LastWeekdayOfMonth { year, month, weekday } => {
            use chrono::Datelike;

            // Determine which year to use
            let target_year = year.unwrap_or_else(|| reference.year());

            // Find the last day of the target month
            let last_day_of_month = if *month == 12 {
                NaiveDate::from_ymd_opt(target_year + 1, 1, 1)?.pred_opt()?
            } else {
                NaiveDate::from_ymd_opt(target_year, month + 1, 1)?.pred_opt()?
            };

            // Work backwards from the last day of the month to find the last occurrence of the weekday
            let mut current = last_day_of_month;
            for _ in 0..7 {
                // At most 7 days to check
                if current.weekday() == *weekday {
                    return Some(TimeValue::Instant(NaiveDateTime::new(
                        current,
                        chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                    )));
                }
                current = current.pred_opt()?;
            }

            None
        }
        TimeExpr::FirstWeekdayOfMonth { year, month, weekday } => {
            use chrono::Datelike;

            // Determine which year to use
            let target_year = year.unwrap_or_else(|| reference.year());

            // Find the first day of the target month
            let first_day_of_month = NaiveDate::from_ymd_opt(target_year, *month, 1)?;

            // Work forwards from the first day to find the first occurrence of the weekday
            let mut current = first_day_of_month;
            for _ in 0..7 {
                // At most 7 days to check
                if current.weekday() == *weekday {
                    return Some(TimeValue::Instant(NaiveDateTime::new(
                        current,
                        chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                    )));
                }
                current = current.succ_opt()?;
            }

            None
        }
        TimeExpr::NthWeekdayOfMonth { n, year, month, weekday } => {
            use chrono::Datelike;

            // Find the nth occurrence of a specific weekday in a month
            // For example, 4th Thursday of November for Thanksgiving
            // Special marker: year=Some(-1) means "last year" (reference.year() - 1)
            let mut target_year = match year {
                Some(-1) => reference.year() - 1,
                Some(y) => *y,
                None => reference.year(),
            };
            let mut first_day_of_month = NaiveDate::from_ymd_opt(target_year, *month, 1)?;

            // Find the first occurrence of the target weekday
            let mut current = first_day_of_month;
            for _ in 0..7 {
                if current.weekday() == *weekday {
                    break;
                }
                current = current.succ_opt()?;
            }

            // Now jump forward by (n-1) weeks to get the nth occurrence
            if *n == 0 || *n > 5 {
                return None; // Invalid, months have at most 5 occurrences of a weekday
            }

            current = current.checked_add_signed(chrono::Duration::weeks((*n - 1) as i64))?;

            // Verify we're still in the same month
            if current.month() != *month {
                return None;
            }

            // If no year was specified and the date has passed, use next year
            if year.is_none() && current < reference.date() {
                target_year += 1;
                first_day_of_month = NaiveDate::from_ymd_opt(target_year, *month, 1)?;
                current = first_day_of_month;
                for _ in 0..7 {
                    if current.weekday() == *weekday {
                        break;
                    }
                    current = current.succ_opt()?;
                }
                current = current.checked_add_signed(chrono::Duration::weeks((*n - 1) as i64))?;

                if current.month() != *month {
                    return None;
                }
            }

            Some(TimeValue::Instant(NaiveDateTime::new(current, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)))
        }
        TimeExpr::NthWeekOf { n, year, month } => {
            use chrono::Datelike;

            let target_year = year.unwrap_or_else(|| reference.year());

            if let Some(target_month) = month {
                // Nth week of a specific month
                // Find the first Monday that falls within the month
                let first_day = NaiveDate::from_ymd_opt(target_year, *target_month, 1)?;

                // Find the first Monday in the month
                let first_day_dow = first_day.weekday();
                let days_until_monday = if first_day_dow == chrono::Weekday::Mon {
                    0
                } else {
                    (7 - first_day_dow.num_days_from_monday()) % 7
                };

                let first_monday = first_day + chrono::Duration::days(days_until_monday as i64);

                // Add (n-1) weeks to get to the nth Monday (which represents the nth week)
                let target_week_start = first_monday + chrono::Duration::weeks((*n as i64) - 1);

                Some(TimeValue::Instant(NaiveDateTime::new(
                    target_week_start,
                    chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                )))
            } else {
                // Nth week of a year - not implemented yet
                None
            }
        }
        TimeExpr::NthLastOf { n, grain, year, month } => {
            use chrono::Datelike;

            let target_year = year.unwrap_or_else(|| reference.year());

            match grain {
                Grain::Week => {
                    if let Some(target_month) = month {
                        // Nth-last week of a month
                        // Count *full* weeks (Mon..Sun) contained in the month.
                        // The week containing the last day of the month may be partial and
                        // should not be counted as the "last week of the month".
                        // Find the last day of the month
                        let last_day = if *target_month == 12 {
                            NaiveDate::from_ymd_opt(target_year + 1, 1, 1)?.pred_opt()?
                        } else {
                            NaiveDate::from_ymd_opt(target_year, target_month + 1, 1)?.pred_opt()?
                        };

                        // Find the last Sunday in the month, then the Monday 6 days earlier.
                        let last_day_dow = last_day.weekday();
                        let days_since_sunday = last_day_dow.num_days_from_sunday() as i64;
                        let last_sunday = last_day - chrono::Duration::days(days_since_sunday);
                        let last_monday = last_sunday - chrono::Duration::days(6);

                        // Go back (n-1) full weeks.
                        let target_monday = last_monday - chrono::Duration::weeks((*n as i64) - 1);

                        Some(TimeValue::Instant(NaiveDateTime::new(
                            target_monday,
                            chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                        )))
                    } else {
                        // Nth-last week of a year
                        let last_day_of_year = NaiveDate::from_ymd_opt(target_year, 12, 31)?;
                        let last_day_dow = last_day_of_year.weekday();

                        // Count *full* weeks (Mon..Sun) contained in the year.
                        // If Dec 31 is not a Sunday, the week containing Dec 31 is partial and
                        // should not be counted as the "last week of the year".
                        let days_since_sunday = last_day_dow.num_days_from_sunday() as i64;
                        let last_sunday = last_day_of_year - chrono::Duration::days(days_since_sunday);
                        let last_monday = last_sunday - chrono::Duration::days(6);

                        let target_monday = last_monday - chrono::Duration::weeks((*n as i64) - 1);

                        Some(TimeValue::Instant(NaiveDateTime::new(
                            target_monday,
                            chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                        )))
                    }
                }
                Grain::Day => {
                    if let Some(target_month) = month {
                        // Nth-last day of a month
                        let last_day = if *target_month == 12 {
                            NaiveDate::from_ymd_opt(target_year + 1, 1, 1)?.pred_opt()?
                        } else {
                            NaiveDate::from_ymd_opt(target_year, target_month + 1, 1)?.pred_opt()?
                        };

                        let target_day = last_day - chrono::Duration::days((*n as i64) - 1);

                        Some(TimeValue::Instant(NaiveDateTime::new(
                            target_day,
                            chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                        )))
                    } else {
                        // Nth-last day of a year
                        let last_day_of_year = NaiveDate::from_ymd_opt(target_year, 12, 31)?;
                        let target_day = last_day_of_year - chrono::Duration::days((*n as i64) - 1);

                        Some(TimeValue::Instant(NaiveDateTime::new(
                            target_day,
                            chrono::NaiveTime::from_hms_opt(0, 0, 0)?,
                        )))
                    }
                }
                _ => None,
            }
        }
        // Holiday normalization
        TimeExpr::Holiday { holiday, year } => normalize_holiday(*holiday, *year, reference),
        TimeExpr::Season(season) => normalize_season(*season, reference),
        TimeExpr::SeasonPeriod { offset } => normalize_season_period(*offset, reference),
        TimeExpr::PartOfDay(part_of_day) => {
            // Apply part of day constraint to today
            apply_part_of_day_to_reference(*part_of_day, reference)
        }
        TimeExpr::After(expr) => {
            // Open-ended interval starting from expr
            let value = normalize(expr, reference)?;
            match value {
                TimeValue::Instant(dt) => Some(TimeValue::OpenAfter(dt)),
                TimeValue::Interval { start, .. } => Some(TimeValue::OpenAfter(start)),
                TimeValue::OpenAfter(dt) => Some(TimeValue::OpenAfter(dt)),
                TimeValue::OpenBefore(dt) => Some(TimeValue::OpenAfter(dt)),
            }
        }
        TimeExpr::Before(expr) => {
            // Open-ended interval ending at expr
            let value = normalize(expr, reference)?;
            match value {
                TimeValue::Instant(dt) => Some(TimeValue::OpenBefore(dt)),
                TimeValue::Interval { end, .. } => Some(TimeValue::OpenBefore(end)),
                TimeValue::OpenAfter(dt) => Some(TimeValue::OpenBefore(dt)),
                TimeValue::OpenBefore(dt) => Some(TimeValue::OpenBefore(dt)),
            }
        }
        TimeExpr::Duration(expr) => {
            // Duration expressions should be normalized within their context
            // For now, treat as instant
            normalize(expr, reference)
        }
        TimeExpr::AmbiguousTime { hour, minute } => {
            // Find the next occurrence of this time (could be AM or PM)
            // Try both AM and PM versions and pick the next one after reference
            let hour_am = if *hour == 12 { 0 } else { *hour };
            let hour_pm = if *hour == 12 { 12 } else { hour + 12 };

            let time_am = NaiveTime::from_hms_opt(hour_am, *minute, 0)?;
            let time_pm = NaiveTime::from_hms_opt(hour_pm, *minute, 0)?;

            // Check which occurrence is next
            let today = reference.date();
            let am_today = NaiveDateTime::new(today, time_am);
            let pm_today = NaiveDateTime::new(today, time_pm);

            // Find the next occurrence
            let next_time = if am_today > reference {
                am_today
            } else if pm_today > reference {
                pm_today
            } else {
                // Both have passed today, use AM tomorrow
                let tomorrow = today.succ_opt()?;
                NaiveDateTime::new(tomorrow, time_am)
            };

            Some(TimeValue::Instant(next_time))
        }
    }
}

fn month_part_bounds(year: i32, month: u32, part: MonthPart) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let (start_day, end_date) = match part {
        MonthPart::Early => {
            let start = NaiveDate::from_ymd_opt(year, month, 1)?;
            let end = NaiveDate::from_ymd_opt(year, month, 11)?;
            (start, end)
        }
        MonthPart::Mid => {
            let start = NaiveDate::from_ymd_opt(year, month, 11)?;
            let end = NaiveDate::from_ymd_opt(year, month, 21)?;
            (start, end)
        }
        MonthPart::Late => {
            let start = NaiveDate::from_ymd_opt(year, month, 21)?;
            let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
            let end = NaiveDate::from_ymd_opt(next_year, next_month, 1)?;
            (start, end)
        }
    };

    let start_dt = start_day.and_hms_opt(0, 0, 0)?;
    let end_dt = end_date.and_hms_opt(0, 0, 0)?;
    Some((start_dt, end_dt))
}

fn month_part_interval(month: u32, part: MonthPart, reference: NaiveDateTime) -> Option<TimeValue> {
    let year = reference.year();
    let (start_this, end_this) = month_part_bounds(year, month, part)?;

    // If reference is before or within this year's interval, use it;
    // otherwise, use the same part next year.
    let (start, end) =
        if reference < end_this { (start_this, end_this) } else { month_part_bounds(year + 1, month, part)? };

    Some(TimeValue::Interval { start, end })
}

fn normalize_month_day_with_weekday(
    month: u32,
    day: u32,
    target_dow: chrono::Weekday,
    reference: NaiveDateTime,
) -> Option<TimeValue> {
    use chrono::Datelike;

    // Start with the reference year
    let mut year = reference.year();

    // Try up to 10 years to find a match (safety limit)
    for _ in 0..10 {
        if let Some(candidate_date) = NaiveDate::from_ymd_opt(year, month, day) {
            // Check if this date matches the weekday
            if candidate_date.weekday() == target_dow {
                let candidate = candidate_date.and_hms_opt(0, 0, 0)?;

                // If it's the same year as reference, return it (even if past)
                // This handles cases like "Sunday, Feb 10" where Feb 10, 2013 is a Sunday
                // but is 2 days before the reference date (Feb 12, 2013)
                if year == reference.year() {
                    return Some(TimeValue::Instant(candidate));
                }

                // For other years, only return if in the future
                if candidate >= reference {
                    return Some(TimeValue::Instant(candidate));
                }
            }
        }

        year += 1;
    }

    // Couldn't find a valid date in the next 10 years
    None
}

fn normalize_day_of_month_with_weekday(
    day: u32,
    target_dow: chrono::Weekday,
    reference: NaiveDateTime,
) -> Option<TimeValue> {
    let start_month = reference.month() as i32 - 1;
    let start_year = reference.year();

    for offset in 0..36 {
        let month_index = start_month + offset;
        let year = start_year + month_index.div_euclid(12);
        let month = month_index.rem_euclid(12) + 1;

        if let Some(candidate_date) = NaiveDate::from_ymd_opt(year, month as u32, day) {
            if candidate_date <= reference.date() {
                continue;
            }
            if candidate_date.weekday() == target_dow {
                let candidate = candidate_date.and_hms_opt(0, 0, 0)?;
                return Some(TimeValue::Instant(candidate));
            }
        }
    }

    None
}

fn apply_constraint(value: TimeValue, constraint: &Constraint, reference: NaiveDateTime) -> Option<TimeValue> {
    match constraint {
        Constraint::Month(target_month) => {
            match value {
                TimeValue::Instant(dt) => {
                    // Intersecting an instant (typically Reference) with a month
                    // gives us the start of that month.
                    let year = if *target_month >= dt.month() { dt.year() } else { dt.year() + 1 };

                    let target_start = NaiveDate::from_ymd_opt(year, *target_month, 1)?.and_hms_opt(0, 0, 0)?;

                    Some(TimeValue::Instant(target_start))
                }
                TimeValue::Interval { start: _start, end: _end } => {
                    // For an interval (like IntervalOf Month), intersecting with a month
                    // constraint should return the full month as an interval.
                    let ref_year = reference.year();
                    let ref_month = reference.month();

                    // Determine which year to use
                    let year = if *target_month >= ref_month { ref_year } else { ref_year + 1 };

                    // Get the start of the target month
                    let target_start = NaiveDate::from_ymd_opt(year, *target_month, 1)?.and_hms_opt(0, 0, 0)?;

                    // Get the end of the target month (start of next month)
                    let (end_year, end_month) =
                        if *target_month == 12 { (year + 1, 1) } else { (year, target_month + 1) };
                    let target_end = NaiveDate::from_ymd_opt(end_year, end_month, 1)?.and_hms_opt(0, 0, 0)?;

                    Some(TimeValue::Interval { start: target_start, end: target_end })
                }
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => {
                    // For open-ended intervals, treat like an instant
                    let year = if *target_month >= dt.month() { dt.year() } else { dt.year() + 1 };
                    let target_start = NaiveDate::from_ymd_opt(year, *target_month, 1)?.and_hms_opt(0, 0, 0)?;
                    Some(TimeValue::Instant(target_start))
                }
            }
        }
        Constraint::DayOfMonth(target_day) => {
            match value {
                TimeValue::Instant(dt) => {
                    // If the instant is at the start of a month (day 1, midnight),
                    // we're likely applying to a "next month" or "this month" expression
                    // In this case, just set the day within that month
                    if dt.day() == 1 && dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 {
                        let target_date =
                            NaiveDate::from_ymd_opt(dt.year(), dt.month(), *target_day)?.and_hms_opt(0, 0, 0)?;
                        Some(TimeValue::Instant(target_date))
                    } else {
                        // Otherwise, find next occurrence of this day of month
                        let current_day = dt.day();
                        let (year, month) = if *target_day > current_day {
                            // Same month if day hasn't passed yet
                            (dt.year(), dt.month())
                        } else {
                            // Next month if day has passed or is today
                            if dt.month() == 12 { (dt.year() + 1, 1) } else { (dt.year(), dt.month() + 1) }
                        };

                        let target_date = NaiveDate::from_ymd_opt(year, month, *target_day)?.and_hms_opt(0, 0, 0)?;

                        Some(TimeValue::Instant(target_date))
                    }
                }
                TimeValue::Interval { .. } => {
                    // Not implemented for intervals yet
                    None
                }
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => {
                    // Treat like an instant
                    if dt.day() == 1 && dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 {
                        let target_date =
                            NaiveDate::from_ymd_opt(dt.year(), dt.month(), *target_day)?.and_hms_opt(0, 0, 0)?;
                        Some(TimeValue::Instant(target_date))
                    } else {
                        let current_day = dt.day();
                        let (year, month) = if *target_day > current_day {
                            (dt.year(), dt.month())
                        } else if dt.month() == 12 {
                            (dt.year() + 1, 1)
                        } else {
                            (dt.year(), dt.month() + 1)
                        };
                        let target_date = NaiveDate::from_ymd_opt(year, month, *target_day)?.and_hms_opt(0, 0, 0)?;
                        Some(TimeValue::Instant(target_date))
                    }
                }
            }
        }
        Constraint::Day(target_day) => {
            match value {
                TimeValue::Instant(dt) => {
                    // Apply day constraint to the given instant
                    // Keep the year and month, change the day
                    let year = dt.year();
                    let month = dt.month();
                    let date = NaiveDate::from_ymd_opt(year, month, *target_day)?;
                    Some(TimeValue::Instant(NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)))
                }
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => {
                    let year = dt.year();
                    let month = dt.month();
                    let date = NaiveDate::from_ymd_opt(year, month, *target_day)?;
                    Some(TimeValue::Instant(NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)))
                }
                TimeValue::Interval { start, .. } => {
                    // For an interval (like a month), pick the specific day within that interval
                    let year = start.year();
                    let month = start.month();
                    let date = NaiveDate::from_ymd_opt(year, month, *target_day)?;
                    Some(TimeValue::Instant(NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)))
                }
            }
        }
        Constraint::DayOfWeek(target_dow) => {
            match value {
                TimeValue::Instant(dt) => {
                    // Find the next occurrence of the target weekday from the reference date
                    use chrono::Datelike;

                    let current_dow = dt.weekday();
                    let target_dow_num = target_dow.num_days_from_monday();
                    let current_dow_num = current_dow.num_days_from_monday();

                    // Calculate days to add
                    let mut days_to_add = if target_dow_num >= current_dow_num {
                        target_dow_num - current_dow_num
                    } else {
                        7 - current_dow_num + target_dow_num
                    };
                    if dt.date() == reference.date() && days_to_add == 0 {
                        days_to_add = 7;
                    }

                    let target_date = dt.date() + chrono::Duration::days(days_to_add as i64);
                    let midnight = NaiveTime::from_hms_opt(0, 0, 0)?;
                    // Preserve time-of-day only when it looks explicitly set (e.g. "Thursday 9am").
                    // If the time matches the reference "now" time, it's typically inherited from
                    // `Reference` and should normalize as a date-only instant at midnight.
                    let target_time = if dt.time() == midnight {
                        midnight
                    } else if dt.time() != reference.time() {
                        dt.time()
                    } else {
                        midnight
                    };
                    let target_instant = target_date.and_time(target_time);

                    Some(TimeValue::Instant(target_instant))
                }
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => {
                    use chrono::Datelike;

                    let current_dow = dt.weekday();
                    let target_dow_num = target_dow.num_days_from_monday();
                    let current_dow_num = current_dow.num_days_from_monday();

                    let mut days_to_add = if target_dow_num >= current_dow_num {
                        target_dow_num - current_dow_num
                    } else {
                        7 - current_dow_num + target_dow_num
                    };
                    if dt.date() == reference.date() && days_to_add == 0 {
                        days_to_add = 7;
                    }

                    let target_date = dt.date() + chrono::Duration::days(days_to_add as i64);
                    let midnight = NaiveTime::from_hms_opt(0, 0, 0)?;
                    let target_time = if dt.time() == midnight {
                        midnight
                    } else if dt.time() != reference.time() {
                        dt.time()
                    } else {
                        midnight
                    };
                    let target_instant = target_date.and_time(target_time);

                    Some(TimeValue::Instant(target_instant))
                }
                TimeValue::Interval { start, end } => {
                    // For "sunday from last week", find the specific weekday within the interval
                    use chrono::Datelike;

                    let target_dow_num = target_dow.num_days_from_monday();

                    // Start from the beginning of the interval
                    let mut current = start;

                    // Find the first occurrence of the target weekday within the interval
                    while current < end {
                        if current.weekday().num_days_from_monday() == target_dow_num {
                            return Some(TimeValue::Instant(current));
                        }
                        current += chrono::Duration::days(1)
                    }

                    // No occurrence found within the interval
                    None
                }
            }
        }
        Constraint::TimeOfDay(time) => {
            // Apply time of day to the value
            match value {
                TimeValue::Instant(dt) => {
                    // Replace the time portion, keep the date
                    let new_dt = dt.date().and_time(*time);

                    // If new_dt is in the past compared to reference, decide whether to move to next day
                    if new_dt < reference {
                        // If the base instant is at midnight (representing a "day" reference),
                        // check if it's a future day. If so, keep it on that day.
                        // If it's a past day (like "yesterday"), also keep it on that day.
                        // Only move forward if it's "today" (reference date)
                        if dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 {
                            // It's a day reference
                            if dt.date() == reference.date() {
                                // Same day - move to next day since time is in past
                                let next_day = new_dt + chrono::Duration::days(1);
                                Some(TimeValue::Instant(next_day))
                            } else {
                                // Different day (past or future) - keep on that day
                                Some(TimeValue::Instant(new_dt))
                            }
                        } else {
                            // Not a day reference (e.g., Reference with a specific time)
                            // Move to next day if in the past
                            let next_day = new_dt + chrono::Duration::days(1);
                            Some(TimeValue::Instant(next_day))
                        }
                    } else {
                        Some(TimeValue::Instant(new_dt))
                    }
                }
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => {
                    let new_dt = dt.date().and_time(*time);
                    if new_dt < reference {
                        if dt.hour() == 0 && dt.minute() == 0 && dt.second() == 0 {
                            if dt.date() == reference.date() {
                                let next_day = new_dt + chrono::Duration::days(1);
                                Some(TimeValue::Instant(next_day))
                            } else {
                                Some(TimeValue::Instant(new_dt))
                            }
                        } else {
                            let next_day = new_dt + chrono::Duration::days(1);
                            Some(TimeValue::Instant(next_day))
                        }
                    } else {
                        Some(TimeValue::Instant(new_dt))
                    }
                }
                TimeValue::Interval { start, end } => {
                    // Duckling-style special case: when constraining a short interval
                    // (like a part-of-day) with a bare "12", interpret it as midnight
                    // rather than noon.
                    if *time == NaiveTime::from_hms_opt(12, 0, 0)? && (end - start) <= Duration::hours(24) {
                        let midnight = NaiveTime::from_hms_opt(0, 0, 0)?;
                        return Some(TimeValue::Instant((start.date() + Duration::days(1)).and_time(midnight)));
                    }

                    // Apply the time-of-day within the interval window.
                    // This is used for cases like "this afternoon at 2" where the part-of-day
                    // interval should disambiguate 2 -> 14:00.
                    let pick_in_window =
                        |window_start: NaiveDateTime, window_end: NaiveDateTime| -> Option<NaiveDateTime> {
                            let mut best: Option<NaiveDateTime> = None;
                            let dates = [window_start.date(), window_end.date()];

                            for date in dates {
                                let base = date.and_time(*time);
                                for candidate in [base, base + Duration::hours(12)] {
                                    if candidate >= window_start
                                        && candidate < window_end
                                        && candidate >= reference
                                        && best.is_none_or(|b| candidate < b)
                                    {
                                        best = Some(candidate);
                                    }
                                }
                            }

                            best
                        };

                    // Prefer a candidate in the current interval window.
                    if let Some(chosen) = pick_in_window(start, end) {
                        return Some(TimeValue::Instant(chosen));
                    }

                    // If none fits (often because it's already in the past), try the next
                    // occurrence of the interval window (shifted by one day).
                    let start_next = start + Duration::days(1);
                    let end_next = end + Duration::days(1);
                    pick_in_window(start_next, end_next).map(TimeValue::Instant)
                }
            }
        }
        Constraint::PartOfDay(pod) => {
            let base_date = match value {
                TimeValue::Instant(dt) => dt.date(),
                TimeValue::Interval { start, .. } => start.date(),
                TimeValue::OpenAfter(dt) | TimeValue::OpenBefore(dt) => dt.date(),
            };

            let (start, end) = part_of_day_bounds(base_date, pod)?;

            // If we're constraining a specific clock time, try to keep it as an
            // instant by disambiguating with the part-of-day (e.g. "3" +
            // afternoon => 15:00). Prefer the earliest candidate >= reference.
            if let TimeValue::Instant(dt) = value {
                // When constraining the plain reference instant ("now"), the intent is a
                // part-of-day interval (e.g. "this afternoon"), not an arbitrary instant.
                if dt == reference {
                    return Some(TimeValue::Interval { start, end });
                }

                // Midnight instants are typically date anchors (e.g. a resolved
                // holiday/date). Applying a part-of-day should then yield the
                // whole part-of-day interval, not a single instant.
                if dt.time() == NaiveTime::from_hms_opt(0, 0, 0)? {
                    return Some(TimeValue::Interval { start, end });
                }

                let pod_implies_pm = matches!(
                    pod,
                    PartOfDay::Afternoon
                        | PartOfDay::AfterLunch
                        | PartOfDay::AfterWork
                        | PartOfDay::Evening
                        | PartOfDay::Night
                        | PartOfDay::Tonight
                        | PartOfDay::LateTonight
                );

                let mut best: Option<NaiveDateTime> = None;
                for date in [reference.date(), dt.date()] {
                    let (pod_start, pod_end) = part_of_day_bounds(date, pod)?;
                    let base = NaiveDateTime::new(date, dt.time());

                    let mut consider = |candidate: NaiveDateTime| {
                        if candidate >= pod_start
                            && candidate < pod_end
                            && candidate >= reference
                            && best.is_none_or(|b| candidate < b)
                        {
                            best = Some(candidate);
                        }
                    };

                    consider(base);
                    if pod_implies_pm && base.time().hour() < 12 {
                        consider(base + Duration::hours(12));
                    }
                }

                if let Some(chosen) = best {
                    return Some(TimeValue::Instant(chosen));
                }
            }

            Some(TimeValue::Interval { start, end })
        }
    }
}

fn part_of_day_bounds(date: NaiveDate, pod: &PartOfDay) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let (start_time, end_time) = match pod {
        PartOfDay::EarlyMorning => {
            (chrono::NaiveTime::from_hms_opt(0, 0, 0)?, chrono::NaiveTime::from_hms_opt(9, 0, 0)?)
        }
        PartOfDay::Morning => (chrono::NaiveTime::from_hms_opt(0, 0, 0)?, chrono::NaiveTime::from_hms_opt(12, 0, 0)?),
        PartOfDay::Afternoon => {
            (chrono::NaiveTime::from_hms_opt(12, 0, 0)?, chrono::NaiveTime::from_hms_opt(19, 0, 0)?)
        }
        PartOfDay::AfterLunch => {
            (chrono::NaiveTime::from_hms_opt(13, 0, 0)?, chrono::NaiveTime::from_hms_opt(17, 0, 0)?)
        }
        PartOfDay::Lunch => (chrono::NaiveTime::from_hms_opt(12, 0, 0)?, chrono::NaiveTime::from_hms_opt(14, 0, 0)?),
        PartOfDay::Evening | PartOfDay::Night | PartOfDay::Tonight => {
            (chrono::NaiveTime::from_hms_opt(18, 0, 0)?, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)
        }
        PartOfDay::LateTonight => {
            (chrono::NaiveTime::from_hms_opt(21, 0, 0)?, chrono::NaiveTime::from_hms_opt(0, 0, 0)?)
        }
        PartOfDay::AfterWork => {
            (chrono::NaiveTime::from_hms_opt(15, 0, 0)?, chrono::NaiveTime::from_hms_opt(21, 0, 0)?)
        }
    };

    let start = NaiveDateTime::new(date, start_time);
    let end = if end_time == chrono::NaiveTime::from_hms_opt(0, 0, 0)? {
        NaiveDateTime::new(date.checked_add_signed(chrono::Duration::days(1))?, end_time)
    } else {
        NaiveDateTime::new(date, end_time)
    };

    Some((start, end))
}

pub fn format_time_value(value: &TimeValue) -> String {
    match value {
        TimeValue::Instant(dt) => fmt_instant(*dt),
        TimeValue::Interval { start, end } => fmt_interval(*start, *end),
        TimeValue::OpenAfter(dt) => format!("{}+", format_datetime(*dt)),
        TimeValue::OpenBefore(dt) => format!("{}-", format_datetime(*dt)),
    }
}

fn format_datetime(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn fmt_instant(dt: NaiveDateTime) -> String {
    format_datetime(dt)
}

fn fmt_interval(start: NaiveDateTime, end: NaiveDateTime) -> String {
    format!("{}/{}", format_datetime(start), format_datetime(end))
}

/// Apply part of day to reference time, returning an interval for that part of day
fn apply_part_of_day_to_reference(part_of_day: PartOfDay, reference: NaiveDateTime) -> Option<TimeValue> {
    let date = reference.date();
    let (start, end) = part_of_day_bounds(date, &part_of_day)?;
    Some(TimeValue::Interval { start, end })
}

/// Normalize a holiday to a specific date
fn normalize_holiday(holiday: Holiday, year: Option<i32>, reference: NaiveDateTime) -> Option<TimeValue> {
    use Holiday::*;
    use chrono::Weekday;

    // Handle special year markers:
    // year = Some(-1) means "last year" (reference.year() - 1)
    // year = Some(1) means "next year" (reference.year() + 1)
    // year = Some(specific_year) where specific_year > 1000 means that explicit year
    // year = None means find nearest occurrence from reference
    let resolved_year = match year {
        Some(-1) => Some(reference.year() - 1),
        Some(1) => Some(reference.year() + 1),
        Some(y) if y > 1000 => Some(y), // Explicit year like 2014
        Some(_) => None,                // Invalid marker, treat as None
        None => None,
    };

    // Convert the holiday to its underlying TimeExpr representation
    let expr = match holiday {
        Thanksgiving => TimeExpr::NthWeekdayOfMonth { n: 4, year: resolved_year, month: 11, weekday: Weekday::Thu },
        Christmas => TimeExpr::MonthDay { month: 12, day: 25 },
        ChristmasEve => TimeExpr::MonthDay { month: 12, day: 24 },
        NewYearsDay => TimeExpr::MonthDay { month: 1, day: 1 },
        NewYearsEve => TimeExpr::MonthDay { month: 12, day: 31 },
        IndependenceDay => TimeExpr::MonthDay { month: 7, day: 4 },
        Halloween => TimeExpr::MonthDay { month: 10, day: 31 },
        VeteransDay => TimeExpr::MonthDay { month: 11, day: 11 },
        StPatricksDay => TimeExpr::MonthDay { month: 3, day: 17 },
        EarthDay => TimeExpr::MonthDay { month: 4, day: 22 },
        MLKDay => TimeExpr::NthWeekdayOfMonth { n: 3, year: resolved_year, month: 1, weekday: Weekday::Mon },
        PresidentsDay => TimeExpr::NthWeekdayOfMonth { n: 3, year: resolved_year, month: 2, weekday: Weekday::Mon },
        MemorialDay => TimeExpr::LastWeekdayOfMonth { year: resolved_year, month: 5, weekday: Weekday::Mon },
        LaborDay => TimeExpr::NthWeekdayOfMonth { n: 1, year: resolved_year, month: 9, weekday: Weekday::Mon },
        ColumbusDay => TimeExpr::NthWeekdayOfMonth { n: 2, year: resolved_year, month: 10, weekday: Weekday::Mon },
        MothersDay => TimeExpr::NthWeekdayOfMonth { n: 2, year: resolved_year, month: 5, weekday: Weekday::Sun },
        FathersDay => TimeExpr::NthWeekdayOfMonth { n: 3, year: resolved_year, month: 6, weekday: Weekday::Sun },
        BossDay => TimeExpr::MonthDay { month: 10, day: 16 },
        BlackFriday => TimeExpr::LastWeekdayOfMonth { year: resolved_year, month: 11, weekday: Weekday::Fri },
    };

    // Normalize the underlying expression
    normalize(&expr, reference)
}

fn normalize_season(season: Season, reference: NaiveDateTime) -> Option<TimeValue> {
    use chrono::NaiveDate;

    let year = reference.year();

    let mk_dt = |y: i32, m: u32, d: u32| {
        Some(NaiveDateTime::new(NaiveDate::from_ymd_opt(y, m, d)?, chrono::NaiveTime::from_hms_opt(0, 0, 0)?))
    };

    // Northern-hemisphere astronomical-ish season boundaries, matching the test corpus.
    // Intervals are [start, end) at midnight.
    let bounds_for_start_year = |start_year: i32| match season {
        Season::Spring => Some((mk_dt(start_year, 3, 21)?, mk_dt(start_year, 6, 21)?)),
        Season::Summer => Some((mk_dt(start_year, 6, 21)?, mk_dt(start_year, 9, 24)?)),
        Season::Fall => Some((mk_dt(start_year, 9, 24)?, mk_dt(start_year, 12, 21)?)),
        Season::Winter => Some((mk_dt(start_year, 12, 21)?, mk_dt(start_year + 1, 3, 21)?)),
    };

    let (start, end) = match season {
        Season::Winter => {
            let (w_prev_start, w_prev_end) = bounds_for_start_year(year - 1)?;
            if reference >= w_prev_start && reference < w_prev_end {
                (w_prev_start, w_prev_end)
            } else {
                bounds_for_start_year(year)?
            }
        }
        _ => {
            let (this_start, this_end) = bounds_for_start_year(year)?;
            if reference < this_start {
                (this_start, this_end)
            } else if reference >= this_end {
                bounds_for_start_year(year + 1)?
            } else {
                (this_start, this_end)
            }
        }
    };

    Some(TimeValue::Interval { start, end })
}

fn normalize_season_period(offset: i32, reference: NaiveDateTime) -> Option<TimeValue> {
    use chrono::NaiveDate;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum SeasonIdx {
        Winter,
        Spring,
        Summer,
        Fall,
    }

    let mk_dt = |y: i32, m: u32, d: u32| {
        Some(NaiveDateTime::new(NaiveDate::from_ymd_opt(y, m, d)?, chrono::NaiveTime::from_hms_opt(0, 0, 0)?))
    };

    let bounds = |idx: SeasonIdx, year: i32| -> Option<(NaiveDateTime, NaiveDateTime)> {
        match idx {
            SeasonIdx::Spring => Some((mk_dt(year, 3, 20)?, mk_dt(year, 6, 20)?)),
            SeasonIdx::Summer => Some((mk_dt(year, 6, 21)?, mk_dt(year, 9, 22)?)),
            SeasonIdx::Fall => Some((mk_dt(year, 9, 23)?, mk_dt(year, 12, 20)?)),
            SeasonIdx::Winter => Some((mk_dt(year, 12, 21)?, mk_dt(year + 1, 3, 19)?)),
        }
    };

    let date = reference.date();
    let year = reference.year();
    let dec21 = NaiveDate::from_ymd_opt(year, 12, 21)?;
    let mar19 = NaiveDate::from_ymd_opt(year, 3, 19)?;
    let mar20 = NaiveDate::from_ymd_opt(year, 3, 20)?;
    let jun20 = NaiveDate::from_ymd_opt(year, 6, 20)?;
    let jun21 = NaiveDate::from_ymd_opt(year, 6, 21)?;
    let sep22 = NaiveDate::from_ymd_opt(year, 9, 22)?;
    let sep23 = NaiveDate::from_ymd_opt(year, 9, 23)?;
    let dec20 = NaiveDate::from_ymd_opt(year, 12, 20)?;

    // Determine the season period containing the reference date.
    let (mut idx, mut period_year) = if date >= dec21 {
        (SeasonIdx::Winter, year)
    } else if date <= mar19 {
        (SeasonIdx::Winter, year - 1)
    } else if date >= mar20 && date <= jun20 {
        (SeasonIdx::Spring, year)
    } else if date >= jun21 && date <= sep22 {
        (SeasonIdx::Summer, year)
    } else if date >= sep23 && date <= dec20 {
        (SeasonIdx::Fall, year)
    } else {
        // In gaps (if any), default to the next winter starting this year.
        (SeasonIdx::Winter, year)
    };

    let mut steps = offset;
    while steps != 0 {
        if steps > 0 {
            // advance
            match idx {
                SeasonIdx::Winter => {
                    idx = SeasonIdx::Spring;
                    period_year += 1;
                }
                SeasonIdx::Spring => idx = SeasonIdx::Summer,
                SeasonIdx::Summer => idx = SeasonIdx::Fall,
                SeasonIdx::Fall => idx = SeasonIdx::Winter,
            }
            steps -= 1;
        } else {
            // retreat
            match idx {
                SeasonIdx::Winter => idx = SeasonIdx::Fall,
                SeasonIdx::Fall => idx = SeasonIdx::Summer,
                SeasonIdx::Summer => idx = SeasonIdx::Spring,
                SeasonIdx::Spring => {
                    idx = SeasonIdx::Winter;
                    period_year -= 1;
                }
            }
            steps += 1;
        }
    }

    let (start, end) = bounds(idx, period_year)?;
    Some(TimeValue::Interval { start, end })
}
