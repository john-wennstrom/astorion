use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::{NumeralData, Rule, Token, TokenKind};

use crate::{
    rules::numeral::helpers::{
        decimals_to_double, first_match_lower, make_numeral, multiply_numerals, parse_decimal, parse_double,
    },
    rules::numeral::predicates::{
        has_grain, is_integer, is_multipliable, is_positive, number_between, tens_multiple_between_20_and_90,
    },
};

// Maps
/// Map of words for numbers 0..19 to their integer values.
static ZERO_NINETEEN_MAP: Lazy<HashMap<&'static str, i64>> = Lazy::new(|| {
    HashMap::from([
        ("naught", 0),
        ("nil", 0),
        ("nought", 0),
        ("none", 0),
        ("zero", 0),
        ("zilch", 0),
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("thirteen", 13),
        ("fourteen", 14),
        ("fifteen", 15),
        ("sixteen", 16),
        ("seventeen", 17),
        ("eighteen", 18),
        ("nineteen", 19),
    ])
});

/// Map of informal/fuzzy number expressions to integer values.
static INFORMAL_MAP: Lazy<HashMap<&'static str, i64>> = Lazy::new(|| {
    HashMap::from([
        ("single", 1),
        ("a couple", 2),
        ("a couple of", 2),
        ("couple", 2),
        ("couples", 2),
        ("couple of", 2),
        ("couples of", 2),
        ("a pair", 2),
        ("a pair of", 2),
        ("pair", 2),
        ("pairs", 2),
        ("pair of", 2),
        ("pairs of", 2),
        ("a few", 3),
        ("few", 3),
        ("a dozen", 12),
        ("a dozen of", 12),
    ])
});

/// Map of tens words (twenty, thirty, ...) to their numeric values.
static TENS_MAP: Lazy<HashMap<&'static str, i64>> = Lazy::new(|| {
    HashMap::from([
        ("twenty", 20),
        ("thirty", 30),
        ("forty", 40),
        ("fourty", 40),
        ("fifty", 50),
        ("sixty", 60),
        ("seventy", 70),
        ("eighty", 80),
        ("ninety", 90),
    ])
});

/// Map of power words to exponent values (e.g. "thousand" => 3).
static POWERS_OF_TENS_MAP: Lazy<HashMap<&'static str, i64>> = Lazy::new(|| {
    HashMap::from([
        ("hundred", 2),
        ("thousand", 3),
        ("l", 5),
        ("lac", 5),
        ("lak", 5),
        ("lakh", 5),
        ("lk", 5),
        ("lkh", 5),
        ("million", 6),
        ("cr", 7),
        ("crore", 7),
        ("koti", 7),
        ("billion", 9),
        ("trillion", 12),
    ])
});

// Rules (converted to Pattern/Rule form)

/// Rule matching integers/words in the 0..19 range and informal phrases.
fn rule_to_nineteen() -> Rule {
    rule! {
        name: "integer (0..19, informal)",
        pattern: [
            re!(r"(?i)(none|zilch|naught|nought|nil|zero|one|single|two|(a )?(pair|couple)s?( of)?|three|(a )?few|fourteen|four|fifteen|five|sixteen|six|seventeen|seven|eighteen|eight|nineteen|nine|ten|eleven|twelve|thirteen)")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            let m = first_match_lower(tokens)?;

            ZERO_NINETEEN_MAP
            .get(m.as_str())
            .copied()
            .or_else(|| INFORMAL_MAP.get(m.as_str()).copied())
            .map(|n| make_numeral(n as f64))
        },
    }
}

/// Rule matching twenty..ninety words.
fn rule_tens() -> Rule {
    rule! {
        name: "integer (20..90)",
        pattern: [
            re!(r"(?i)(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            let m = first_match_lower(tokens)?;

            TENS_MAP
            .get(m.as_str())
            .copied()
            .map(|n| make_numeral(n as f64))
        },
    }
}

/// Rule matching powers of ten (hundred, thousand, million, ...).
fn rule_powers_of_ten() -> Rule {
    rule! {
        name: "powers of tens",
        pattern: [
            re!(r"(?i)(hundred|thousand|l(?:ac|a?kh?|k)?|million|(?:k|c)r(?:ore)?|koti|billion)s?")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            let mut m = first_match_lower(tokens)?;

            // Because the regex allows `s?`, we normalize plurals just in case:
            if m.ends_with('s') {
                m.pop();
            }

            POWERS_OF_TENS_MAP
            .get(m.as_str())
            .copied()
            .map(|exp| make_numeral(10f64.powi(exp as i32)))
        },
    }
}

/// Rule matching composite tens (twenty one .. ninety nine).
fn rule_composite_tens() -> Rule {
    rule! {
        name: "integer 21..99",
        pattern: [
            pred!(tens_multiple_between_20_and_90),
            re!(r"(?i)[\s\-]+"),
            pred!(number_between::<1, 10>),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }

            match (&tokens[0].kind, &tokens[2].kind) {
                (TokenKind::Numeral(tens), TokenKind::Numeral(units)) => {
                    Some(make_numeral(tens.value + units.value))
                }
                _ => None,
            }
        },
    }
}

