//! Integration tests for task handler.

mod common;

use chrono::NaiveDate;

use cli::{args::TaskAction, handler::task, types::ShortId};
use domain::model::Status;

#[tokio::test]
async fn add_creates_task() {
    let (ctx, _dir) = common::setup().await;

    let action = TaskAction::Add {
        title: "Buy milk".to_owned(),
        under: None,
        project: None,
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Buy milk");
}

#[tokio::test]
async fn add_subtask() {
    let (ctx, _dir) = common::setup().await;

    let project = ctx.project_service.find_by("work").await.unwrap();
    let parent = ctx
        .task_service
        .create("Parent", None, project.id, None)
        .await
        .unwrap();

    let parent_short: ShortId = parent.id.into();
    let action = TaskAction::Add {
        title: "Child".to_owned(),
        under: Some(parent_short),
        project: None,
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    let child = tasks.iter().find(|t| t.title == "Child").unwrap();
    assert_eq!(child.parent, Some(parent.id));
}

#[tokio::test]
async fn list_empty() {
    let (ctx, _dir) = common::setup().await;

    let action = TaskAction::List {
        all: false,
        project: None,
    };
    task::handle(action, &ctx).await.unwrap();
}

#[tokio::test]
async fn start_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();

    let action = TaskAction::Start {
        id: t.id.into(),
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].status(), Status::InProgress);
}

#[tokio::test]
async fn done_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id, None).await.unwrap();

    let action = TaskAction::Done {
        id: t.id.into(),
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].status(), Status::Done);
}

#[tokio::test]
async fn block_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id, None).await.unwrap();

    let action = TaskAction::Block {
        id: t.id.into(),
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].status(), Status::Blocked);
}

#[tokio::test]
async fn reset_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id, None).await.unwrap();

    let action = TaskAction::Reset {
        id: t.id.into(),
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].status(), Status::NotStarted);
}

#[tokio::test]
async fn rename_changes_title() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Old", None, project.id, None)
        .await
        .unwrap();

    let action = TaskAction::Rename {
        id: t.id.into(),
        title: "New".to_owned(),
    };
    task::handle(action, &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].title, "New");
}

#[tokio::test]
async fn remove_deletes_task() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Temp", None, project.id, None)
        .await
        .unwrap();

    let action = TaskAction::Remove { id: t.id.into() };
    task::handle(action, &ctx).await.unwrap();

    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn move_reorder() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None)
        .await
        .unwrap();

    let action = TaskAction::Move {
        id: t.id.into(),
        under: None,
        order: Some(5),
    };
    task::handle(action, &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].order, Some(5));
}

#[tokio::test]
async fn no_active_project_fails() {
    let (ctx, _dir) = common::context().await;
    ctx.project_service.create("work", None).await.unwrap();
    // no active project set

    let action = TaskAction::Add {
        title: "Task".to_owned(),
        under: None,
        project: None,
        date: None,
    };
    let result = task::handle(action, &ctx).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn explicit_project_flag() {
    let (ctx, _dir) = common::context().await;
    ctx.project_service.create("other", None).await.unwrap();
    // no active project, but --project is specified

    let action = TaskAction::Add {
        title: "Task".to_owned(),
        under: None,
        project: Some("other".to_owned()),
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let project = ctx.project_service.find_by("other").await.unwrap();
    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(tasks.len(), 1);
}

#[tokio::test]
async fn add_with_date_sets_created_at() {
    let (ctx, _dir) = common::setup().await;
    let date = NaiveDate::from_ymd_opt(2025, 10, 31).unwrap();

    let action = TaskAction::Add {
        title: "Backdated task".to_owned(),
        under: None,
        project: None,
        date: Some(date),
    };
    task::handle(action, &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Backdated task");
    assert_eq!(tasks[0].created.date_naive(), date);
}

#[tokio::test]
async fn add_without_date_uses_today() {
    let (ctx, _dir) = common::setup().await;

    let action = TaskAction::Add {
        title: "Today task".to_owned(),
        under: None,
        project: None,
        date: None,
    };
    task::handle(action, &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].created.date_naive(),
        chrono::Utc::now().date_naive()
    );
}
