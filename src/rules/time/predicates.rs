use crate::time_expr::{Constraint, TimeExpr};
use crate::{Dimension, Token, TokenKind};
use chrono::Weekday;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Regex pattern for matching month names and abbreviations
pub(crate) const MONTH_REGEX_PATTERN: &str = r"(?i)(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)";

/// Compiled regex for month names (with anchors for exact matching in predicates)
pub(crate) static MONTH_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(&format!(r"^{}$", MONTH_REGEX_PATTERN)).unwrap());

/// Compiled regex for month names (for use in rule patterns)
pub(crate) static MONTH_PATTERN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(MONTH_REGEX_PATTERN).unwrap());

pub(crate) static DAY_OF_WEEK: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        ("monday", "monday"),
        ("mon", "monday"),
        ("tuesday", "tuesday"),
        ("tue", "tuesday"),
        ("tues", "tuesday"),
        ("wednesday", "wednesday"),
        ("wed", "wednesday"),
        ("thursday", "thursday"),
        ("thu", "thursday"),
        ("thurs", "thursday"),
        ("friday", "friday"),
        ("fri", "friday"),
        ("saturday", "saturday"),
        ("sat", "saturday"),
        ("sunday", "sunday"),
        ("sun", "sunday"),
    ])
});

pub(crate) static MONTH_NAME: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    HashMap::from([
        ("january", 1),
        ("jan", 1),
        ("february", 2),
        ("feb", 2),
        ("march", 3),
        ("mar", 3),
        ("april", 4),
        ("apr", 4),
        ("may", 5),
        ("june", 6),
        ("jun", 6),
        ("july", 7),
        ("jul", 7),
        ("august", 8),
        ("aug", 8),
        ("september", 9),
        ("sept", 9),
        ("sep", 9),
        ("october", 10),
        ("oct", 10),
        ("november", 11),
        ("nov", 11),
        ("december", 12),
        ("dec", 12),
    ])
});

/// Returns true when the token matches the month regex pattern.
pub fn is_month(token: &Token) -> bool {
    let text = match &token.kind {
        TokenKind::RegexMatch(groups) => match groups.first() {
            Some(s) => s.as_str(),
            None => return false,
        },
        _ => return false,
    };

    MONTH_REGEX.is_match(text)
}

/// Returns true when the token is a Time dimension token.
pub fn is_time(t: &Token) -> bool {
    matches!(t.dim, Dimension::Time)
}

fn is_dom_numeral(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::Numeral(nd)
        if nd.value.fract().abs() < f64::EPSILON && nd.value >= 1.0 && nd.value <= 31.0)
}

/// Returns true when the token looks like an ordinal day-of-month value.
pub fn is_dom_ordinal(token: &Token) -> bool {
    is_dom_numeral(token)
}

// ============================================================================
// TimeExpr-based predicates (formerly in predicates_v2.rs)
// ============================================================================

/// Returns true if the token is a TimeExpr
pub fn is_time_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(_))
}

/// Returns true if the token is a TimeExpr with a Month constraint
pub fn is_month_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::Month(_), .. }))
}

/// Returns the month number from a TimeExpr if it's a month constraint
pub fn month_from_expr(token: &Token) -> Option<u32> {
    match &token.kind {
        TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::Month(m), .. }) => Some(*m),
        _ => None,
    }
}

/// Returns true if the token is a TimeExpr with a DayOfWeek constraint
pub fn is_weekday_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfWeek(_), .. }))
}

/// Returns the weekday from a TimeExpr if it's a weekday constraint
pub fn weekday_from_expr(token: &Token) -> Option<Weekday> {
    match &token.kind {
        TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfWeek(d), .. }) => Some(*d),
        _ => None,
    }
}

/// Returns true if the token is a TimeExpr with a DayOfMonth constraint
pub fn is_day_of_month_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfMonth(_), .. }))
}

/// Returns the day of month from a TimeExpr if it's a DayOfMonth constraint
pub fn day_of_month_from_expr(token: &Token) -> Option<u32> {
    match &token.kind {
        TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfMonth(d), .. }) => Some(*d),
        // Also accept plain Numeral tokens in the valid day-of-month range,
        // so that word/number days like "seventeen" can participate in
        // month/day compositions once numeral rules have fired.
        TokenKind::Numeral(nd) if nd.value.fract().abs() < f64::EPSILON && nd.value >= 1.0 && nd.value <= 31.0 => {
            Some(nd.value as u32)
        }
        _ => None,
    }
}

