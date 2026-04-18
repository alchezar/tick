//! Integration tests for task handler.

mod common;

use chrono::{Local, NaiveDate};

use cli::{args::TaskAction, handler::task, types::ShortId};
use domain::{model::Status, repository::TaskFilter};

#[tokio::test]
async fn add_creates_task() {
    let (ctx, _dir) = common::setup().await;

    let action = TaskAction::Add {
        title: "Buy milk".to_owned(),
        parent: None,
        project: None,
        date: None,
        number: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Buy milk");
}

#[tokio::test]
async fn add_subtask() {
    let (ctx, _dir) = common::setup().await;

    let project = ctx.project_service.find_by("work").await.unwrap();
    let parent = ctx
        .task_service
        .create("Parent", None, project.id, None, None, None)
        .await
        .unwrap();

    let parent_short = ShortId::from(parent.id);
    let action = TaskAction::Add {
        title: "Child".to_owned(),
        parent: Some(parent_short),
        project: None,
        date: None,
        number: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    let child = tasks.iter().find(|t| t.title == "Child").unwrap();
    assert_eq!(child.parent, Some(parent.id));
}

#[tokio::test]
async fn list_empty() {
    let (ctx, _dir) = common::setup().await;

    let action = TaskAction::List {
        from: None,
        until: None,
        all: false,
        subtree: None,
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
        .create("Task", None, project.id, None, None, None)
        .await
        .unwrap();

    let action = TaskAction::Start {
        id: t.id.into(),
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(updated[0].status(), Status::InProgress);
}

#[tokio::test]
async fn done_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None, None, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id, None).await.unwrap();

    let action = TaskAction::Done {
        id: t.id.into(),
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(updated[0].status(), Status::Done);
}

#[tokio::test]
async fn block_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None, None, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id, None).await.unwrap();

    let action = TaskAction::Block {
        id: t.id.into(),
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(updated[0].status(), Status::Blocked);
}

#[tokio::test]
async fn reset_changes_status() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None, None, None)
        .await
        .unwrap();
    ctx.task_service.start(&t.id, None).await.unwrap();

    let action = TaskAction::Reset {
        id: t.id.into(),
        date: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(updated[0].status(), Status::NotStarted);
}

#[tokio::test]
async fn rename_changes_title() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Old", None, project.id, None, None, None)
        .await
        .unwrap();

    let action = TaskAction::Rename {
        id: t.id.into(),
        title: "New".to_owned(),
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(updated[0].title, "New");
}

#[tokio::test]
async fn remove_deletes_task() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Temp", None, project.id, None, None, None)
        .await
        .unwrap();

    let action = TaskAction::Remove { id: t.id.into() };
    task::handle(Some(action), &ctx).await.unwrap();

    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn move_reorder() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let t = ctx
        .task_service
        .create("Task", None, project.id, None, None, None)
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

    let updated = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(updated[0].order, Some(5));
}

#[tokio::test]
async fn move_up() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let a = ctx
        .task_service
        .create("A", None, project.id, None, None, None)
        .await
        .unwrap();
    let b = ctx
        .task_service
        .create("B", None, project.id, None, None, None)
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

    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
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
        .create("A", None, project.id, None, None, None)
        .await
        .unwrap();
    let b = ctx
        .task_service
        .create("B", None, project.id, None, None, None)
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

    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    let updated_a = tasks.iter().find(|t| t.id == a.id).unwrap();
    let updated_b = tasks.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(updated_a.order, Some(1));
    assert_eq!(updated_b.order, Some(0));
}

#[tokio::test]
async fn move_up_at_zero_stays() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let a = ctx
        .task_service
        .create("A", None, project.id, None, None, None)
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

    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(tasks[0].order, Some(0));
}

#[tokio::test]
async fn move_down_at_last_stays() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let _a = ctx
        .task_service
        .create("A", None, project.id, None, None, None)
        .await
        .unwrap();
    let b = ctx
        .task_service
        .create("B", None, project.id, None, None, None)
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

    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    let updated_b = tasks.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(updated_b.order, Some(1));
}

#[tokio::test]
async fn no_active_project_fails() {
    let (ctx, _dir) = common::context().await;
    ctx.project_service
        .create("work", None, None)
        .await
        .unwrap();
    // no active project set

    let action = TaskAction::Add {
        title: "Task".to_owned(),
        parent: None,
        project: None,
        date: None,
        number: None,
    };
    let result = task::handle(Some(action), &ctx).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn explicit_project_flag() {
    let (ctx, _dir) = common::context().await;
    ctx.project_service
        .create("other", None, None)
        .await
        .unwrap();
    // no active project, but --project is specified

    let action = TaskAction::Add {
        title: "Task".to_owned(),
        parent: None,
        project: Some("other".to_owned()),
        date: None,
        number: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let project = ctx.project_service.find_by("other").await.unwrap();
    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
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
        number: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
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
        number: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0].created.date_naive(),
        chrono::Utc::now().date_naive()
    );
}

