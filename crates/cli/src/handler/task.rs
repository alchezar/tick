//! Handler for task management commands.

use std::collections::HashSet;

use chrono::{Local, NaiveDate};

use crate::{
    args::TaskAction,
    context::AppContext,
    error::{CliError, CliResult},
    guard::Confirm,
    types::ShortId,
};
use domain::{
    model::{Status, Task},
    repository::{ProjectRepository, TaskFilter, TaskRepository, Transactional},
};

/// Dispatches a task subcommand.
///
/// # Errors
/// Returns [`CliError`] on domain, config, or resolve errors.
pub async fn handle<R, C>(action: Option<TaskAction>, context: &AppContext<R, C>) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
    C: Confirm,
{
    match action.unwrap_or_default() {
        TaskAction::Add {
            title,
            parent,
            date,
            project,
        } => add(context, project, &title, parent, date).await,
        TaskAction::List {
            from,
            until,
            all,
            subtree,
            project,
        } => list(context, project, all, from, until, subtree).await,
        TaskAction::Start { id, date } => {
            change_status(context, id, Status::InProgress, date).await
        }
        TaskAction::Done { id, date } => change_status(context, id, Status::Done, date).await,
        TaskAction::Block { id, date } => change_status(context, id, Status::Blocked, date).await,
        TaskAction::Abandon { id, date } => {
            change_status(context, id, Status::Abandoned, date).await
        }
        TaskAction::Reset { id, date } => {
            change_status(context, id, Status::NotStarted, date).await
        }
        TaskAction::Move {
            id,
            parent,
            up,
            down,
            order,
        } => move_task(context, id, parent, up, down, order).await,
        TaskAction::Rename { id, title } => rename(context, id, &title).await,
        TaskAction::Remove { id } => remove(context, id).await,
    }
}

/// Adds a new task.
async fn add<R, C>(
    context: &AppContext<R, C>,
    project: Option<String>,
    title: &str,
    parent: Option<ShortId>,
    date: Option<NaiveDate>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let parent = match parent {
        Some(id) => Some(context.task_service.find_by_prefix(id.as_str()).await?),
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

    let project_id = context.resolve_project(project.as_deref()).await?.id;
    let task = context
        .task_service
        .create(title, parent, project_id, created_at)
        .await?;

    let short_id = ShortId::from(task.id);
    println!("[{short_id}] created: {title}");
    Ok(())
}

/// Lists tasks as a tree.
async fn list<R, C>(
    context: &AppContext<R, C>,
    project: Option<String>,
    show_all: bool,
    from: Option<NaiveDate>,
    until: Option<NaiveDate>,
    subtree: Option<ShortId>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let show_date = show_all || from.is_some() || until.is_some() || subtree.is_some();
    let project_id = context.resolve_project(project.as_deref()).await?.id;
    let filter = if show_date {
        TaskFilter::ByProject(project_id)
    } else {
        TaskFilter::ActiveByProject(project_id, Local::now().date_naive())
    };
    let mut visible = context.task_service.list(&filter).await?;

    if let Some(from) = from {
        visible.retain(|t| t.status().is_active() || t.updated.date_naive() >= from);
    }
    if let Some(until) = until {
        visible.retain(|t| t.status().is_active() || t.updated.date_naive() < until);
    }

    let subtree_root_id = match subtree {
        Some(short) => Some(context.task_service.find_by_prefix(short.as_str()).await?),
        None => None,
    };

    if visible.is_empty() {
        println!("no tasks");
        return Ok(());
    }

    let ids = visible.iter().map(|t| t.id).collect::<HashSet<_>>();
    let mut roots = visible
        .iter()
        .filter(|t| match subtree_root_id {
            Some(id) => t.id == id,
            None => t.parent.is_none_or(|p| !ids.contains(&p)),
        })
        .collect::<Vec<_>>();
    roots.sort_by_key(|t| t.order);

    for root in roots {
        print_task(root, &visible, 1, show_date);
    }
    Ok(())
}

/// Prints a task and its children recursively.
fn print_task(task: &Task, all: &[Task], depth: usize, show_date: bool) {
    let short_id = ShortId::from(task.id);
    let updated = if show_date {
        format!("|{}", task.updated.format("%Y-%m-%d"))
    } else {
        String::new()
    };
    let indent = " -".repeat(depth);
    let icon = super::terminal_emoji(task.status().icon());
    let task_title = &task.title;
    println!("[{short_id}{updated}]{indent} {icon} {task_title}");

    let mut children = all
        .iter()
        .filter(|t| t.parent == Some(task.id))
        .collect::<Vec<_>>();
    children.sort_by_key(|t| t.order);

    for child in children {
        print_task(child, all, depth + 1, show_date);
    }
}

/// Changes task status.
async fn change_status<R, C>(
    context: &AppContext<R, C>,
    id: ShortId,
    status: Status,
    date: Option<NaiveDate>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let task_id = context.task_service.find_by_prefix(id.as_str()).await?;

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
        Status::Abandoned => context.task_service.abandon(&task_id, at).await?,
    }

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] {status}");
    Ok(())
}

/// Moves a task to a new parent or changes its display order.
async fn move_task<R, C>(
    context: &AppContext<R, C>,
    id: ShortId,
    parent: Option<ShortId>,
    up: bool,
    down: bool,
    order: Option<usize>,
) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let task_id = context.task_service.find_by_prefix(id.as_str()).await?;
    let project_id = context.resolve_project(None).await?.id;

    if let Some(parent_short) = parent {
        let parent_uuid = context
            .task_service
            .find_by_prefix(parent_short.as_str())
            .await?;
        context
            .task_service
            .move_to_parent(&task_id, Some(parent_uuid), project_id)
            .await?;
    }

    if up || down {
        let task = context.task_service.find_task(&task_id).await?;
        let current = task.order.unwrap_or(0);

        let mut siblings = context
            .task_service
            .list(&task.siblings_filter(project_id))
            .await?
            .into_iter()
            .filter(|t| t.id != task_id)
            .collect::<Vec<_>>();
        siblings.sort_by_key(|t| t.order.unwrap_or(0));

        let neighbor = if up {
            siblings.iter().rfind(|t| t.order.unwrap_or(0) < current)
        } else {
            siblings.iter().find(|t| t.order.unwrap_or(0) > current)
        };

        if let Some(neighbor) = neighbor {
            let neighbor_order = neighbor.order.unwrap_or(0);
            context
                .task_service
                .swap_order(&task_id, current, &neighbor.id, neighbor_order)
                .await?;
        }
    } else if let Some(ord) = order {
        let mut tasks = context
            .task_service
            .list(&TaskFilter::ByProject(project_id))
            .await?;
        context
            .task_service
            .reorder(&task_id, ord, &mut tasks)
            .await?;
    }

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] moved");
    Ok(())
}

/// Renames a task.
async fn rename<R, C>(context: &AppContext<R, C>, id: ShortId, title: &str) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let task_id = context.task_service.find_by_prefix(id.as_str()).await?;
    context.task_service.rename(&task_id, title).await?;

    let short_id = ShortId::from(task_id);
    println!("[{short_id}] renamed: {title}");
    Ok(())
}

/// Deletes a task.
async fn remove<R, C>(context: &AppContext<R, C>, id: ShortId) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
    C: Confirm,
{
    let task_id = context.task_service.find_by_prefix(id.as_str()).await?;

    let short_id = ShortId::from(task_id);
    context
        .confirmer
        .borrow_mut()
        .confirm(&format!(r#"task "{short_id}""#))?;

    context.task_service.delete(&task_id).await?;

    println!(r#"Task "{short_id}" removed"#);
    Ok(())
}