/// Returns true if the token is a TimeExpr with a TimeOfDay constraint
pub fn is_time_of_day_expr(token: &Token) -> bool {
    time_from_expr(token).is_some()
}

/// Returns true if the token is a TimeExpr::AmbiguousTime (e.g. "seven thirty").
pub fn is_ambiguous_time_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(TimeExpr::AmbiguousTime { .. }))
}

/// Returns the time from a TimeExpr if it's a TimeOfDay constraint
pub fn time_from_expr(token: &Token) -> Option<chrono::NaiveTime> {
    fn time_from_time_expr(expr: &TimeExpr) -> Option<chrono::NaiveTime> {
        match expr {
            TimeExpr::Intersect { constraint: Constraint::TimeOfDay(t), .. } => Some(*t),
            // Only unwrap no-op shifts (used for precision markers like hh:mm:ss).
            TimeExpr::Shift { expr, amount: 0, .. } => time_from_time_expr(expr),
            _ => None,
        }
    }

    match &token.kind {
        TokenKind::TimeExpr(expr) => time_from_time_expr(expr),
        _ => None,
    }
}

/// Returns true if the token is a future shift expression (Shift or StartOf{Shift} with positive amount)
pub fn is_future_shift_expr(token: &Token) -> bool {
    match &token.kind {
        TokenKind::TimeExpr(TimeExpr::Shift { amount, .. }) => *amount > 0,
        TokenKind::TimeExpr(TimeExpr::StartOf { expr, .. }) => {
            matches!(&**expr, TimeExpr::Shift { amount, .. } if *amount > 0)
        }
        _ => false,
    }
}

/// Generic helper to extract the inner TimeExpr from a token
pub fn get_time_expr(token: &Token) -> Option<&TimeExpr> {
    match &token.kind {
        TokenKind::TimeExpr(expr) => Some(expr),
        _ => None,
    }
}

/// Returns true if token is a day-of-month expression (integer 1-31)
pub fn is_day_of_month_numeral(token: &Token) -> bool {
    // Accept either an existing DayOfMonth time expression or a bare
    // numeral in the 1..=31 range. This lets rules that compose
    // month/day structures work with inputs like "seventeen" once
    // numeral rules have produced a Numeral token.
    if is_day_of_month_expr(token) {
        return true;
    }
    matches!(&token.kind, TokenKind::Numeral(nd) if nd.value.fract().abs() < f64::EPSILON && nd.value >= 1.0 && nd.value <= 31.0)
}

/// Returns true if the token is a TimeExpr::MonthDay
pub fn is_month_day_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(TimeExpr::MonthDay { .. }))
}

/// Returns (month, day) from a MonthDay expression
pub fn month_day_from_expr(token: &Token) -> Option<(u32, u32)> {
    match &token.kind {
        TokenKind::TimeExpr(TimeExpr::MonthDay { month, day }) => Some((*month, *day)),
        _ => None,
    }
}

/// Returns true if token is a weekday name (regex match)
pub fn is_weekday_name(token: &Token) -> bool {
    match &token.kind {
        TokenKind::RegexMatch(groups) => {
            if let Some(text) = groups.first() {
                DAY_OF_WEEK.contains_key(text.to_lowercase().as_str())
            } else {
                false
            }
        }
        TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfWeek(_), .. }) => true,
        _ => false,
    }
}

/// Extract weekday from a weekday name token
pub fn weekday_from_name(token: &Token) -> Option<Weekday> {
    match &token.kind {
        TokenKind::RegexMatch(groups) => {
            let text = groups.first()?.to_lowercase();
            let normalized = DAY_OF_WEEK.get(text.as_str())?;
            match *normalized {
                "monday" => Some(Weekday::Mon),
                "tuesday" => Some(Weekday::Tue),
                "wednesday" => Some(Weekday::Wed),
                "thursday" => Some(Weekday::Thu),
                "friday" => Some(Weekday::Fri),
                "saturday" => Some(Weekday::Sat),
                "sunday" => Some(Weekday::Sun),
                _ => None,
            }
        }
        TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfWeek(weekday), .. }) => Some(*weekday),
        _ => None,
    }
}

/// Returns true if token is a duration expression
pub fn is_duration_expr(token: &Token) -> bool {
    matches!(&token.kind, TokenKind::TimeExpr(TimeExpr::Duration(_)))
}

/// Extract duration from a duration expression
pub fn get_duration_expr(token: &Token) -> Option<&TimeExpr> {
    match &token.kind {
        TokenKind::TimeExpr(expr @ TimeExpr::Duration(_)) => Some(expr),
        _ => None,
    }
}
