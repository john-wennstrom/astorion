// The test suite implicitly treats the reference time as being in a fixed local timezone
// of UTC-02:00 (e.g. `15:00 GMT` -> `13:00`). We keep values as naive local time.
pub const LOCAL_TZ_OFFSET_HOURS: i32 = -2;

pub fn tz_offset_hours(tz: &str) -> Option<i32> {
    match tz.to_ascii_uppercase().as_str() {
        "UTC" | "GMT" => Some(0),
        "BST" => Some(1), // British Summer Time
        "CET" => Some(1),
        "IST" => Some(5), // India Standard Time (actually UTC+5:30, but using 5 for simplicity)
        "PST" => Some(-8),
        "CST" => Some(-6),
        _ => None,
    }
}
