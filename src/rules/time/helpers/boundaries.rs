use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::rules::time::helpers::shift::shift_datetime_by_grain;
use crate::time_expr::{Grain, TimeValue};

pub fn start_of(grain: Grain, dt: NaiveDateTime) -> NaiveDateTime {
    match grain {
        Grain::Second => {
            let time = dt.time().with_nanosecond(0).unwrap_or_else(|| dt.time());
            NaiveDateTime::new(dt.date(), time)
        }
        Grain::Minute => {
            let time = NaiveTime::from_hms_opt(dt.hour(), dt.minute(), 0).unwrap_or_else(|| dt.time());
            NaiveDateTime::new(dt.date(), time)
        }
        Grain::Hour => {
            let time = NaiveTime::from_hms_opt(dt.hour(), 0, 0).unwrap_or_else(|| dt.time());
            NaiveDateTime::new(dt.date(), time)
        }
        Grain::Day => NaiveDateTime::new(dt.date(), NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_else(|| dt.time())),
        Grain::Week => {
            let weekday_offset = dt.date().weekday().num_days_from_monday() as i64;
            let start_date = dt.date() - Duration::days(weekday_offset);
            NaiveDateTime::new(start_date, NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_else(|| dt.time()))
        }
        Grain::Month => NaiveDateTime::new(
            NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1).unwrap_or_else(|| dt.date()),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_else(|| dt.time()),
        ),
        Grain::Quarter => {
            let quarter_start = ((dt.month() - 1) / 3) * 3 + 1;
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(dt.year(), quarter_start, 1).unwrap_or_else(|| dt.date()),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_else(|| dt.time()),
            )
        }
        Grain::Year => NaiveDateTime::new(
            NaiveDate::from_ymd_opt(dt.year(), 1, 1).unwrap_or_else(|| dt.date()),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_else(|| dt.time()),
        ),
    }
}

pub fn interval_of(grain: Grain, dt: NaiveDateTime) -> TimeValue {
    let start = start_of(grain, dt);
    let end = shift_datetime_by_grain(start, 1, grain);
    TimeValue::Interval { start, end }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn start_of_week_aligns_to_monday() {
        let dt = NaiveDate::from_ymd_opt(2024, 4, 10).unwrap().and_hms_opt(15, 45, 12).unwrap();
        let start = start_of(Grain::Week, dt);
        let expected = NaiveDate::from_ymd_opt(2024, 4, 8).unwrap().and_hms_opt(0, 0, 0).unwrap();
        assert_eq!(start, expected);
    }

    #[test]
    fn start_of_quarter_returns_first_month() {
        let dt = NaiveDate::from_ymd_opt(2024, 5, 22).unwrap().and_hms_opt(9, 30, 0).unwrap();
        let start = start_of(Grain::Quarter, dt);
        let expected = NaiveDate::from_ymd_opt(2024, 4, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        assert_eq!(start, expected);
    }

    #[test]
    fn interval_of_day_is_one_day_long() {
        let dt = NaiveDate::from_ymd_opt(2024, 8, 31).unwrap().and_hms_opt(12, 0, 0).unwrap();
        let TimeValue::Interval { start, end } = interval_of(Grain::Day, dt) else {
            panic!("expected day interval");
        };
        let expected_start = NaiveDate::from_ymd_opt(2024, 8, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let expected_end = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        assert_eq!(start, expected_start);
        assert_eq!(end, expected_end);
    }
}
