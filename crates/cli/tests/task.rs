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
        parent: None,
        project: None,
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

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
        parent: Some(parent_short),
        project: None,
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

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
    task::handle(Some(action), &ctx).await.unwrap();
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
    task::handle(Some(action), &ctx).await.unwrap();

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
    task::handle(Some(action), &ctx).await.unwrap();

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
    task::handle(Some(action), &ctx).await.unwrap();

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
    task::handle(Some(action), &ctx).await.unwrap();

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
    task::handle(Some(action), &ctx).await.unwrap();

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
    task::handle(Some(action), &ctx).await.unwrap();

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
        parent: None,
        up: false,
        down: false,
        order: Some(5),
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(updated[0].order, Some(5));
}

#[tokio::test]
async fn move_up() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let a = ctx
        .task_service
        .create("A", None, project.id, None)
        .await
        .unwrap();
    let b = ctx
        .task_service
        .create("B", None, project.id, None)
        .await
        .unwrap();

    // B starts at order 1, move up -> order 0
    let action = TaskAction::Move {
        id: b.id.into(),
        parent: None,
        up: true,
        down: false,
        order: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    let updated_a = tasks.iter().find(|t| t.id == a.id).unwrap();
    let updated_b = tasks.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(updated_b.order, Some(0));
    assert_eq!(updated_a.order, Some(1));
}

#[tokio::test]
async fn move_down() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let a = ctx
        .task_service
        .create("A", None, project.id, None)
        .await
        .unwrap();
    let b = ctx
        .task_service
        .create("B", None, project.id, None)
        .await
        .unwrap();

    // A starts at order 0, move down -> order 1
    let action = TaskAction::Move {
        id: a.id.into(),
        parent: None,
        up: false,
        down: true,
        order: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    let updated_a = tasks.iter().find(|t| t.id == a.id).unwrap();
    let updated_b = tasks.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(updated_a.order, Some(1));
    assert_eq!(updated_b.order, Some(2));
}

#[tokio::test]
async fn move_up_at_zero_stays() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let a = ctx
        .task_service
        .create("A", None, project.id, None)
        .await
        .unwrap();

    // A is at order 0, move up -> stays at 0
    let action = TaskAction::Move {
        id: a.id.into(),
        parent: None,
        up: true,
        down: false,
        order: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(tasks[0].order, Some(0));
}

#[tokio::test]
async fn move_down_at_last_stays() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let _a = ctx
        .task_service
        .create("A", None, project.id, None)
        .await
        .unwrap();
    let b = ctx
        .task_service
        .create("B", None, project.id, None)
        .await
        .unwrap();

    // B is at order 1 (last), move down -> stays at 1
    let action = TaskAction::Move {
        id: b.id.into(),
        parent: None,
        up: false,
        down: true,
        order: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    let updated_b = tasks.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(updated_b.order, Some(1));
}

#[tokio::test]
async fn no_active_project_fails() {
    let (ctx, _dir) = common::context().await;
    ctx.project_service.create("work", None).await.unwrap();
    // no active project set

    let action = TaskAction::Add {
        title: "Task".to_owned(),
        parent: None,
        project: None,
        date: None,
    };
    let result = task::handle(Some(action), &ctx).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn explicit_project_flag() {
    let (ctx, _dir) = common::context().await;
    ctx.project_service.create("other", None).await.unwrap();
    // no active project, but --project is specified

    let action = TaskAction::Add {
        title: "Task".to_owned(),
        parent: None,
        project: Some("other".to_owned()),
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

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
        parent: None,
        project: None,
        date: Some(date),
    };
    task::handle(Some(action), &ctx).await.unwrap();

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
        parent: None,
        project: None,
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx.task_service.list(&project.id).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].created.date_naive(),
        chrono::Utc::now().date_naive()
    );
}
