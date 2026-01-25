use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

use crate::time_expr::{Grain, TimeExpr};

pub fn shift_by_grain(expr: TimeExpr, amount: i32, grain: Grain) -> TimeExpr {
    TimeExpr::Shift { expr: Box::new(expr), amount, grain }
}

pub fn shift_datetime_by_grain(dt: NaiveDateTime, amount: i32, grain: Grain) -> NaiveDateTime {
    match grain {
        Grain::Second => dt + Duration::seconds(amount as i64),
        Grain::Minute => dt + Duration::minutes(amount as i64),
        Grain::Hour => dt + Duration::hours(amount as i64),
        Grain::Day => dt + Duration::days(amount as i64),
        Grain::Week => dt + Duration::weeks(amount as i64),
        Grain::Month => add_months(dt, amount),
        Grain::Quarter => add_months(dt, amount * 3),
        Grain::Year => add_months(dt, amount * 12),
    }
}

fn add_months(dt: NaiveDateTime, months: i32) -> NaiveDateTime {
    let base_year = dt.date().year();
    let base_month = dt.date().month() as i32;
    let zero_based = base_month - 1 + months;
    let year = base_year + zero_based.div_euclid(12);
    let month_zero = zero_based.rem_euclid(12);
    let month = (month_zero + 1) as u32;
    let day = dt.date().day().min(days_in_month(year, month));
    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| dt.date());
    NaiveDateTime::new(date, dt.time())
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    let first_next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(year, month, 1).unwrap());
    let last_day = first_next - Duration::days(1);
    last_day.day()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::time_expr::Grain;

    #[test]
    fn shift_datetime_by_month_clamps_day() {
        let dt = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(8, 0, 0).unwrap();
        let shifted = shift_datetime_by_grain(dt, 1, Grain::Month);
        let expected = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap().and_hms_opt(8, 0, 0).unwrap();
        assert_eq!(shifted, expected);
    }

    #[test]
    fn shift_datetime_by_quarter_advances_three_months() {
        let dt = NaiveDate::from_ymd_opt(2023, 11, 15).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let shifted = shift_datetime_by_grain(dt, 1, Grain::Quarter);
        let expected = NaiveDate::from_ymd_opt(2024, 2, 15).unwrap().and_hms_opt(0, 0, 0).unwrap();
        assert_eq!(shifted, expected);
    }

    #[test]
    fn shift_by_grain_wraps_expression() {
        let expr = shift_by_grain(TimeExpr::Reference, -2, Grain::Week);
        let TimeExpr::Shift { amount, grain, .. } = expr else {
            panic!("expected shift expression");
        };
        assert_eq!(amount, -2);
        assert_eq!(grain, Grain::Week);
    }
}