#[tokio::test]
async fn list_from_includes_closed_since_date() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();

    let past = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let recent = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();

    // Task closed in the past (before --from).
    let old = ctx
        .task_service
        .create(
            "Old done",
            None,
            project.id,
            Some(past.and_hms_opt(8, 0, 0).unwrap().and_utc()),
            None,
            None,
        )
        .await
        .unwrap();
    ctx.task_service
        .start(&old.id, Some(past.and_hms_opt(8, 30, 0).unwrap().and_utc()))
        .await
        .unwrap();
    ctx.task_service
        .done(&old.id, Some(past.and_hms_opt(9, 0, 0).unwrap().and_utc()))
        .await
        .unwrap();

    // Task closed recently (after --from).
    let new = ctx
        .task_service
        .create(
            "New done",
            None,
            project.id,
            Some(recent.and_hms_opt(8, 0, 0).unwrap().and_utc()),
            None,
            None,
        )
        .await
        .unwrap();
    ctx.task_service
        .start(
            &new.id,
            Some(recent.and_hms_opt(8, 30, 0).unwrap().and_utc()),
        )
        .await
        .unwrap();
    ctx.task_service
        .done(
            &new.id,
            Some(recent.and_hms_opt(9, 0, 0).unwrap().and_utc()),
        )
        .await
        .unwrap();

    // Active task (always visible).
    ctx.task_service
        .create("Active", None, project.id, None, None, None)
        .await
        .unwrap();

    let from_date = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let action = TaskAction::List {
        from: Some(from_date),
        until: None,
        all: false,
        subtree: None,
        project: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    // Verify via the same filter logic: ByProject then retain.
    let all = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    let visible = all
        .iter()
        .filter(|t| t.status().is_active() || t.updated.date_naive() >= from_date)
        .collect::<Vec<_>>();

    // Active + "New done" should be visible; "Old done" filtered out.
    assert_eq!(visible.len(), 2);
    assert!(visible.iter().any(|t| t.title == "Active"));
    assert!(visible.iter().any(|t| t.title == "New done"));
}

#[tokio::test]
async fn list_until_excludes_closed_on_date() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();

    let early = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let late = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();

    // Task closed early (before --until).
    let old = ctx
        .task_service
        .create(
            "Old done",
            None,
            project.id,
            Some(early.and_hms_opt(8, 0, 0).unwrap().and_utc()),
            None,
            None,
        )
        .await
        .unwrap();
    ctx.task_service
        .start(
            &old.id,
            Some(early.and_hms_opt(8, 30, 0).unwrap().and_utc()),
        )
        .await
        .unwrap();
    ctx.task_service
        .done(&old.id, Some(early.and_hms_opt(9, 0, 0).unwrap().and_utc()))
        .await
        .unwrap();

    // Task closed on the until date (excluded - until is exclusive).
    let border = ctx
        .task_service
        .create(
            "Border done",
            None,
            project.id,
            Some(late.and_hms_opt(8, 0, 0).unwrap().and_utc()),
            None,
            None,
        )
        .await
        .unwrap();
    ctx.task_service
        .start(
            &border.id,
            Some(late.and_hms_opt(8, 30, 0).unwrap().and_utc()),
        )
        .await
        .unwrap();
    ctx.task_service
        .done(
            &border.id,
            Some(late.and_hms_opt(9, 0, 0).unwrap().and_utc()),
        )
        .await
        .unwrap();

    // Active task (always visible).
    ctx.task_service
        .create("Active", None, project.id, None, None, None)
        .await
        .unwrap();

    let until_date = late;
    let action = TaskAction::List {
        from: None,
        until: Some(until_date),
        all: false,
        subtree: None,
        project: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    // Verify: until is exclusive, so "Border done" (updated on until_date) should be excluded.
    let all = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    let visible = all
        .iter()
        .filter(|t| t.status().is_active() || t.updated.date_naive() < until_date)
        .collect::<Vec<_>>();

    assert_eq!(visible.len(), 2);
    assert!(visible.iter().any(|t| t.title == "Active"));
    assert!(visible.iter().any(|t| t.title == "Old done"));
}

