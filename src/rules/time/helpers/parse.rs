//! Parsing utilities for extracting values from tokens

use crate::time_expr::{Constraint, Grain, PartOfDay, Season, TimeExpr};
use crate::{Pattern, Token, TokenKind};

/// Extract integer value from a numeral token
pub fn integer_value(token: &Token) -> Option<i64> {
    match &token.kind {
        TokenKind::Numeral(nd) if nd.value.fract().abs() < f64::EPSILON => Some(nd.value as i64),
        _ => None,
    }
}

/// Extract day-of-month value from a token
pub fn dom_value(token: &Token) -> Option<i64> {
    match &token.kind {
        // Direct numeral (e.g., produced from "seventeen")
        TokenKind::Numeral(_) => integer_value(token),
        // Or an already-built DayOfMonth time expression
        TokenKind::TimeExpr(TimeExpr::Intersect { constraint: Constraint::DayOfMonth(d), .. }) => Some(*d as i64),
        _ => None,
    }
}

/// Extract ordinal value from text (e.g., "1st", "2nd", "first", "second")
pub fn ordinal_value(token: &Token) -> Option<i32> {
    let text = match &token.kind {
        TokenKind::RegexMatch(groups) => groups.first()?.to_lowercase(),
        _ => return None,
    };
    let text = text.trim();

    if text.chars().all(|c| c.is_ascii_digit()) {
        return text.parse().ok();
    }

    if text.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        let trimmed = text.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        return trimmed.parse().ok();
    }

    match text {
        "first" => Some(1),
        "second" => Some(2),
        "third" => Some(3),
        "fourth" => Some(4),
        "fifth" => Some(5),
        "sixth" => Some(6),
        "seventh" => Some(7),
        "eighth" => Some(8),
        "ninth" => Some(9),
        "tenth" => Some(10),
        _ => None,
    }
}

/// Parse integer text like "one", "two", "three", etc.
pub fn parse_integer_text(text: &str) -> Option<i32> {
    let normalized = text.trim().to_lowercase();
    if normalized.chars().all(|c| c.is_ascii_digit()) {
        return normalized.parse().ok();
    }
    match normalized.as_str() {
        "zero" => Some(0),
        "o" => Some(0),
        "oh" => Some(0),
        "ou" => Some(0),
        "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        "thirty" => Some(30),
        "forty" => Some(40),
        "fifty" => Some(50),
        _ => None,
    }
}

/// Extract integer from specific capture group of regex match
pub fn regex_group_int_value(token: &Token, idx: usize) -> Option<i64> {
    match &token.kind {
        TokenKind::RegexMatch(groups) => groups.get(idx)?.parse().ok(),
        _ => None,
    }
}

/// Parse grain from cycle text (e.g., "day", "week", "month")
pub fn grain_from_cycle(cycle: &str) -> Option<Grain> {
    match cycle {
        "day" => Some(Grain::Day),
        "week" => Some(Grain::Week),
        "month" => Some(Grain::Month),
        "quarter" => Some(Grain::Quarter),
        "year" => Some(Grain::Year),
        _ => None,
    }
}

/// Parse part of day from text (e.g., "morning", "afternoon", "evening")
pub fn part_of_day_from_text(text: &str) -> Option<PartOfDay> {
    let normalized = text.trim().to_lowercase();
    let normalized = normalized.strip_prefix("the ").unwrap_or(&normalized);
    let normalized = normalized.strip_prefix("in the ").unwrap_or(normalized);
    let normalized = normalized.strip_prefix("in ").unwrap_or(normalized);
    let normalized = normalized.strip_prefix("at ").unwrap_or(normalized);
    if normalized.contains("early") && normalized.contains("morning") {
        return Some(PartOfDay::EarlyMorning);
    }
    if normalized.contains("morning") {
        return Some(PartOfDay::Morning);
    }
    if normalized.contains("afternoon") {
        return Some(PartOfDay::Afternoon);
    }
    if normalized.contains("lunch") {
        return Some(PartOfDay::Lunch);
    }
    if normalized.contains("evening") {
        return Some(PartOfDay::Evening);
    }
    if normalized.contains("night") {
        return Some(PartOfDay::Night);
    }
    None
}

/// Extract part of day from token (wraps part_of_day_from_text)
pub fn part_of_day_from_token(token: &Token) -> Option<PartOfDay> {
    match &token.kind {
        TokenKind::RegexMatch(groups) => {
            let text = groups.first()?;
            part_of_day_from_text(text)
        }
        _ => None,
    }
}

