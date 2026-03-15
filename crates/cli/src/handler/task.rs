//! Handler for task management commands.

use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    args::TaskAction,
    context::AppContext,
    error::{CliError, CliResult},
    types::ShortId,
};
use domain::{
    model::{Status, Task},
    repository::{ProjectRepository, TaskRepository, Transactional},
};

/// Dispatches a task subcommand.
///
/// # Errors
/// Returns [`CliError`] on domain, config, or resolve errors.
pub async fn handle<R>(action: TaskAction, context: &AppContext<R>) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let project_id = action
        .project()
        .or(context.config.active_project())
        .ok_or(CliError::NoActiveProject)
        .map(|slug| context.project_service.find_by(slug))?
        .await?
        .id;

    match action {
        TaskAction::Add { title, under, .. } => add(context, project_id, &title, under).await,
        TaskAction::List { all, .. } => list(context, project_id, all).await,
        TaskAction::Start { id } => {
            change_status(context, project_id, id, Status::InProgress).await
        }
        TaskAction::Done { id } => change_status(context, project_id, id, Status::Done).await,
        TaskAction::Block { id } => change_status(context, project_id, id, Status::Blocked).await,
        TaskAction::Reset { id } => {
            change_status(context, project_id, id, Status::NotStarted).await
        }
        TaskAction::Move { id, under, order } => {
            move_task(context, project_id, id, under, order).await
        }
        TaskAction::Rename { id, title } => rename(context, project_id, id, &title).await,
        TaskAction::Remove { id } => remove(context, project_id, id).await,
    }
}

/// Adds a new task.
async fn add<R>(
    context: &AppContext<R>,
    project_id: Uuid,
    title: &str,
    under: Option<ShortId>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let parent = match under {
        Some(id) => Some(id.to_uuid(&context.task_service, &project_id).await?),
        None => None,
    };

    let task = context
        .task_service
        .create(title, parent.as_ref(), project_id)
        .await?;
    println!("[{}] created: {title}", ShortId::from(task.id));
    Ok(())
}

/// Lists tasks as a tree.
async fn list<R>(context: &AppContext<R>, project_id: Uuid, show_all: bool) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let tasks = context.task_service.list(&project_id).await?;

    let visible = if show_all {
        tasks
    } else {
        tasks
            .into_iter()
            .filter(|t| t.status().is_active())
            .collect::<Vec<_>>()
    };

    if visible.is_empty() {
        println!("no tasks");
        return Ok(());
    }

    let ids = visible.iter().map(|t| t.id).collect::<HashSet<_>>();
    let mut roots = visible
        .iter()
        .filter(|t| t.parent.is_none_or(|p| !ids.contains(&p)))
        .collect::<Vec<_>>();
    roots.sort_by_key(|t| t.order);

    for root in roots {
        print_task(root, &visible, 0);
    }
    Ok(())
}

/// Prints a task and its children recursively.
fn print_task(task: &Task, all: &[Task], depth: usize) {
    let indent = "  ".repeat(depth);
    let icon = task.status().icon();
    println!("{indent}{icon} [{}] {}", ShortId::from(task.id), task.title);

    let mut children = all
        .iter()
        .filter(|t| t.parent == Some(task.id))
        .collect::<Vec<_>>();
    children.sort_by_key(|t| t.order);

    for child in children {
        print_task(child, all, depth + 1);
    }
}

/// Changes task status.
async fn change_status<R>(
    context: &AppContext<R>,
    project_id: Uuid,
    id: ShortId,
    status: Status,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let uuid = id.to_uuid(&context.task_service, &project_id).await?;

    match status {
        Status::InProgress => context.task_service.start(&uuid).await?,
        Status::Done => context.task_service.done(&uuid).await?,
        Status::Blocked => context.task_service.block(&uuid).await?,
        Status::NotStarted => context.task_service.reset(&uuid).await?,
    }

    println!("[{}] {status}", ShortId::from(uuid));
    Ok(())
}

/// Moves a task under a new parent or changes its order.
async fn move_task<R>(
    context: &AppContext<R>,
    project_id: Uuid,
    id: ShortId,
    under: Option<ShortId>,
    order: Option<usize>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let uuid = id.to_uuid(&context.task_service, &project_id).await?;

    if let Some(parent_short) = under {
        let parent_uuid = parent_short
            .to_uuid(&context.task_service, &project_id)
            .await?;
        context
            .task_service
            .move_to_parent(&uuid, Some(&parent_uuid))
            .await?;
    }

    if let Some(ord) = order {
        context.task_service.reorder(&uuid, ord).await?;
    }

    println!("[{}] moved", ShortId::from(uuid));
    Ok(())
}

/// Renames a task.
async fn rename<R>(
    context: &AppContext<R>,
    project_id: Uuid,
    id: ShortId,
    title: &str,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let uuid = id.to_uuid(&context.task_service, &project_id).await?;
    context.task_service.rename(&uuid, title).await?;
    println!("[{}] renamed: {title}", ShortId::from(uuid));
    Ok(())
}

/// Deletes a task.
async fn remove<R>(context: &AppContext<R>, project_id: Uuid, id: ShortId) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let uuid = id.to_uuid(&context.task_service, &project_id).await?;
    context.task_service.delete(&uuid).await?;
    println!("[{}] removed", ShortId::from(uuid));
    Ok(())
}