#[tokio::test]
async fn list_subtree_shows_only_descendants() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();

    // Create tree: Root -> Child -> Grandchild, and a Sibling at root level.
    let root = ctx
        .task_service
        .create("Root", None, project.id, None, None, None)
        .await
        .unwrap();
    let child = ctx
        .task_service
        .create("Child", Some(root.id), project.id, None, None, None)
        .await
        .unwrap();
    ctx.task_service
        .create("Grandchild", Some(child.id), project.id, None, None, None)
        .await
        .unwrap();
    ctx.task_service
        .create("Sibling", None, project.id, None, None, None)
        .await
        .unwrap();

    let root_short = ShortId::from(root.id);
    let action = TaskAction::List {
        from: None,
        until: None,
        all: false,
        subtree: Some(root_short),
        project: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    // Verify: subtree filter loads all tasks via ByProject, root is the
    // only entry point and print_task recurses into children.
    let all = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(all.len(), 4);

    // The subtree rooted at `root` contains Root, Child, Grandchild (3 tasks).
    let descendants = all
        .iter()
        .filter(|t| t.id == root.id || t.parent == Some(root.id) || t.parent == Some(child.id))
        .collect::<Vec<_>>();
    assert_eq!(descendants.len(), 3);
    assert!(descendants.iter().any(|t| t.title == "Root"));
    assert!(descendants.iter().any(|t| t.title == "Child"));
    assert!(descendants.iter().any(|t| t.title == "Grandchild"));
}

#[tokio::test]
async fn list_subtree_includes_done_tasks() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();

    let root = ctx
        .task_service
        .create("Root", None, project.id, None, None, None)
        .await
        .unwrap();
    let child = ctx
        .task_service
        .create("Done child", Some(root.id), project.id, None, None, None)
        .await
        .unwrap();
    let past = NaiveDate::from_ymd_opt(2025, 1, 10)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc();
    ctx.task_service.start(&child.id, Some(past)).await.unwrap();
    ctx.task_service.done(&child.id, Some(past)).await.unwrap();

    // Without --subtree, done tasks closed in the past are hidden.
    let all_active = ctx
        .task_service
        .list(&TaskFilter::ActiveByProject(
            project.id,
            Local::now().date_naive(),
        ))
        .await
        .unwrap();
    assert!(all_active.iter().all(|t| t.title != "Done child"));

    // With --subtree, all tasks (including done) are shown.
    let root_short = ShortId::from(root.id);
    let action = TaskAction::List {
        from: None,
        until: None,
        all: false,
        subtree: Some(root_short),
        project: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let all = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    let done_child = all.iter().find(|t| t.title == "Done child").unwrap();
    assert_eq!(done_child.status(), Status::Done);
    assert_eq!(done_child.parent, Some(root.id));
}

#[tokio::test]
async fn move_without_flags_promotes_to_root() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();

    let parent = ctx
        .task_service
        .create("Parent", None, project.id, None, None, None)
        .await
        .unwrap();
    let child = ctx
        .task_service
        .create("Child", Some(parent.id), project.id, None, None, None)
        .await
        .unwrap();
    assert!(child.parent.is_some());

    let action = TaskAction::Move {
        id: child.id.into(),
        parent: None,
        up: false,
        down: false,
        order: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx.task_service.find_task(&child.id).await.unwrap();
    assert!(updated.parent.is_none(), "task should be promoted to root");
}

#[tokio::test]
async fn add_with_pull_request() {
    let (ctx, _dir) = common::setup().await;

    let action = TaskAction::Add {
        title: "Feature".to_owned(),
        parent: None,
        project: None,
        date: None,
        number: Some(42),
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let project = ctx.project_service.find_by("work").await.unwrap();
    let tasks = ctx
        .task_service
        .list(&TaskFilter::ByProject(project.id))
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].pull_request_number, Some(42));
}

#[tokio::test]
async fn set_pull_request_via_handler() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let task = ctx
        .task_service
        .create("Task", None, project.id, None, None, None)
        .await
        .unwrap();

    let action = TaskAction::Link {
        id: task.id.into(),
        number: Some(66),
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx.task_service.find_task(&task.id).await.unwrap();
    assert_eq!(updated.pull_request_number, Some(66));
}

#[tokio::test]
async fn clear_pull_request_via_handler() {
    let (ctx, _dir) = common::setup().await;
    let project = ctx.project_service.find_by("work").await.unwrap();
    let task = ctx
        .task_service
        .create("Task", None, project.id, None, Some(99), None)
        .await
        .unwrap();

    let action = TaskAction::Link {
        id: task.id.into(),
        number: None,
    };
    task::handle(Some(action), &ctx).await.unwrap();

    let updated = ctx.task_service.find_task(&task.id).await.unwrap();
    assert!(updated.pull_request_number.is_none());
}

#[test]
fn pull_request_link_formats_blue_url() {
    let result = cli::handler::pull_request_link("https://github.com/owner/repo", 66);
    assert_eq!(
        result,
        "\x1b[90mhttps://github.com/owner/repo/pull/\x1b[34m66\x1b[0m"
    );
}
