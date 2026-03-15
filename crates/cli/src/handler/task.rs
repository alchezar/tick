//! Handler for task management commands.

use std::collections::HashSet;

use chrono::NaiveDate;
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
        TaskAction::Add {
            title, under, date, ..
        } => add(context, project_id, &title, under, date).await,
        TaskAction::List { all, .. } => list(context, project_id, all).await,
        TaskAction::Start { id, date } => {
            change_status(context, project_id, id, Status::InProgress, date).await
        }
        TaskAction::Done { id, date } => {
            change_status(context, project_id, id, Status::Done, date).await
        }
        TaskAction::Block { id, date } => {
            change_status(context, project_id, id, Status::Blocked, date).await
        }
        TaskAction::Reset { id, date } => {
            change_status(context, project_id, id, Status::NotStarted, date).await
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
    date: Option<NaiveDate>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let parent = match under {
        Some(id) => Some(
            context
                .task_service
                .find_by_prefix(&project_id, id.as_str())
                .await?,
        ),
        None => None,
    };

    let created_at = date
        .map(|d| {
            d.and_hms_opt(8, 0, 0)
                .ok_or_else(|| CliError::InvalidDate {
                    date: d.to_string(),
                })
                .map(|dt| dt.and_utc())
        })
        .transpose()?;

    let task = context
        .task_service
        .create(title, parent.as_ref(), project_id, created_at)
        .await?;

    let short_id = ShortId::from(task.id);
    println!("[{short_id}] created: {title}");
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
    let short_id = ShortId::from(task.id);
    let indent = " -".repeat(depth);
    let icon = task.status().icon();
    let task_title = &task.title;
    println!("[{short_id}] {indent} {icon} {task_title}");

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
    date: Option<NaiveDate>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let task_id = context
        .task_service
        .find_by_prefix(&project_id, id.as_str())
        .await?;

    let at = date
        .map(|d| {
            d.and_hms_opt(8, 0, 0)
                .ok_or_else(|| CliError::InvalidDate {
                    date: d.to_string(),
                })
                .map(|dt| dt.and_utc())
        })
        .transpose()?;

    match status {
        Status::InProgress => context.task_service.start(&task_id, at).await?,
        Status::Done => context.task_service.done(&task_id, at).await?,
        Status::Blocked => context.task_service.block(&task_id, at).await?,
        Status::NotStarted => context.task_service.reset(&task_id, at).await?,
    }

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] {status}");
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
    let task_id = context
        .task_service
        .find_by_prefix(&project_id, id.as_str())
        .await?;

    if let Some(parent_short) = under {
        let parent_uuid = context
            .task_service
            .find_by_prefix(&project_id, parent_short.as_str())
            .await?;
        context
            .task_service
            .move_to_parent(&task_id, Some(&parent_uuid))
            .await?;
    }

    if let Some(ord) = order {
        context.task_service.reorder(&task_id, ord).await?;
    }

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] moved");
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
    let task_id = context
        .task_service
        .find_by_prefix(&project_id, id.as_str())
        .await?;
    context.task_service.rename(&task_id, title).await?;

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] renamed: {title}");
    Ok(())
}

/// Deletes a task.
async fn remove<R>(context: &AppContext<R>, project_id: Uuid, id: ShortId) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let task_id = context
        .task_service
        .find_by_prefix(&project_id, id.as_str())
        .await?;
    context.task_service.delete(&task_id).await?;

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] removed");
    Ok(())
}
