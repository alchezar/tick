//! Integration tests for report handler.

mod common;

use chrono::{Local, NaiveDate};

use cli::handler::report;

fn today() -> NaiveDate {
    Local::now().date_naive()
}

#[tokio::test]
async fn report_no_projects() {
    let (ctx, _dir) = common::context().await;

    report::handle(None, false, Some(today()), &ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn report_empty_project() {
    let (ctx, _dir) = common::setup().await;

    report::handle(Some("work"), false, Some(today()), &ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn report_with_tasks() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    ctx.task_service
        .create("Task A", None, project.id, None)
        .await
        .unwrap();
    let t = ctx
        .task_service
        .create("Task B", None, project.id, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id).await.unwrap();

    report::handle(Some("work"), false, Some(today()), &ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn report_all_projects() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    ctx.task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();

    report::handle(None, false, Some(today()), &ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn report_nonexistent_project_fails() {
    let (ctx, _dir) = common::context().await;

    let result = report::handle(Some("nope"), false, Some(today()), &ctx).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn report_specific_date() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    ctx.task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();

    let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
    report::handle(Some("work"), false, Some(date), &ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn report_defaults_to_today() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    ctx.task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();

    report::handle(Some("work"), false, None, &ctx)
        .await
        .unwrap();
}
