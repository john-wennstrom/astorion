use crate::Dimension;
use crate::TokenKind;
/// Cycle-based rules (this/last/next year/month/week/day/quarter)
use crate::engine::BucketMask;
use crate::rules::numeral::helpers::first_match_lower;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token};

/// "this|last|next <cycle>" (this year, next month, last week)
pub fn rule_cycle_this_last_next() -> Rule {
    rule! {
        name: "this|last|next <cycle>",
        pattern: [
            re!(r"(?i)(this|current|coming|next|(the( following)?)|last|past|previous|upcoming)\s+"),
            re!(r"(?i)(year|yr|quarter|qtr|month|week|day)\b"),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let qualifier_raw = first(tokens)?;
            let qualifier = qualifier_raw.trim();
            let grain_raw = first(&tokens[1..])?;
            let grain_str = grain_raw.trim();

            let grain_normalized = match grain_str {
                "yr" => "year",
                "qtr" => "quarter",
                other => other,
            };

            let grain = grain_from_cycle(grain_normalized)?;

            let amount = match qualifier {
                "this" | "current" | "the" => 0,
                "coming" | "next" | "upcoming" | "the following" => 1,
                "last" | "past" | "previous" => -1,
                _ => return None,
            };

            let base = if amount == 0 {
                TimeExpr::Reference
            } else {
                shift_by_grain(TimeExpr::Reference, amount, grain)
            };

            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(base),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(base),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "this|last|next qtr" - special handling for quarter abbreviation
pub fn rule_cycle_this_last_next_qtr() -> Rule {
    rule! {
        name: "this|last|next qtr",
        pattern: [
            re!(r"(?i)(this|current|coming|next|last|past|previous|upcoming)\s+"),
            re!(r"(?i)qtr(s)?\b|qr\b"),
        ],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let qualifier_raw = first(tokens)?;
            let qualifier = qualifier_raw.trim();
            let grain = Grain::Quarter;

            let amount = match qualifier {
                "this" | "current" => 0,
                "coming" | "next" | "upcoming" => 1,
                "last" | "past" | "previous" => -1,
                _ => return None,
            };

            let base = if amount == 0 {
                TimeExpr::Reference
            } else {
                shift_by_grain(TimeExpr::Reference, amount, grain)
            };

            Some(TimeExpr::StartOf {
                expr: Box::new(base),
                grain,
            })
        }
    }
}

/// "upcoming <grain>" (upcoming week, upcoming month)
pub fn rule_upcoming_grain() -> Rule {
    rule! {
        name: "upcoming <cycle>",
        pattern: [re!(r"(?i)upcoming\s+(year|quarter|month|week|day)s?\b")],
        required_phrases: ["upcoming", "year", "quarter", "month", "week", "day"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let grain_str = first_match_lower(tokens)?;
            let grain_word = grain_str.split_whitespace().nth(1)?;
            let grain = grain_from_cycle(grain_word)?;

            let shifted = shift_by_grain(TimeExpr::Reference, 1, grain);
            let expr = if grain == Grain::Week {
                TimeExpr::IntervalOf {
                    expr: Box::new(shifted),
                    grain,
                }
            } else {
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain,
                }
            };

            Some(expr)
        }
    }
}

/// "upcoming qtr"
pub fn rule_upcoming_grain_alt() -> Rule {
    rule! {
        name: "upcoming qtr",
        pattern: [re!(r"(?i)upcoming\s+qtr\b")],
        required_phrases: ["upcoming", "qtr"],
        buckets: BucketMask::empty().bits(),
        prod: |_tokens: &[Token]| -> Option<TimeExpr> {
            let grain = Grain::Quarter;
            let shifted = shift_by_grain(TimeExpr::Reference, 1, grain);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain,
            })
        }
    }
}

/// "upcoming two weeks", "upcoming 2 months"
pub fn rule_upcoming_n_cycles() -> Rule {
    rule! {
        name: "upcoming <integer> <cycle>",
        pattern: [
            re!(r"(?i)upcoming\s+"),
            pred!(|t: &Token| crate::rules::numeral::predicates::is_integer(t)),
            re!(r"\s+"),
            re!(r"(?i)(year|quarter|month|week|day)s?\b"),
        ],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Numeral],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n = integer_value(tokens.get(1)?)? as i32;
            if n <= 0 {
                return None;
            }

            let grain_str = match &tokens.get(3)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let grain = grain_from_cycle(grain_str.trim())?;

            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain,
            };

            let expr = shift_by_grain(base, n, grain);
            Some(expr)
        }
    }
}

/// "two upcoming weeks", "2 upcoming quarters"
pub fn rule_n_upcoming_cycles() -> Rule {
    rule! {
        name: "<integer> upcoming <cycle>",
        pattern: [
            pred!(|t: &Token| crate::rules::numeral::predicates::is_integer(t)),
            re!(r"\s+"),
            re!(r"(?i)upcoming\s+"),
            re!(r"(?i)(year|quarter|month|week|day)s?\b"),
        ],
        buckets: BucketMask::empty().bits(),
        deps: [Dimension::Numeral],
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n = integer_value(tokens.first()?)? as i32;
            if n <= 0 {
                return None;
            }

            let grain_str = match &tokens.get(3)?.kind {
                TokenKind::RegexMatch(groups) => groups.get(1)?,
                _ => return None,
            };
            let grain = grain_from_cycle(grain_str.trim())?;

            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain,
            };

            let expr = shift_by_grain(base, n, grain);
            Some(expr)
        }
    }
}

