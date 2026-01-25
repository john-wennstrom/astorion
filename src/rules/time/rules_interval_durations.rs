//! Interval duration rules (for duration from time, last/next X, from-to patterns)

use crate::engine::BucketMask;
use crate::rules::time::helpers::shift::shift_by_grain;
use crate::rules::time::helpers::*;
use crate::rules::time::predicates::*;
use crate::time_expr::{Grain, TimeExpr};
use crate::{Rule, Token, TokenKind};

/// "for <duration> from <time>" (for 2 hours from 3pm)
pub fn rule_interval_for_duration_from() -> Rule {
    rule! {
        name: "for <duration> from <time>",
        pattern: [re!(r"(?i)for\s+"), pattern_regex(duration_pattern()), re!(r"\s+(from|starting\s+from|starting|beginning|after)\s+"), pred!(is_time_expr)],
        required_phrases: [],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let (amount, grain) = parse_duration(tokens.get(1)?)?;
            let time_expr = get_time_expr(tokens.get(3)?)?;

            let end_expr = shift_by_grain(time_expr.clone(), amount + 1, grain);
            Some(TimeExpr::IntervalBetween {
                start: Box::new(time_expr.clone()),
                end: Box::new(end_expr),
            })
        }
    }
}

/// "<time> for <duration>" (3pm for 2 hours, Monday for 3 days)
pub fn rule_interval_time_for_duration() -> Rule {
    rule! {
        name: "<time> for <duration>",
        pattern: [pred!(is_time_expr), re!(r"(?i)\s+for\s+"), pattern_regex(duration_pattern())],
        required_phrases: ["for"],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.first()?)?;
            let (amount, grain) = parse_duration(tokens.get(2)?)?;

            let end_expr = shift_by_grain(time_expr.clone(), amount + 1, grain);
            Some(TimeExpr::IntervalBetween {
                start: Box::new(time_expr.clone()),
                end: Box::new(end_expr),
            })
        }
    }
}

/// "from <time> for <duration>" (from 3pm for 2 hours)
pub fn rule_interval_from_time_for_duration() -> Rule {
    rule! {
        name: "from <time> for <duration>",
        pattern: [re!(r"(?i)(from|starting|beginning|after|starting from)"), pred!(is_time_expr), re!(r"(?i)for"), pattern_regex(duration_pattern())],
        required_phrases: [],
        buckets: BucketMask::HAS_DIGITS.bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?;
            let (amount, grain) = parse_duration(tokens.get(3)?)?;

            let end_expr = shift_by_grain(time_expr.clone(), amount + 1, grain);
            Some(TimeExpr::IntervalBetween {
                start: Box::new(time_expr.clone()),
                end: Box::new(end_expr),
            })
        }
    }
}

/// "from <time> for <text-duration>" (from 3pm for two hours)
pub fn rule_interval_from_time_for_text_duration() -> Rule {
    rule! {
        name: "from <time> for <text-duration>",
        pattern: [re!(r"(?i)(from|starting|beginning)\s+"), pred!(is_time_expr), re!(r"\s+for\s+(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|forty|fifty)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)")],
        required_phrases: [],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let time_expr = get_time_expr(tokens.get(1)?)?;

            let groups = match &tokens.get(2)?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let number = groups.get(1)?.to_lowercase();
            let amount = match number.as_str() {
                "one" => 1,
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

            let end_expr = shift_by_grain(time_expr.clone(), amount + 1, grain);
            Some(TimeExpr::IntervalBetween {
                start: Box::new(time_expr.clone()),
                end: Box::new(end_expr),
            })
        }
    }
}

/// "last|past|next <duration>" (last 2 hours, next 3 days, past 5 minutes)
pub fn rule_duration_last_next() -> Rule {
    rule! {
        name: "last|past|next <duration>",
        pattern: [re!(r"(?i)(last|past|next)\s+"), re!(r"(\d+|an?|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|few|couple)\s+(seconds?|minutes?|hours?|days?|weeks?|months?|years?)")],
        buckets: BucketMask::empty().bits(),
        prod: |tokens: &[Token]| -> Option<TimeExpr> {
            let qualifier = first(tokens)?.trim().to_lowercase();

            let groups = match &tokens.get(1)?.kind {
                TokenKind::RegexMatch(groups) => groups,
                _ => return None,
            };

            let amount_str = groups.get(1)?.to_lowercase();
            let amount = match amount_str.as_str() {
                "a" | "an" | "one" => 1,
                "two" | "couple" => 2,
                "three" | "few" => 3,
                "four" => 4,
                "five" => 5,
                "six" => 6,
                "seven" => 7,
                "eight" => 8,
                "nine" => 9,
                "ten" => 10,
                "eleven" => 11,
                "twelve" => 12,
                _ => amount_str.parse::<i32>().ok()?,
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

            let expr = match qualifier.as_str() {
                "last" | "past" => {
                    // "last 2 seconds" means interval from 2 seconds ago until now
                    let start_shift = shift_by_grain(TimeExpr::Reference, -amount, grain);

                    // For larger grains (hour+), round to grain boundaries
                    let (start, end) = match grain {
                        Grain::Hour | Grain::Day | Grain::Week | Grain::Month | Grain::Year => {
                            let rounded_end = TimeExpr::StartOf {
                                expr: Box::new(TimeExpr::Reference),
                                grain,
                            };
                            let rounded_start = TimeExpr::StartOf {
                                expr: Box::new(start_shift),
                                grain,
                            };
                            (rounded_start, rounded_end)
                        }
                        _ => (start_shift, TimeExpr::Reference),
                    };

                    TimeExpr::IntervalBetween {
                        start: Box::new(start),
                        end: Box::new(end),
                    }
                }
                "next" => {
                    // "next 3 seconds" means the 3 seconds after this moment
                    let (start, end) = match grain {
                        Grain::Hour | Grain::Day | Grain::Week | Grain::Month | Grain::Year => {
                            // Round to next grain boundary and count from there
                            let rounded_start_base = TimeExpr::StartOf {
                                expr: Box::new(TimeExpr::Reference),
                                grain,
                            };
                            let rounded_start = shift_by_grain(rounded_start_base, 1, grain);
                            let rounded_end = shift_by_grain(rounded_start.clone(), amount, grain);
                            (rounded_start, rounded_end)
                        }
                        _ => {
                            // For seconds/minutes, start from next unit
                            let start = shift_by_grain(TimeExpr::Reference, 1, grain);
                            let end = shift_by_grain(TimeExpr::Reference, amount + 1, grain);
                            (start, end)
                        }
                    };

                    TimeExpr::IntervalBetween {
                        start: Box::new(start),
                        end: Box::new(end),
                    }
                }
                _ => return None,
            };
            Some(expr)
        }
    }
}
