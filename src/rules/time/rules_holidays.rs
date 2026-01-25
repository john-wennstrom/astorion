//! Holiday-specific rules

use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};
use chrono::{Datelike, Weekday};

/// "Thanksgiving" - 4th Thursday of November
pub fn rule_thanksgiving() -> Rule {
    rule! {
        name: "thanksgiving",
        pattern: [re!(r"(?i)thanksgiving(?:\s+day)?")],
        required_phrases: ["thanksgiving"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::NthWeekdayOfMonth {
                n: 4,
                year: None,
                month: 11,
                weekday: Weekday::Thu,
            })
        }
    }
}

/// "Boss's Day" - October 16
pub fn rule_bosss_day() -> Rule {
    rule! {
        name: "boss's day",
        pattern: [re!(r"(?i)boss'?s?(?:\s+day)?(?:\s+(\d{4}))?")],
        required_phrases: ["boss"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let year = groups.get(1)
                .and_then(|s| if s.is_empty() { None } else { s.parse::<i32>().ok() });

            if let Some(y) = year {
                let base_date = chrono::NaiveDate::from_ymd_opt(y, 10, 16)?;
                let actual_date = match base_date.weekday() {
                    chrono::Weekday::Sat => base_date.pred_opt()?,
                    chrono::Weekday::Sun => base_date.succ_opt()?,
                    _ => base_date,
                };
                Some(TimeExpr::Absolute {
                    year: actual_date.year(),
                    month: actual_date.month(),
                    day: actual_date.day(),
                    hour: None,
                    minute: None,
                })
            } else {
                Some(TimeExpr::MonthDay { month: 10, day: 16 })
            }
        }
    }
}

/// "MLK day" - 3rd Monday of January
pub fn rule_mlk_day() -> Rule {
    rule! {
        name: "MLK day",
        pattern: [re!(r"(?i)(?:(last|next|this)\s+)?(?:martin\s+luther\s+king(?:\s+jr\.?)?(?:\s+day)?|MLK(?:\s+(?:jr\.?))?(?:\s+day)?|civil\s+rights\s+day)(?:\s+(?:of\s+)?(last\s+year|(\d{4})))?")],
        required_phrases: ["martin", "luther", "king", "mlk", "civil"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let mut modifier = None;
            let mut year_str = None;
            let mut is_last_year = false;

            for group in groups.iter().skip(1) {
                let lower = group.to_lowercase();
                if lower == "last" || lower == "next" || lower == "this" {
                    modifier = Some(lower);
                } else if lower == "last year" {
                    is_last_year = true;
                } else if group.chars().all(|c| c.is_ascii_digit()) {
                    year_str = Some(group.as_str());
                }
            }

            let year = if is_last_year {
                Some(-1)
            } else {
                year_str.and_then(|s| s.parse::<i32>().ok())
            };

            let expr = if modifier.as_deref() == Some("last") && !is_last_year && year.is_none() {
                let base = TimeExpr::NthWeekdayOfMonth {
                    n: 3,
                    year: None,
                    month: 1,
                    weekday: Weekday::Mon,
                };
                shift_by_grain(base, -1, Grain::Year)
            } else {
                TimeExpr::NthWeekdayOfMonth {
                    n: 3,
                    year,
                    month: 1,
                    weekday: Weekday::Mon,
                }
            };

            Some(expr)
        }
    }
}

/// "Black Friday" - day after Thanksgiving
pub fn rule_black_friday() -> Rule {
    rule! {
        name: "black friday",
        pattern: [re!(r"(?i)black\s+friday(?:\s+(?:of\s+)?(?:this\s+)?year)?(?:\s+(\d{4}))?")],
        required_phrases: ["black", "friday"],
        buckets: BucketMask::WEEKDAYISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let year = groups.get(1)
                .and_then(|s| if s.is_empty() { None } else { s.parse::<i32>().ok() });

            Some(TimeExpr::LastWeekdayOfMonth {
                year,
                month: 11,
                weekday: Weekday::Fri,
            })
        }
    }
}
