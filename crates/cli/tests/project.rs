//! Integration tests for project handler.

mod common;

use cli::{args::ProjectAction, handler::project};

#[tokio::test]
async fn add_creates_project() {
    let (mut ctx, _) = common::context().await;
    let action = ProjectAction::Add {
        slug: "work".to_owned(),
        title: Some("Work".to_owned()),
    };

    project::handle(Some(action), &mut ctx).await.unwrap();

    let p = ctx.project_service.find_by("work").await.unwrap();
    assert_eq!(p.slug, "work");
    assert_eq!(p.title.as_deref(), Some("Work"));
}

#[tokio::test]
async fn add_without_title() {
    let (mut ctx, _) = common::context().await;
    let action = ProjectAction::Add {
        slug: "side".to_owned(),
        title: None,
    };

    project::handle(Some(action), &mut ctx).await.unwrap();

    let p = ctx.project_service.find_by("side").await.unwrap();
    assert_eq!(p.slug, "side");
    assert!(p.title.is_none());
}

#[tokio::test]
async fn list_empty() {
    let (mut ctx, _) = common::context().await;

    project::handle(Some(ProjectAction::List), &mut ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_after_add() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service.create("a", None).await.unwrap();
    ctx.project_service.create("b", Some("B")).await.unwrap();

    project::handle(Some(ProjectAction::List), &mut ctx)
        .await
        .unwrap();
}

#[tokio::test]
async fn switch_sets_active() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service.create("work", None).await.unwrap();

    let action = ProjectAction::Switch {
        slug: "work".to_owned(),
    };
    project::handle(Some(action), &mut ctx).await.unwrap();

    assert_eq!(ctx.config.active_project(), Some("work"));
}

#[tokio::test]
async fn switch_nonexistent_fails() {
    let (mut ctx, _) = common::context().await;

    let action = ProjectAction::Switch {
        slug: "nope".to_owned(),
    };
    let result = project::handle(Some(action), &mut ctx).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rename_changes_title() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service
        .create("work", Some("Old"))
        .await
        .unwrap();

    let action = ProjectAction::Rename {
        slug: "work".to_owned(),
        new_title: "New Title".to_owned(),
    };
    project::handle(Some(action), &mut ctx).await.unwrap();

    let p = ctx.project_service.find_by("work").await.unwrap();
    assert_eq!(p.title.as_deref(), Some("New Title"));
}

#[tokio::test]
async fn reslug_changes_slug() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service.create("old", None).await.unwrap();

    let action = ProjectAction::Reslug {
        slug: "old".to_owned(),
        new_slug: "new".to_owned(),
    };
    project::handle(Some(action), &mut ctx).await.unwrap();

    let p = ctx.project_service.find_by("new").await.unwrap();
    assert_eq!(p.slug, "new");
}

#[tokio::test]
async fn reslug_updates_active_project() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service.create("work", None).await.unwrap();
    ctx.config.active_project = Some("work".to_owned());

    let action = ProjectAction::Reslug {
        slug: "work".to_owned(),
        new_slug: "job".to_owned(),
    };
    project::handle(Some(action), &mut ctx).await.unwrap();

    assert_eq!(ctx.config.active_project(), Some("job"));
}

#[tokio::test]
async fn remove_deletes_project() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service.create("tmp", None).await.unwrap();

    let action = ProjectAction::Remove {
        slug: "tmp".to_owned(),
    };
    project::handle(Some(action), &mut ctx).await.unwrap();

    let result = ctx.project_service.find_by("tmp").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn remove_clears_active_if_deleted() {
    let (mut ctx, _) = common::context().await;
    ctx.project_service.create("work", None).await.unwrap();
    ctx.config.active_project = Some("work".to_owned());

    let action = ProjectAction::Remove {
        slug: "work".to_owned(),
    };
    project::handle(Some(action), &mut ctx).await.unwrap();

    assert!(ctx.config.active_project().is_none());
}

#[tokio::test]
async fn show_active_no_project() {
    let (mut ctx, _) = common::context().await;

    project::handle(None, &mut ctx).await.unwrap();
}