/// Extract season from regex text match
pub fn season_from_text(token: &Token) -> Option<Season> {
    match &token.kind {
        TokenKind::RegexMatch(groups) => {
            let text = groups.get(1)?.to_lowercase();
            match text.as_str() {
                "summer" => Some(Season::Summer),
                "fall" | "autumn" => Some(Season::Fall),
                "winter" => Some(Season::Winter),
                "spring" => Some(Season::Spring),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Parse duration from a regex token (e.g., "5 minutes", "3 hours")
pub fn parse_duration(token: &Token) -> Option<(i32, Grain)> {
    let groups = match &token.kind {
        TokenKind::RegexMatch(groups) => groups,
        _ => return None,
    };

    let full_match = groups.first()?.to_lowercase();
    let captures =
        regex::Regex::new(r"(?i)^\s*(\d+)\s*(seconds?|minutes?|hours?|days?|weeks?|months?|years?|h|'|min)\s*$")
            .ok()?
            .captures(full_match.as_str())?;
    let amount: i32 = captures.get(1)?.as_str().parse().ok()?;
    let unit = captures.get(2)?.as_str();

    let grain = match unit {
        "second" | "seconds" => Grain::Second,
        "minute" | "minutes" | "'" | "min" => Grain::Minute,
        "hour" | "hours" | "h" => Grain::Hour,
        "day" | "days" => Grain::Day,
        "week" | "weeks" => Grain::Week,
        "month" | "months" => Grain::Month,
        "year" | "years" => Grain::Year,
        _ => return None,
    };

    Some((amount, grain))
}

/// Parse text duration like "one year", "three days"
pub fn parse_text_duration(token: &Token) -> Option<(i32, Grain)> {
    let groups = match &token.kind {
        TokenKind::RegexMatch(groups) => groups,
        _ => return None,
    };

    let full_match = groups.first()?.to_lowercase();
    let parts: Vec<&str> = full_match.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let amount = match parts[0] {
        "a" | "an" | "one" => 1,
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
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        _ => return None,
    };

    let grain = match parts[1] {
        "second" | "seconds" => Grain::Second,
        "minute" | "minutes" => Grain::Minute,
        "hour" | "hours" => Grain::Hour,
        "day" | "days" => Grain::Day,
        "week" | "weeks" => Grain::Week,
        "month" | "months" => Grain::Month,
        "year" | "years" => Grain::Year,
        _ => return None,
    };

    Some((amount, grain))
}

/// Get duration pattern for regex matching
pub fn duration_pattern() -> &'static str {
    r"(?i)(\d+\s*(seconds?|minutes?|hours?|days?|weeks?|months?|years?|h|'|min))"
}

/// Get text duration pattern for regex matching
pub fn text_duration_pattern() -> &'static str {
    r"(?i)((a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fifty)\s+(second|seconds|minute|minutes|hour|hours|day|days|week|weeks|month|months|year|years))"
}

/// Get timezone pattern for regex matching
pub fn timezone_pattern() -> &'static str {
    r"(?i)\b(YEKT|YEKST|YAKT|YAKST|WITA|WIT|WIB|WGT|WGST|WFT|WET|WEST|WAT|WAST|VUT|VLAT|VLAST|VET|UZT|UYT|UYST|UTC|ULAT|TVT|TMT|TLT|TKT|TJT|TFT|TAHT|SST|SRT|SGT|SCT|SBT|SAST|SAMT|RET|PYT|PYST|PWT|PST|PONT|PMST|PMDT|PKT|PHT|PHOT|PGT|PETT|PETST|PET|PDT|OMST|OMSST|NZST|NZDT|NUT|NST|NPT|NOVT|NOVST|NFT|NDT|NCT|MYT|MVT|MUT|MST|MSK|MSD|MMT|MHT|MDT|MAWT|MART|MAGT|MAGST|LINT|LHST|LHDT|KUYT|KST|KRAT|KRAST|KGT|JST|IST|IRST|IRKT|IRKST|IRDT|IOT|IDT|ICT|HOVT|HKT|GYT|GST|GMT|GILT|GFT|GET|GAMT|GALT|FNT|FKT|FKST|FJT|FJST|EST|EGT|EGST|EET|EEST|EDT|ECT|EAT|EAST|EASST|DAVT|ChST|CXT|CVT|CST|COT|CLT|CLST|CKT|CHAST|CHADT|CET|CEST|CDT|CCT|CAT|CAST|BTT|BST|BRT|BRST|BOT|BNT|AZT|AZST|AZOT|AZOST|AWST|AWDT|AST|ART|AQTT|ANAT|ANAST|AMT|AMST|ALMT|AKST|AKDT|AFT|AEST|AEDT|ADT|ACST|ACDT)\b"
}

/// Create a Pattern from a regex string
pub fn pattern_regex(pattern: &'static str) -> Pattern {
    Pattern::Regex(Box::leak(Box::new(regex::Regex::new(pattern).unwrap())))
}

/// Create a time expression with hours and minutes
pub fn time_expr_with_minutes(hours: i64, minutes: i64, _latent: bool) -> Option<TimeExpr> {
    time_expr_with_hms(hours, minutes, 0)
}

/// Create a time expression with hours, minutes, and seconds
pub fn time_expr_with_hms(hours: i64, minutes: i64, seconds: i64) -> Option<TimeExpr> {
    if !(0..24).contains(&hours) || !(0..60).contains(&minutes) || !(0..60).contains(&seconds) {
        return None;
    }

    let time = chrono::NaiveTime::from_hms_opt(hours as u32, minutes as u32, seconds as u32)?;
    Some(TimeExpr::Intersect { expr: Box::new(TimeExpr::Reference), constraint: Constraint::TimeOfDay(time) })
}
