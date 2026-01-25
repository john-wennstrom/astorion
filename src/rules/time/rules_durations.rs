//! Duration-based rules (in X time, X ago, within X)

use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};

/// "in|within|after <duration>" (in 5 minutes, within 2 hours, after 3 days)
pub fn rule_duration_in_within_after() -> Rule {
    rule! {
        name: "in|within|after <duration>",
        pattern: [re!(r"(?i)(in|within|after)\s+(\d+)\s*(?:more\s+)?(seconds?|minutes?|hours?|days?|weeks?|months?|years?|h)")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let qualifier = groups.get(1)?.to_lowercase();
            let amount = groups.get(2)?.parse::<i32>().ok()?;
            let unit = groups.get(3)?.to_lowercase();

            let grain = match unit.as_str() {
                "second" | "seconds" => Grain::Second,
                "minute" | "minutes" => Grain::Minute,
                "hour" | "hours" | "h" => Grain::Hour,
                "day" | "days" => Grain::Day,
                "week" | "weeks" => Grain::Week,
                "month" | "months" => Grain::Month,
                "year" | "years" => Grain::Year,
                _ => return None,
            };

            match qualifier.as_str() {
                "within" => {
                    let shifted = shift_by_grain(TimeExpr::Reference, amount, grain);
                    let end_expr = match grain {
                        Grain::Week | Grain::Day => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Day,
                        },
                        Grain::Hour => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Hour,
                        },
                        Grain::Minute => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Minute,
                        },
                        _ => shifted,
                    };
                    Some(TimeExpr::IntervalBetween {
                        start: Box::new(TimeExpr::Reference),
                        end: Box::new(end_expr),
                    })
                }
                "after" => {
                    let shifted = shift_by_grain(TimeExpr::Reference, amount, grain);
                    let base_time = match grain {
                        Grain::Week => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Day,
                        },
                        Grain::Day => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Hour,
                        },
                        _ => shifted,
                    };
                    Some(TimeExpr::OpenAfter {
                        expr: Box::new(base_time),
                    })
                }
                "in" => {
                    let shifted = shift_by_grain(TimeExpr::Reference, amount, grain);
                    let expr = match grain {
                        Grain::Week => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Day,
                        },
                        Grain::Day => TimeExpr::StartOf {
                            expr: Box::new(shifted),
                            grain: Grain::Hour,
                        },
                        _ => shifted,
                    };
                    Some(expr)
                }
                _ => None,
            }
        }
    }
}

/// "in a/an <duration>" (in a day, in an hour)
pub fn rule_in_a_duration() -> Rule {
    rule! {
        name: "in a/an <duration>",
        pattern: [re!(r"(?i)in\s+(a|an|one)\s+(sec|second|seconds|minute|minutes|hour|hours|day|days|week|weeks|month|months|year|years)")],
        required_phrases: ["in"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let unit = groups.get(2)?.to_lowercase();
            let grain = match unit.as_str() {
                "sec" | "second" | "seconds" => Grain::Second,
                "minute" | "minutes" => Grain::Minute,
                "hour" | "hours" => Grain::Hour,
                "day" | "days" => Grain::Day,
                "week" | "weeks" => Grain::Week,
                "month" | "months" => Grain::Month,
                "year" | "years" => Grain::Year,
                _ => return None,
            };

            let expr = if matches!(grain, Grain::Day | Grain::Week | Grain::Month | Grain::Year) {
                let shifted = shift_by_grain(TimeExpr::Reference, 1, grain);
                TimeExpr::StartOf {
                    expr: Box::new(shifted),
                    grain: Grain::Hour,
                }
            } else {
                shift_by_grain(TimeExpr::Reference, 1, grain)
            };

            Some(expr)
        }
    }
}

