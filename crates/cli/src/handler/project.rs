//! Handler for project management commands.

use crate::{args::ProjectAction, config::Config, error::CliResult};
use domain::{
    repository::{ProjectRepository, Transactional},
    service::ProjectService,
};

/// Dispatches a project subcommand.
///
/// # Errors
/// Returns [`CliError`](crate::error::CliError) on domain or config errors.
pub async fn handle<R>(
    action: Option<ProjectAction>,
    config: &mut Config,
    service: &ProjectService<R>,
) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    match action {
        None => show_active(config),
        Some(ProjectAction::List) => list(service).await,
        Some(ProjectAction::Add { slug, title }) => add(service, &slug, title.as_deref()).await,
        Some(ProjectAction::Switch { slug }) => switch(config, service, &slug).await,
        Some(ProjectAction::Rename { slug, new_title }) => rename(service, &slug, &new_title).await,
        Some(ProjectAction::Reslug { slug, new_slug }) => {
            reslug(config, service, &slug, &new_slug).await
        }
        Some(ProjectAction::Remove { slug }) => remove(config, service, &slug).await,
    }
}

/// Shows the active project slug and title.
#[allow(clippy::unnecessary_wraps)]
fn show_active(config: &Config) -> CliResult<()> {
    match config.active_project() {
        Some(slug) => println!("{slug}"),
        None => println!("no active project"),
    }
    Ok(())
}

/// Lists all projects.
async fn list<R>(service: &ProjectService<R>) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    let projects = service.list().await?;

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
async fn add<R>(service: &ProjectService<R>, slug: &str, title: Option<&str>) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    let project = service.create(slug, title).await?;
    match &project.title {
        Some(title) => println!("created: {} - {title}", project.slug),
        None => println!("created: {}", project.slug),
    }
    Ok(())
}

/// Switches the active project.
async fn switch<R>(config: &mut Config, service: &ProjectService<R>, slug: &str) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    service.find_by(slug).await?;
    config.set_active(slug)?;
    println!("switched to: {slug}");
    Ok(())
}

/// Renames a project (changes display title).
async fn rename<R>(service: &ProjectService<R>, slug: &str, new_title: &str) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    service.rename(slug, new_title).await?;
    println!("renamed: {slug} -> {new_title}");
    Ok(())
}

/// Changes the slug of a project.
async fn reslug<R>(
    config: &mut Config,
    service: &ProjectService<R>,
    slug: &str,
    new_slug: &str,
) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    service.reslug(slug, new_slug).await?;

    if config.active_project() == Some(slug) {
        config.set_active(new_slug)?;
    }

    println!("reslugged: {slug} -> {new_slug}");
    Ok(())
}

/// Deletes a project and all its tasks.
async fn remove<R>(config: &mut Config, service: &ProjectService<R>, slug: &str) -> CliResult<()>
where
    R: ProjectRepository + Transactional,
{
    service.delete(slug).await?;

    if config.active_project() == Some(slug) {
        config.active_project = None;
        config.save()?;
    }

    println!("removed: {slug}");
    Ok(())
}
