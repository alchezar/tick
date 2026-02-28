//! Shared test helpers: fake repositories and convenience constructors.

#![allow(unused)]

pub mod fake;

use chrono::{NaiveDate, Utc};
use fake::FakeRepo;

use domain::service::{ProjectService, TaskService};

pub fn task_service() -> TaskService<FakeRepo> {
    TaskService::new(FakeRepo::default())
}

pub fn project_service() -> ProjectService<FakeRepo> {
    ProjectService::new(FakeRepo::default())
}

pub fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

pub fn datetime(date: NaiveDate, hour: u32) -> chrono::DateTime<Utc> {
    date.and_hms_opt(hour, 0, 0).unwrap().and_utc()
}
