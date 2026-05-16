//! Shared test helpers: fake repositories and convenience constructors.

#![allow(unused)]

pub mod fake;

use chrono::{DateTime, NaiveDate, Utc};
use fake::FakeRepo;

use domain::service::{ProjectService, TaskService};

pub fn task_service() -> TaskService<FakeRepo> {
    TaskService::new(FakeRepo::default())
}

pub fn project_service() -> ProjectService<FakeRepo> {
    ProjectService::new(FakeRepo::default())
}

pub fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

pub fn datetime(date: NaiveDate, hour: u32) -> DateTime<Utc> {
    date.and_hms_opt(hour, 0, 0).unwrap().and_utc()
}
