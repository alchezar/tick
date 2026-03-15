//! Handler for project management commands.

use crate::{args::ProjectAction, context::AppContext, error::CliResult};
use domain::repository::{ProjectRepository, TaskRepository, Transactional};

/// Dispatches a project subcommand.
///
/// # Errors
/// Returns [`CliError`](crate::error::CliError) on domain or config errors.
pub async fn handle<R>(action: Option<ProjectAction>, ctx: &mut AppContext<R>) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    match action {
        None => show_active(ctx),
        Some(ProjectAction::List) => list(ctx).await,
        Some(ProjectAction::Add { slug, title }) => add(ctx, &slug, title.as_deref()).await,
        Some(ProjectAction::Switch { slug }) => switch(ctx, &slug).await,
        Some(ProjectAction::Rename { slug, new_title }) => rename(ctx, &slug, &new_title).await,
        Some(ProjectAction::Reslug { slug, new_slug }) => reslug(ctx, &slug, &new_slug).await,
        Some(ProjectAction::Remove { slug }) => remove(ctx, &slug).await,
    }
}

/// Shows the active project slug and title.
#[allow(clippy::unnecessary_wraps)]
fn show_active<R>(context: &AppContext<R>) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    match context.config.active_project() {
        Some(slug) => println!("{slug}"),
        None => println!("no active project"),
    }
    Ok(())
}

/// Lists all projects.
async fn list<R>(context: &AppContext<R>) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let projects = context.project_service.list().await?;

    if projects.is_empty() {
        println!("no projects");
        return Ok(());
    }

    for p in projects {
        match &p.title {
            Some(title) => println!("{} - {title}", p.slug),
            None => println!("{}", p.slug),
        }
    }
    Ok(())
}

/// Creates a new project.
async fn add<R>(context: &AppContext<R>, slug: &str, title: Option<&str>) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    let project = context.project_service.create(slug, title).await?;
    match &project.title {
        Some(title) => println!("created: {} - {title}", project.slug),
        None => println!("created: {}", project.slug),
    }
    Ok(())
}

/// Switches the active project.
async fn switch<R>(context: &mut AppContext<R>, slug: &str) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    context.project_service.find_by(slug).await?;
    context.config.set_active(slug)?;
    println!("switched to: {slug}");
    Ok(())
}

/// Renames a project (changes display title).
async fn rename<R>(context: &AppContext<R>, slug: &str, new_title: &str) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    context.project_service.rename(slug, new_title).await?;
    println!("renamed: {slug} -> {new_title}");
    Ok(())
}

/// Changes the slug of a project.
async fn reslug<R>(context: &mut AppContext<R>, slug: &str, new_slug: &str) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    context.project_service.reslug(slug, new_slug).await?;

    if context.config.active_project() == Some(slug) {
        context.config.set_active(new_slug)?;
    }

    println!("reslugged: {slug} -> {new_slug}");
    Ok(())
}

/// Deletes a project and all its tasks.
async fn remove<R>(context: &mut AppContext<R>, slug: &str) -> CliResult<()>
where
    R: ProjectRepository + TaskRepository + Transactional,
{
    context.project_service.delete(slug).await?;

    if context.config.active_project() == Some(slug) {
        context.config.active_project = None;
        context.config.save()?;
    }

    println!("removed: {slug}");
    Ok(())
}