fn rule_skip_hundreds_1() -> Rule {
    rule! {
        name: "integer 100..999 without hundred (two tokens)",
        pattern: [
            re!(r"(?i)(one|two|three|four|five|six|seven|eight|nine)"),
            re!(r"(?i)[\s\-]+"),
            re!(r"(?i)(ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)"),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() != 3 { return None; }

            let get_lower = |idx: usize| -> Option<String> {
                match tokens.get(idx) {
                    Some(Token { kind: TokenKind::RegexMatch(groups), .. }) => groups.first().map(|s| s.to_lowercase()),
                    _ => None,
                }
            };

            let x1 = get_lower(0)?;
            let x2 = get_lower(2)?;

            let hundreds = ZERO_NINETEEN_MAP.get(x1.as_str())?;
            let rest = ZERO_NINETEEN_MAP
                .get(x2.as_str())
                .or_else(|| TENS_MAP.get(x2.as_str()))?;

            Some(make_numeral((*hundreds * 100 + rest) as f64))
        },
    }
}

fn rule_skip_hundreds_2() -> Rule {
    rule! {
        name: "integer 100..999 without hundred (three tokens)",
        pattern: [
            re!(r"(?i)(one|two|three|four|five|six|seven|eight|nine)"),
            re!(r"(?i)[\s\-]+"),
            re!(r"(?i)(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)"),
            re!(r"(?i)[\s\-]+"),
            re!(r"(?i)(one|two|three|four|five|six|seven|eight|nine)"),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() != 5 { return None; }

            let get_lower = |idx: usize| -> Option<String> {
                match tokens.get(idx) {
                    Some(Token { kind: TokenKind::RegexMatch(groups), .. }) => groups.first().map(|s| s.to_lowercase()),
                    _ => None,
                }
            };

            let x1 = get_lower(0)?;
            let x2 = get_lower(2)?;
            let x3 = get_lower(4)?;

            let hundreds = ZERO_NINETEEN_MAP.get(x1.as_str())?;
            let tens = TENS_MAP.get(x2.as_str())?;
            let rest = ZERO_NINETEEN_MAP.get(x3.as_str())?;

            Some(make_numeral((*hundreds * 100 + tens + rest) as f64))
        },
    }
}

fn rule_dot_spelled_out() -> Rule {
    rule! {
        name: "one point 2",
        pattern: [
            pred!(|t: &Token| matches!(t.kind, TokenKind::Numeral(_))),
            re!(r"(?i)\s*(point|dot)\s*"),
            pred!(|t: &Token| !has_grain(t)),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }

            match (&tokens[0].kind, &tokens[2].kind) {
                (TokenKind::Numeral(nd1), TokenKind::Numeral(nd2)) => Some(make_numeral(
                    nd1.value + decimals_to_double(nd2.value),
                )),
                _ => None,
            }
        },
    }
}

fn rule_leading_dot_spelled_out() -> Rule {
    rule! {
        name: "point 77",
        pattern: [
            re!(r"(?i)\s*(point|dot)\s*"),
            pred!(|t: &Token| !has_grain(t)),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 2 { return None; }

            match &tokens[1].kind {
                TokenKind::Numeral(nd) => Some(make_numeral(decimals_to_double(nd.value))),
                _ => None,
            }
        },
    }
}

fn rule_sum() -> Rule {
    rule! {
        name: "intersect 2 numbers",
        pattern: [
            pred!(|t: &Token| has_grain(t) && is_positive(t)),
            re!(r"(?i)\s*"),
            pred!(|t: &Token| !is_multipliable(t) && is_positive(t)),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }

            match (tokens.first(), tokens.last()) {
                (
                    Some(Token { kind: TokenKind::Numeral(NumeralData { value: val1, grain: Some(g), .. }), .. }),
                     Some(Token { kind: TokenKind::Numeral(NumeralData { value: val2, .. }), .. }),
                ) if 10_f64.powi(*g as i32) > *val2 => {
                    Some(make_numeral(val1 + val2))
                }
                _ => None,
            }
        },
    }
}

