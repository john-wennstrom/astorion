//! Trigger scanning (input pre-classification).
//!
//! This module inspects the raw input string and produces coarse signals that
//! let the parser quickly decide which rules should be considered.
//!
//! The scan produces two kinds of signals:
//!
//! - **Buckets** (`BucketMask`): cheap booleans derived from the input such as
//!   “contains digits” or “looks month-like”. These are used to enable bucketed
//!   rules via `RuleIndex::by_bucket`.
//! - **Phrases** (`TriggerInfo::phrases`): a set of lowercased key phrases
//!   discovered in the input (e.g. "tomorrow", "between", "weekend"). These are
//!   used for phrase gating in `Parser::new_compiled`.
//!
//! ## Design notes
//!
//! - This is a *heuristic* scan. False positives are acceptable because the
//!   downstream parser still has to match full rule patterns.
//! - For now the scan uses ASCII lowercasing and simple tokenization because
//!   current rules are English-only. When adding non-English locales, consider
//!   locale-aware case folding and tokenization.
//!
//! ## Extension points
//!
//! - Adding new buckets/phrases is allowed, but keep the scan cheap: the goal is
//!   to reduce the active rule set without making the scan itself expensive.

use super::compiled_rules::BucketMask;
use std::collections::HashSet;

/// Input characteristics detected from the raw input.
///
/// This is used to quickly gate rule activation before saturation.
#[derive(Debug, Clone)]
pub struct TriggerInfo {
    pub buckets: BucketMask,
    pub phrases: HashSet<String>,
}

impl TriggerInfo {
    /// Scan `input` for coarse buckets and key phrases.
    ///
    /// Note: uses `to_ascii_lowercase()` since all current triggers are ASCII English.
    /// When adding non-English locales (Swedish, Russian, etc.), this should become
    /// locale-aware or switch to `to_lowercase()`.
    pub fn scan(input: &str) -> Self {
        let mut buckets = BucketMask::empty();
        let mut phrases = HashSet::new();
        let lower = input.to_ascii_lowercase();

        // Buckets
        if input.bytes().any(|b| b.is_ascii_digit()) {
            buckets |= BucketMask::HAS_DIGITS;
        }

        if input.contains(':') {
            buckets |= BucketMask::HAS_COLON;
        }

        // AM/PM with crude word boundary checks
        if lower.contains("am") || lower.contains("a.m") {
            buckets |= BucketMask::HAS_AMPM;
        }
        if lower.contains("pm") || lower.contains("p.m") {
            buckets |= BucketMask::HAS_AMPM;
        }

        // Weekday detection (singular + common plural forms)
        const WEEKDAYS: &[&str] = &[
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
            "mondays",
            "tuesdays",
            "wednesdays",
            "thursdays",
            "fridays",
            "saturdays",
            "sundays",
            "mon",
            "tue",
            "wed",
            "thu",
            "fri",
            "sat",
            "sun",
        ];
        for wd in WEEKDAYS {
            if lower.split_whitespace().any(|w| w.trim_matches(|c: char| !c.is_alphabetic()) == *wd) {
                buckets |= BucketMask::WEEKDAYISH;
                break;
            }
        }

        // Month detection
        const MONTHS: &[&str] = &[
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
            "jan",
            "feb",
            "mar",
            "apr",
            "jun",
            "jul",
            "aug",
            "sep",
            "oct",
            "nov",
            "dec",
        ];
        for month in MONTHS {
            if lower.split_whitespace().any(|w| w.trim_matches(|c: char| !c.is_alphabetic()) == *month) {
                buckets |= BucketMask::MONTHISH;
                break;
            }
        }

        // Ordinal detection
        const ORDINALS: &[&str] = &[
            "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "1st",
            "2nd", "3rd", "4th", "5th",
        ];
        for ord in ORDINALS {
            if lower.split_whitespace().any(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric()) == *ord) {
                buckets |= BucketMask::ORDINALISH;
                break;
            }
        }

        // Key phrases
        const KEY_PHRASES: &[&str] = &[
            "tomorrow",
            "yesterday",
            "today",
            "next",
            "last",
            "this",
            "now",
            "from",
            "by",
            "to",
            "until",
            "through",
            "thru",
            "between",
            "after",
            "before",
            "since",
            "eod",
            "eom",
            "bom",
            "month",
            "before last",
            "after next",
            "at",
            "on",
            "in",
            "for",
            "of",
            "ago",
            "hence",
            "back",
            "following",
            "thanksgiving",
            "christmas",
            "xmas",
            "boss",
            "black",
            "friday",
            "mlk",
            "martin",
            "new",
            "year",
            "eve",
            "summer",
            "fall",
            "autumn",
            "winter",
            "spring",
            "asap",
            "soon",
            "immediately",
            "moment",
            "atm",
            "ides",
            "ide",
            "tmrw",
            "tommorow",
            "tomorrows",
            "ystrday",
            "yestrday",
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
            "mon",
            "tue",
            "wed",
            "thu",
            "fri",
            "sat",
            "sun",
            "week",
            "weekend",
            "wkend",
            "month",
            "quarter",
            "qtr",
            "qr",
            "half",
            "past",
            "after",
            "to",
            "till",
            "through",
            "thru",
            "before",
            "of",
            "day",
            "hour",
            "minute",
            "second",
            "noon",
            "midnight",
            "midnite",
            "mid",
            "eod",
            "end",
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
            "morning",
            "afternoon",
            "evening",
            "night",
            "tonight",
            "late",
            "early",
            "mid",
            "beginning",
        ];
        for phrase in KEY_PHRASES {
            if phrase.contains(' ') {
                // For multi-word phrases like "before last" or "after next",
                // do a simple substring match on the lowercased input.
                if lower.contains(phrase) {
                    phrases.insert(phrase.to_string());
                }
            } else {
                // For single-word phrases, match against normalized whitespace tokens.
                if lower.split_whitespace().any(|w| w.trim_matches(|c: char| !c.is_alphabetic()) == *phrase) {
                    phrases.insert(phrase.to_string());
                }
            }
        }

        TriggerInfo { buckets, phrases }
    }
}
