//! Season-based rules

use crate::engine::BucketMask;
use crate::rules::time::helpers::*;
use crate::time_expr::TimeExpr;
use crate::{Rule, Token, TokenKind};

/// "summer", "fall", "winter", "spring", "autumn"
pub fn rule_season() -> Rule {
    rule! {
        name: "season",
        pattern: [re!(r"(?i)(summer|fall|autumn|winter|spring)")],
        optional_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let season = season_from_text(tokens.first()?)?;
            Some(TimeExpr::Season(season))
        }
    }
}

/// "this summer", "last winter", "next spring"
pub fn rule_modifier_season() -> Rule {
    rule! {
        name: "this/last/next <season>",
        pattern: [
            re!(r"(?i)(this|last|next|coming|past)\s+"),
            re!(r"(?i)(summer|fall|autumn|winter|spring)")
        ],
        optional_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let _modifier = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let season = season_from_text(tokens.get(1)?)?;
            let expr = TimeExpr::Season(season);

            // TODO: Apply modifier shift
            Some(expr)
        }
    }
}

/// "this season", "last season", "next season" (also plural "seasons")
pub fn rule_relative_season() -> Rule {
    rule! {
        name: "this/last/next season",
        pattern: [re!(r"(?i)\b(this|current|last|next|coming|past|previous)\s+seasons?\b")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let modifier = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?.to_lowercase(),
                _ => return None,
            };

            let offset = match modifier.as_str() {
                "this" | "current" => 0,
                "next" | "coming" => 1,
                "last" | "past" | "previous" => -1,
                _ => return None,
            };

            Some(TimeExpr::SeasonPeriod { offset })
        }
    }
}

/// "Christmas", "Xmas"
pub fn rule_christmas() -> Rule {
    rule! {
        name: "Christmas",
        pattern: [re!(r"(?i)(christmas|xmas)")],
        optional_phrases: ["christmas", "xmas"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::MonthDay { month: 12, day: 25 })
        }
    }
}

/// "Christmas Eve"
pub fn rule_christmas_eve() -> Rule {
    rule! {
        name: "Christmas Eve",
        pattern: [re!(r"(?i)(christmas|xmas)\s+eve")],
        optional_phrases: ["christmas", "xmas", "eve"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::MonthDay { month: 12, day: 24 })
        }
    }
}

/// "New Year's", "New Year's Day"
pub fn rule_new_years() -> Rule {
    rule! {
        name: "New Year's",
        pattern: [re!(r"(?i)new\s+year'?s?(\s+day)?")],
        required_phrases: ["new", "year"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::MonthDay { month: 1, day: 1 })
        }
    }
}

/// "New Year's Eve"
pub fn rule_new_years_eve() -> Rule {
    rule! {
        name: "New Year's Eve",
        pattern: [re!(r"(?i)new\s+year'?s?\s+eve")],
        required_phrases: ["new", "year", "eve"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            Some(TimeExpr::MonthDay { month: 12, day: 31 })
        }
    }
}
