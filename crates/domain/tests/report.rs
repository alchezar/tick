//! Integration tests for `ReportService` helpers.

use chrono::NaiveDate;

use domain::service;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn monday_returns_friday() {
    // 2025-02-17 is Monday
    assert_eq!(service::prev_workday(date(2026, 2, 23)), date(2026, 2, 20));
}

#[test]
fn tuesday_returns_monday() {
    assert_eq!(service::prev_workday(date(2026, 2, 17)), date(2026, 2, 16));
}

#[test]
fn friday_returns_thursday() {
    assert_eq!(service::prev_workday(date(2026, 2, 20)), date(2026, 2, 19));
}
