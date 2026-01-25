/// Normalize a year value to a 4-digit year.
///
/// For 2-digit years:
/// - 50-99 are interpreted as 1950-1999
/// - 0-49 are interpreted as 2000-2049
///
/// For values >= 100, returns the value as-is.
pub fn year_from(val: i64) -> i32 {
    if val < 100 { if val >= 50 { 1900 + val as i32 } else { 2000 + val as i32 } } else { val as i32 }
}
