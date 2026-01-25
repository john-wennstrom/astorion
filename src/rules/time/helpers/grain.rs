//! Grain and time expression utilities

use crate::Token;
use crate::time_expr::{Constraint, Grain, PartOfDay, TimeExpr};
use chrono::Timelike;

/// Get the container grain for a time expression
pub fn container_grain_for_expr(expr: &TimeExpr) -> Grain {
    match expr {
        TimeExpr::StartOf { grain, .. } | TimeExpr::IntervalOf { grain, .. } => *grain,
        TimeExpr::Shift { grain, .. } => *grain,
        TimeExpr::Intersect { constraint, .. } => match constraint {
            Constraint::Month(_) => Grain::Month,
            Constraint::DayOfMonth(_) => Grain::Month,
            Constraint::DayOfWeek(_) => Grain::Week,
            Constraint::Day(_) => Grain::Day,
            Constraint::TimeOfDay(_) | Constraint::PartOfDay(_) => Grain::Day,
        },
        TimeExpr::MonthPart { .. } => Grain::Month,
        TimeExpr::MonthDay { .. } => Grain::Day,
        TimeExpr::ClosestWeekdayTo { .. } => Grain::Day,
        TimeExpr::Absolute { month, day, .. } => {
            if *month == 1 && *day == 1 {
                Grain::Year
            } else {
                Grain::Day
            }
        }
        TimeExpr::Interval { .. }
        | TimeExpr::IntervalBetween { .. }
        | TimeExpr::IntervalUntil { .. }
        | TimeExpr::OpenAfter { .. }
        | TimeExpr::OpenBefore { .. } => Grain::Day,
        TimeExpr::Reference | TimeExpr::At(_) => Grain::Day,
        TimeExpr::LastWeekdayOfMonth { .. } => Grain::Day,
        TimeExpr::FirstWeekdayOfMonth { .. } => Grain::Day,
        TimeExpr::NthWeekdayOfMonth { .. } => Grain::Day,
        TimeExpr::NthWeekOf { .. } => Grain::Week,
        TimeExpr::NthLastOf { grain, .. } => *grain,
        // New variants
        TimeExpr::Holiday { .. } => Grain::Day,
        TimeExpr::Season(_) => Grain::Month,
        TimeExpr::SeasonPeriod { .. } => Grain::Month,
        TimeExpr::PartOfDay(_) => Grain::Day,
        TimeExpr::After(_) | TimeExpr::Before(_) => Grain::Day,
        TimeExpr::Duration(_) => Grain::Day,
        TimeExpr::AmbiguousTime { .. } => Grain::Minute,
    }
}

/// Get the grain of a time of day
pub fn time_of_day_grain(time: &chrono::NaiveTime) -> Grain {
    if time.second() != 0 {
        Grain::Second
    } else {
        // Default to Minute for intervals. Hour-level precision (e.g., "3-4pm")
        // is handled by specific rules that explicitly create hour-grain intervals.
        Grain::Minute
    }
}

/// Get the grain of a time expression
pub fn grain_of_time_expr(expr: &TimeExpr) -> Grain {
    match expr {
        TimeExpr::Intersect { constraint: Constraint::TimeOfDay(time), .. } => time_of_day_grain(time),
        _ => Grain::Minute, // Default to minute for other time expressions
    }
}

/// Adjust time for part of day
pub fn adjust_time_for_part_of_day(time: chrono::NaiveTime, part: PartOfDay) -> chrono::NaiveTime {
    let hour = time.hour();
    let minute = time.minute();
    let second = time.second();

    let adjusted_hour = match part {
        PartOfDay::Morning | PartOfDay::EarlyMorning => {
            if hour == 12 {
                0
            } else {
                hour
            }
        }
        PartOfDay::Afternoon | PartOfDay::AfterLunch | PartOfDay::AfterWork => match hour.cmp(&12) {
            std::cmp::Ordering::Equal => 12,
            std::cmp::Ordering::Less => hour + 12,
            std::cmp::Ordering::Greater => hour,
        },
        PartOfDay::Evening | PartOfDay::Night | PartOfDay::Tonight | PartOfDay::LateTonight => match hour.cmp(&12) {
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Less => hour + 12,
            std::cmp::Ordering::Greater => hour,
        },
        PartOfDay::Lunch => {
            if hour < 11 {
                hour + 12
            } else {
                hour
            }
        }
    };

    chrono::NaiveTime::from_hms_opt(adjusted_hour, minute, second).unwrap_or(time)
}

/// Add year to a time expression
pub fn time_expr_with_year(expr: &TimeExpr, year: i32) -> Option<TimeExpr> {
    match expr {
        TimeExpr::MonthDay { month, day } => {
            Some(TimeExpr::Absolute { year, month: *month, day: *day, hour: None, minute: None })
        }
        TimeExpr::ClosestWeekdayTo { n, weekday, target } => {
            let target_with_year = time_expr_with_year(target.as_ref(), year)?;
            Some(TimeExpr::ClosestWeekdayTo { n: *n, weekday: *weekday, target: Box::new(target_with_year) })
        }
        TimeExpr::Intersect { constraint: Constraint::Month(month), expr } if matches!(**expr, TimeExpr::Reference) => {
            Some(TimeExpr::Absolute { year, month: *month, day: 1, hour: None, minute: None })
        }
        TimeExpr::Absolute { month, day, hour, minute, .. } => {
            Some(TimeExpr::Absolute { year, month: *month, day: *day, hour: *hour, minute: *minute })
        }
        _ => None,
    }
}

/// Create time expression with minute offset from hour token
pub fn time_expr_minutes_offset(hour_token: &Token, minute_offset: i64) -> Option<TimeExpr> {
    use crate::rules::time::helpers::parse::time_expr_with_minutes;
    use crate::rules::time::predicates::time_from_expr;

    let time = time_from_expr(hour_token)?;
    let total_minutes = (time.hour() as i64 * 60 + time.minute() as i64 + minute_offset).rem_euclid(24 * 60);
    let mut hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    if time.hour() != 12 && hours > 0 && hours < 12 {
        hours += 12;
    }

    time_expr_with_minutes(hours, minutes, false)
}

/// Extract constraint from a time expression
pub fn constraint_from_expr(expr: &TimeExpr) -> Option<Constraint> {
    match expr {
        TimeExpr::Intersect { expr, constraint } if matches!(**expr, TimeExpr::Reference) => Some(constraint.clone()),
        _ => None,
    }
}

/// Intersect two time expressions
pub fn intersect_time_exprs(lhs: &TimeExpr, rhs: &TimeExpr) -> Option<TimeExpr> {
    if let Some(constraint) = constraint_from_expr(rhs) {
        return Some(TimeExpr::Intersect { expr: Box::new(lhs.clone()), constraint });
    }

    if let Some(constraint) = constraint_from_expr(lhs) {
        return Some(TimeExpr::Intersect { expr: Box::new(rhs.clone()), constraint });
    }

    None
}