fn rule_sum_and() -> Rule {
    rule! {
        name: "intersect 2 numbers (with and)",
        pattern: [
            pred!(|t: &Token| has_grain(t) && is_positive(t)),
            re!(r"(?i)\s*and\s*"),
            pred!(|t: &Token| !is_multipliable(t) && is_positive(t)),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }

            match (tokens.first(), tokens.last()) {
                (
                     Some(Token { kind: TokenKind::Numeral(NumeralData { value: val1, grain: Some(g), .. }), .. }),
                     Some(Token { kind: TokenKind::Numeral(NumeralData { value: val2, .. }), .. }),
                ) if 10_f64.powi(*g as i32) > *val2 => {
                    Some(make_numeral(val1 + val2))
                }
                _ => None,
            }
        },
    }
}

/// Handle expressions like "X thousand and Y" by multiplying the leading
/// component by 1,000 and adding the remainder.
fn rule_thousand_and_remainder() -> Rule {
    rule! {
        name: "thousand and remainder",
        pattern: [
            pred!(|t: &Token| is_positive(t) && !is_multipliable(t)),
            re!(r"(?i)\s*thousand\s+and\s+"),
            pred!(|t: &Token| is_positive(t) && !is_multipliable(t)),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }

            match (tokens.first(), tokens.last()) {
                (
                     Some(Token { kind: TokenKind::Numeral(NumeralData { value: val1, .. }), .. }),
                     Some(Token { kind: TokenKind::Numeral(NumeralData { value: val2, .. }), .. }),
                ) if *val1 >= 1.0 && *val1 < 1000.0 && *val2 >= 0.0 && *val2 < 1000.0 => {
                    Some(make_numeral(val1 * 1000.0 + val2))
                }
                _ => None,
            }
        },
    }
}

fn rule_multiply() -> Rule {
    rule! {
        name: "compose by multiplication",
        pattern: [
            pred!(is_positive),
            re!(r"(?i)\s*"),
            pred!(is_multipliable),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }

            match (&tokens[0].kind, &tokens[2].kind) {
                (TokenKind::Numeral(nd1), TokenKind::Numeral(nd2)) => {
                    Some(multiply_numerals(nd1, nd2))
                }
                _ => None,
            }
        },
    }
}

fn rule_legal_parentheses() -> Rule {
    rule! {
        name: "<integer> '('<integer>')'",
        pattern: [
            pred!(|t: &Token| is_integer(t) && is_positive(t)),
            re!(r"\s*\("),
            pred!(|t: &Token| is_integer(t) && is_positive(t)),
            re!(r"\s*\)"),
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 4 { return None; }

            match (&tokens[0].kind, &tokens[2].kind) {
                (
                     TokenKind::Numeral(NumeralData { value: n1, .. }),
                     TokenKind::Numeral(NumeralData { value: n2, .. }),
                ) if (*n1 - *n2).abs() < f64::EPSILON => Some(make_numeral(*n1)),
                _ => None,
            }
        },
    }
}

/// Rule matching decimal numbers like `12.34`.
fn rule_decimals() -> Rule {
    rule! {
        name: "decimal number",
        pattern: [
            re!(r"(\d*\.\d+)")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.is_empty() { return None; }
            match &tokens[0].kind {
                TokenKind::RegexMatch(groups) => {
                    let s = groups.get(1).or_else(|| groups.first()).map(|s| s.as_str()).unwrap_or("");
                    parse_decimal(s).map(make_numeral)
                }
                _ => None,
            }
        },
    }
}

