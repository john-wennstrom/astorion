use chrono::{NaiveDateTime, NaiveTime, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonthPart {
    Early,
    Mid,
    Late,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Holiday {
    NewYearsDay,
    MLKDay,
    PresidentsDay,
    StPatricksDay,
    EarthDay,
    MemorialDay,
    FathersDay,
    MothersDay,
    IndependenceDay,
    LaborDay,
    ColumbusDay,
    Halloween,
    VeteransDay,
    Thanksgiving,
    Christmas,
    ChristmasEve,
    NewYearsEve,
    BossDay,
    BlackFriday,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeValue {
    Instant(NaiveDateTime),
    Interval { start: NaiveDateTime, end: NaiveDateTime },
    OpenAfter(NaiveDateTime),  // From this time onwards (formatted with +)
    OpenBefore(NaiveDateTime), // Up until this time (formatted with -)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Constraint {
    DayOfMonth(u32),
    DayOfWeek(Weekday),
    Month(u32),
    Day(u32),
    TimeOfDay(NaiveTime),
    PartOfDay(PartOfDay),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartOfDay {
    EarlyMorning,
    Morning,
    Afternoon,
    AfterLunch,
    Lunch,
    Evening,
    Night,
    Tonight,
    LateTonight,
    AfterWork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TimeExpr {
    Reference,
    At(NaiveDateTime),
    Interval {
        start: NaiveDateTime,
        end: NaiveDateTime,
    },
    Shift {
        expr: Box<TimeExpr>,
        amount: i32,
        grain: Grain,
    },
    StartOf {
        expr: Box<TimeExpr>,
        grain: Grain,
    },
    IntervalOf {
        expr: Box<TimeExpr>,
        grain: Grain,
    },
    Intersect {
        expr: Box<TimeExpr>,
        constraint: Constraint,
    },
    MonthPart {
        month: Option<u32>, // None means current month
        part: MonthPart,
    },
    IntervalUntil {
        target: Box<TimeExpr>,
    },
    /// Interval between two time expressions
    IntervalBetween {
        start: Box<TimeExpr>,
        end: Box<TimeExpr>,
    },
    /// Open-ended interval from expr onwards (formatted with +)
    OpenAfter {
        expr: Box<TimeExpr>,
    },
    /// Open-ended interval up until expr (formatted with -)
    OpenBefore {
        expr: Box<TimeExpr>,
    },
    /// Month and day without year (picks next occurrence)
    MonthDay {
        month: u32,
        day: u32,
    },
    /// Nth closest `weekday` to the (instant) resolved by `target`.
    ///
    /// `n = 1` means the closest; `n = 2` means the second closest; etc.
    ClosestWeekdayTo {
        n: u32,
        weekday: chrono::Weekday,
        target: Box<TimeExpr>,
    },
    /// Explicit absolute datetime
    Absolute {
        year: i32,
        month: u32,
        day: u32,
        hour: Option<u32>,
        minute: Option<u32>,
    },
    /// Last occurrence of a weekday in a month
    LastWeekdayOfMonth {
        year: Option<i32>, // None means current year from reference
        month: u32,
        weekday: chrono::Weekday,
    },
    /// First occurrence of a weekday in a month
    FirstWeekdayOfMonth {
        year: Option<i32>, // None means current year from reference
        month: u32,
        weekday: chrono::Weekday,
    },
    /// Nth occurrence of a weekday in a month (e.g., 4th Thursday)
    NthWeekdayOfMonth {
        n: u32,            // 1-based: 1 = first, 2 = second, etc.
        year: Option<i32>, // None means current year from reference
        month: u32,
        weekday: chrono::Weekday,
    },
    /// Nth week of a month/year
    NthWeekOf {
        n: u32, // 1-based: 1 = first, 2 = second, etc.
        year: Option<i32>,
        month: Option<u32>, // None means year-based
    },
    /// Nth-to-last week/day of a month/year (counting backwards)
    NthLastOf {
        n: u32,       // 1 = last, 2 = second-last, etc.
        grain: Grain, // Week or Day
        year: Option<i32>,
        month: Option<u32>, // None means year-based
    },
    /// The season period relative to the reference date ("this season", "next season", "last season").
    ///
    /// `offset = 0` => season containing the reference date.
    /// `offset = -1` => previous season.
    /// `offset = 1` => next season.
    SeasonPeriod {
        offset: i32,
    },
    /// Season expression (spring, summer, fall, winter)
    Season(Season),
    /// Holiday (Thanksgiving, Christmas, etc.)
    Holiday {
        holiday: Holiday,
        year: Option<i32>, // None means find nearest occurrence from reference
    },
    /// Part of day (morning, afternoon, evening, night)
    PartOfDay(PartOfDay),
    /// Open-ended "after <time>"
    After(Box<TimeExpr>),
    /// Open-ended "before <time>"
    Before(Box<TimeExpr>),
    /// Duration (for use in intervals)
    Duration(Box<TimeExpr>),
    /// Ambiguous time that should be interpreted based on reference time
    /// If reference is during day hours (6 AM - 6 PM), hour is interpreted as PM
    /// Otherwise, hour is interpreted as AM
    AmbiguousTime {
        hour: u32,   // 1-12
        minute: u32, // 0-59
    },
}