/// "in <n> and a/an half hours" (in 2 and an half hours)
pub fn rule_in_n_and_a_half_hours() -> Rule {
    rule! {
        name: "in <n> and a/an half hours",
        pattern: [re!(r"(?i)in\s+(\d+)\s+and\s+a?n\s+half\s+hours?")],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let hours = groups.get(1)?.parse::<i32>().ok()?;
            if hours < 0 {
                return None;
            }

            let minutes = hours.saturating_mul(60).saturating_add(30);
            let expr = shift_by_grain(TimeExpr::Reference, minutes, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in <number> (minutes)" - defaults to minutes (in 5, in 30')
pub fn rule_in_number_minutes() -> Rule {
    rule! {
        name: "in <number> (defaults to minutes)",
        pattern: [re!(r"(?i)in\s+(\d{1,2})'?(?:\s+min(?:ute)?s?)?")],
        required_phrases: ["in"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let amount = regex_group_int_value(tokens.first()?, 1)? as i32;
            let expr = shift_by_grain(TimeExpr::Reference, amount, Grain::Minute);
            Some(expr)
        }
    }
}

/// "in a couple/pair/few of <duration>"
pub fn rule_in_couple_pair_few_duration() -> Rule {
    rule! {
        name: "in a couple/pair/few of <duration>",
        pattern: [re!(r"(?i)in\s+(?:a\s+)?(couple|pair|few)\s+(?:of\s+)?(seconds?|minutes?|hours?|days?|weeks?|months?|years?)")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let quantifier = groups.get(1)?.to_lowercase();
            let amount = match quantifier.as_str() {
                "couple" | "pair" => 2,
                "few" => 3,
                _ => return None,
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

            let expr = shift_by_grain(TimeExpr::Reference, amount, grain);
            Some(expr)
        }
    }
}

/// "in <text-number> <duration>" (in two hours, in five days)
pub fn rule_in_text_number_duration() -> Rule {
    rule! {
        name: "in <text-number> <duration>",
        pattern: [re!(r"(?i)in\s+(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let number = groups.get(1)?.to_lowercase();
            let amount = match number.as_str() {
                "one" => 1, "two" => 2, "three" => 3, "four" => 4,
                "five" => 5, "six" => 6, "seven" => 7, "eight" => 8,
                "nine" => 9, "ten" => 10, "eleven" => 11, "twelve" => 12,
                _ => return None,
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

            let expr = shift_by_grain(TimeExpr::Reference, amount, grain);
            Some(expr)
        }
    }
}

/// "<text-number> <duration> ago" (two hours ago, five days ago)
pub fn rule_text_number_duration_ago() -> Rule {
    rule! {
        name: "<text-number> <duration> ago",
        pattern: [re!(r"(?i)(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)\s+ago")],
        required_phrases: ["ago", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let number = groups.get(1)?.to_lowercase();
            let amount = match number.as_str() {
                "one" => -1, "two" => -2, "three" => -3, "four" => -4,
                "five" => -5, "six" => -6, "seven" => -7, "eight" => -8,
                "nine" => -9, "ten" => -10, "eleven" => -11, "twelve" => -12,
                _ => return None,
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

            let expr = shift_by_grain(TimeExpr::Reference, amount, grain);
            Some(expr)
        }
    }
}

/// "<number> <duration> ago" (5 minutes ago, 3 days ago)
pub fn rule_duration_ago() -> Rule {
    rule! {
        name: "<number> <duration> ago",
        pattern: [re!(r"(?i)(\d+)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)\s+ago")],
        required_phrases: ["ago"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount = -(groups.get(1)?.parse::<i32>().ok()?);
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

            let expr = shift_by_grain(TimeExpr::Reference, amount, grain);
            Some(expr)
        }
    }
}

/// "a couple/pair/few <duration> ago"
pub fn rule_couple_pair_few_duration_ago() -> Rule {
    rule! {
        name: "a couple/pair/few <duration> ago",
        pattern: [re!(r"(?i)(?:a\s+)?(couple|pair|few)\s+(?:of\s+)?(seconds?|minutes?|hours?|days?|weeks?|months?|years?)\s+ago")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let groups = match &tokens.first()?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let quantifier = groups.get(1)?.to_lowercase();
            let amount = match quantifier.as_str() {
                "couple" | "pair" => -2,
                "few" => -3,
                _ => return None,
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

            let expr = shift_by_grain(TimeExpr::Reference, amount, grain);
            Some(expr)
        }
    }
}