fn rule_fractions() -> Rule {
    rule! {
        name: "fractional number",
        pattern: [
            re!(r"(\d+)/(\d+)")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            match &tokens.first() {
                Some(Token { kind: TokenKind::RegexMatch(groups), .. }) => {
                    let numerator_str = groups.get(1).or_else(|| groups.first()).map(|s| s.as_str()).unwrap_or("");
                    let denominator_str = groups.get(2).or_else(|| groups.get(1)).map(|s| s.as_str()).unwrap_or("");

                    if let (Some(n), Some(d)) = (parse_decimal(numerator_str), parse_decimal(denominator_str)) {
                        if d.abs() > f64::EPSILON {
                            Some(make_numeral(n / d))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }
        },
    }
}

/// Rule matching plain integer digit sequences like `0`, `33`, `0033`.
fn rule_integers() -> Rule {
    rule! {
        name: "integer digits",
        pattern: [
            re!(r"(\d+)")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.is_empty() { return None; }
            match &tokens[0].kind {
                TokenKind::RegexMatch(groups) => {
                    let s = groups.get(1).or_else(|| groups.first()).map(|s| s.as_str()).unwrap_or("");
                    parse_double(s).map(make_numeral)
                }
                _ => None,
            }
        },
    }
}

/// Rule matching ordinal digit sequences like `1st`, `2nd`, `3rd`, `15th`.
fn rule_ordinal_digits() -> Rule {
    rule! {
        name: "ordinal digits",
        pattern: [re!(r"(?i)\b(\d+)(st|nd|rd|th)\b")],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.is_empty() {
                return None;
            }

            match &tokens[0].kind {
                TokenKind::RegexMatch(groups) => {
                    let digits = groups.get(1).or_else(|| groups.first()).map(|s| s.as_str())?;
                    parse_double(digits).map(make_numeral)
                }
                _ => None,
            }
        },
    }
}

/// Rule matching ordinal words like `first`, `second`, `third`, etc.
fn rule_ordinal_words() -> Rule {
    rule! {
        name: "ordinal words",
        pattern: [re!(r"(?i)\b(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth|thirteenth|fourteenth|fifteenth|sixteenth|seventeenth|eighteenth|nineteenth|twentieth|twenty-first|twenty-second|twenty-third|twenty-fourth|twenty-fifth|twenty-sixth|twenty-seventh|twenty-eighth|twenty-ninth|thirtieth|thirty-first)\b")],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.is_empty() {
                return None;
            }

            match &tokens[0].kind {
                TokenKind::RegexMatch(groups) => {
                    let word = groups.get(1).or_else(|| groups.first()).map(|s| s.to_lowercase())?;
                    let value = match word.as_str() {
                        "first" => 1.0,
                        "second" => 2.0,
                        "third" => 3.0,
                        "fourth" => 4.0,
                        "fifth" => 5.0,
                        "sixth" => 6.0,
                        "seventh" => 7.0,
                        "eighth" => 8.0,
                        "ninth" => 9.0,
                        "tenth" => 10.0,
                        "eleventh" => 11.0,
                        "twelfth" => 12.0,
                        "thirteenth" => 13.0,
                        "fourteenth" => 14.0,
                        "fifteenth" => 15.0,
                        "sixteenth" => 16.0,
                        "seventeenth" => 17.0,
                        "eighteenth" => 18.0,
                        "nineteenth" => 19.0,
                        "twentieth" => 20.0,
                        "twenty-first" => 21.0,
                        "twenty-second" => 22.0,
                        "twenty-third" => 23.0,
                        "twenty-fourth" => 24.0,
                        "twenty-fifth" => 25.0,
                        "twenty-sixth" => 26.0,
                        "twenty-seventh" => 27.0,
                        "twenty-eighth" => 28.0,
                        "twenty-ninth" => 29.0,
                        "thirtieth" => 30.0,
                        "thirty-first" => 31.0,
                        _ => return None,
                    };
                    Some(make_numeral(value))
                }
                _ => None,
            }
        },
    }
}

/// Rule matching comma-separated numbers like `1,234`.
fn rule_commas() -> Rule {
    rule! {
        name: "comma-separated numbers",
        pattern: [
            re!(r"(\d+(,\d\d\d)+(\.\d+)?)")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.is_empty() { return None; }
            match &tokens[0].kind {
                TokenKind::RegexMatch(groups) => {
                    let s = groups.get(1).or_else(|| groups.first()).map(|s| s.as_str()).unwrap_or("").replace(',', "");
                    parse_double(&s).map(make_numeral)
                }
                _ => None,
            }
        },
    }
}

/// Rule matching numeric suffixes such as `1.2k`, `3M`, `4g`.
fn rule_suffixes() -> Rule {
    rule! {
        name: "suffixes (K,M,G)",
        pattern: [
            // Support numbers with or without a leading zero before the decimal point
            // (e.g., ".0012G").
            re!(r"(?i)(\d+\.\d+|\d+|\.\d+)\s*([kmg])\b")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.is_empty() { return None; }
            match &tokens[0].kind {
                TokenKind::RegexMatch(groups) => {
                    let num_str = groups.get(1).or_else(|| groups.first()).map(|s| s.as_str()).unwrap_or("");
                    let suf_str = groups.get(2).or_else(|| groups.get(1)).map(|s| s.as_str().to_lowercase()).unwrap_or_default();
                    if let Some(mut base) = parse_double(num_str) {
                        let factor = match suf_str.as_str() {
                            "k" => 1e3,
                            "m" => 1e6,
                            "g" => 1e9,
                            _ => 1.0,
                        };
                        base *= factor;
                        Some(make_numeral(base))
                    } else { None }
                }
                _ => None,
            }
        },
    }
}

/// Rule handling explicit "minus/negative" prefixes directly in a single regex.
///
/// This covers inputs like "minus 1,200,000" where the sign and number appear
/// together in the source string, avoiding dependency on whitespace handling
/// between separate tokens.
fn rule_negative_prefix() -> Rule {
    rule! {
        name: "negative numbers (prefixed)",
        pattern: [
            re!(r"(?ix)
                (?:minus|negative)          # leading sign words
                \s*                         # optional whitespace
                (                           # capture the numeric portion
                    (?:\d{1,3}(?:,\d{3})+) # numbers with at least one comma
                    |\d+                    # or plain digits
                )
                (?:\.\d+)?                 # optional fractional part
                \b                          # stop before trailing digits/letters
            ")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            match tokens.first().map(|t| &t.kind) {
                Some(TokenKind::RegexMatch(groups)) => {
                    let num_str = groups
                        .get(1)
                        .or_else(|| groups.first())
                        .map(|s| s.as_str())
                        .unwrap_or("")
                        .replace(',', "");

                    parse_double(&num_str).map(|v| make_numeral(-v))
                }
                _ => None,
            }
        },
    }
}

/// Rule combining a leading negative sign/word with a numeral token.
fn rule_negative() -> Rule {
    rule! {
        name: "negative numbers",
        pattern: [
            re!(r"(?ix)
                (?:-\s*negative|-\s*minus|-)\s*   # hyphen-led signs allow tight binding
                |(?:\bminus\b|\bnegative\b)\s+    # word signs consume trailing space
            "),
            pred!(is_positive)
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            // tokens[0] is the regex match (sign/word), tokens[1] is a Numeral token
            if tokens.len() < 2 { return None; }
            match &tokens[1].kind {
                TokenKind::Numeral(nd) => Some(make_numeral(-nd.value)),
                _ => None,
            }
        },
    }
}

fn rule_negative_words() -> Rule {
    rule! {
        name: "negative numbers (words)",
        pattern: [
            re!(r"(?i)(minus|negative)"),
            re!(r"\s+"),
            pred!(is_positive)
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 3 { return None; }
            match &tokens[2].kind {
                TokenKind::Numeral(nd) => Some(make_numeral(-nd.value)),
                _ => None,
            }
        },
    }
}

fn rule_dozen() -> Rule {
    rule! {
        name: "a dozen of",
        pattern: [
            re!(r"(?i)(?:a\s+)?dozens?(?:\s+of)?")
        ],
        prod: |_tokens: &[Token]| -> Option<NumeralData> {
            Some(make_numeral(12.0))
        },
    }
}

fn rule_dozen_multiplication() -> Rule {
    rule! {
        name: "dozen as multiplier",
        pattern: [
            pred!(|t: &Token| matches!(t.kind, TokenKind::Numeral(_))),
            re!(r"(?i)\s*dozens?")
        ],
        prod: |tokens: &[Token]| -> Option<NumeralData> {
            if tokens.len() < 2 { return None; }
            match (&tokens[0].kind, &tokens[1].kind) {
                (TokenKind::Numeral(base), TokenKind::RegexMatch(_)) => {
                    Some(make_numeral(base.value * 12.0))
                }
                _ => None,
            }
        },
    }
}

pub fn get() -> Vec<Rule> {
    vec![
        rule_ordinal_digits(),
        rule_ordinal_words(),
        rule_integers(),
        rule_to_nineteen(),
        rule_tens(),
        rule_powers_of_ten(),
        rule_composite_tens(),
        rule_skip_hundreds_1(),
        rule_skip_hundreds_2(),
        rule_decimals(),
        rule_fractions(),
        rule_commas(),
        rule_suffixes(),
        rule_dot_spelled_out(),
        rule_leading_dot_spelled_out(),
        rule_multiply(),
        rule_sum(),
        rule_sum_and(),
        rule_thousand_and_remainder(),
        rule_negative_prefix(),
        rule_negative(),
        rule_negative_words(),
        rule_legal_parentheses(),
        rule_dozen(),
        rule_dozen_multiplication(),
    ]
}