/// "<ordinal> quarter" (first quarter, second quarter, Q1, Q2)
pub fn rule_cycle_ordinal_quarter() -> Rule {
    rule! {
        name: "<ordinal> quarter",
        pattern: [re!(r"(?i)(first|second|third|fourth|1st|2nd|3rd|4th)\s+quarter\b")],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let ordinal = matched.split_whitespace().next()?;

            let n = match ordinal {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                _ => return None,
            };

            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            let shifted = shift_by_grain(base, n - 1, Grain::Quarter);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Quarter,
            })
        }
    }
}

/// "Q<number>" (Q1, Q2, Q3, Q4)
pub fn rule_cycle_numeral_quarter() -> Rule {
    rule! {
        name: "Q<number>",
        pattern: [re!(r"(?i)Q([1-4])\b")],
        required_phrases: [],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let n = regex_group_int_value(tokens.first()?, 1)? as i32;
            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            let shifted = shift_by_grain(base, n - 1, Grain::Quarter);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Quarter,
            })
        }
    }
}

/// "<ordinal> qtr" (1st qtr, 2nd qtr)
pub fn rule_cycle_ordinal_qtr() -> Rule {
    rule! {
        name: "<ordinal> qtr",
        pattern: [re!(r"(?i)(first|second|third|fourth|1st|2nd|3rd|4th)\s+qtr\b")],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let ordinal = matched.split_whitespace().next()?;

            let n = match ordinal {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                _ => return None,
            };

            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            let shifted = shift_by_grain(base, n - 1, Grain::Quarter);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Quarter,
            })
        }
    }
}

/// "the <ordinal> quarter" (the first quarter, the second quarter)
pub fn rule_cycle_the_ordinal_quarter() -> Rule {
    rule! {
        name: "the <ordinal> quarter",
        pattern: [re!(r"(?i)the\s+(first|second|third|fourth|1st|2nd|3rd|4th)\s+quarter\b")],
        buckets: BucketMask::ORDINALISH.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let words: Vec<&str> = matched.split_whitespace().collect();
            let ordinal = words.get(1)?;

            let n = match *ordinal {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                _ => return None,
            };

            let base = TimeExpr::StartOf {
                expr: Box::new(TimeExpr::Reference),
                grain: Grain::Year,
            };
            let shifted = shift_by_grain(base, n - 1, Grain::Quarter);
            Some(TimeExpr::StartOf {
                expr: Box::new(shifted),
                grain: Grain::Quarter,
            })
        }
    }
}

/// "<ordinal> quarter <year>" (first quarter 2024)
pub fn rule_cycle_ordinal_quarter_year() -> Rule {
    rule! {
        name: "<ordinal> quarter <year>",
        pattern: [re!(r"(?i)(first|second|third|fourth|1st|2nd|3rd|4th)\s+quarter\s+(\d{4})\b")],
        buckets: (BucketMask::ORDINALISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let parts: Vec<&str> = matched.split_whitespace().collect();
            let ordinal = parts.first()?;
            let year_str = parts.get(2)?;

            let n = match *ordinal {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                _ => return None,
            };

            let year = year_str.parse::<i32>().ok()?;
            let start_month = (n - 1) * 3 + 1;
            Some(TimeExpr::Absolute {
                year,
                month: start_month,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}

/// "<ordinal> qtr <year>"
pub fn rule_cycle_ordinal_qtr_year() -> Rule {
    rule! {
        name: "<ordinal> qtr <year>",
        pattern: [re!(r"(?i)(first|second|third|fourth|1st|2nd|3rd|4th)\s+qtr\s+(\d{4})\b")],
        buckets: (BucketMask::ORDINALISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let parts: Vec<&str> = matched.split_whitespace().collect();
            let ordinal = parts.first()?;
            let year_str = parts.get(2)?;

            let n = match *ordinal {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                _ => return None,
            };

            let year = year_str.parse::<i32>().ok()?;
            let start_month = (n - 1) * 3 + 1;
            Some(TimeExpr::Absolute {
                year,
                month: start_month,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}

/// "the <ordinal> qtr of <year>"
pub fn rule_cycle_the_ordinal_qtr_of_year() -> Rule {
    rule! {
        name: "the <ordinal> qtr of <year>",
        pattern: [re!(r"(?i)the\s+(first|second|third|fourth|1st|2nd|3rd|4th)\s+qtr\s+of\s+(\d{4})\b")],
        buckets: (BucketMask::ORDINALISH | BucketMask::HAS_DIGITS).bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let matched = first_match_lower(tokens)?;
            let parts: Vec<&str> = matched.split_whitespace().collect();
            let ordinal = parts.get(1)?;
            let year_str = parts.get(4)?;

            let n = match *ordinal {
                "first" | "1st" => 1,
                "second" | "2nd" => 2,
                "third" | "3rd" => 3,
                "fourth" | "4th" => 4,
                _ => return None,
            };

            let year = year_str.parse::<i32>().ok()?;
            let start_month = (n - 1) * 3 + 1;
            Some(TimeExpr::Absolute {
                year,
                month: start_month,
                day: 1,
                hour: None,
                minute: None,
            })
        }
    }
}
