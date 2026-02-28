//! Business logic for project management.

use crate::{
    error::{CoreError, CoreResult},
    model::Project,
    repository::ProjectRepository,
};

/// Encapsulates all business rules for project management.
///
/// Enforces slug uniqueness and provides CRUD operations for projects.
#[derive(Debug)]
pub struct ProjectService<R>
where
    R: ProjectRepository,
{
    repo: R,
}

impl<R> ProjectService<R>
where
    R: ProjectRepository,
{
    /// Creates a new `ProjectService` with the given repository.
    #[inline]
    #[must_use]
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Creates a new project and persists it.
    ///
    /// # Errors
    /// - [`CoreError::ProjectAlreadyExists`] if a project with this slug already exists.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn create(&self, slug: &str, title: Option<&str>) -> CoreResult<Project> {
        if self.repo.find_project_by_slug(slug)?.is_some() {
            return Err(CoreError::ProjectAlreadyExists {
                slug: slug.to_owned(),
            });
        }

        let project = Project::new(slug, title);
        self.repo.save_project(&project)?;
        Ok(project)
    }

    /// Returns the project with the given slug.
    ///
    /// # Errors
    /// - [`CoreError::ProjectNotFound`] if no project exists with this slug.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn find_by(&self, slug: &str) -> CoreResult<Project> {
        self.repo
            .find_project_by_slug(slug)?
            .ok_or(CoreError::ProjectNotFound {
                slug: slug.to_owned(),
            })
    }

    /// Returns all projects.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    #[inline]
    pub fn list(&self) -> CoreResult<Vec<Project>> {
        self.repo.list_projects()
    }

    /// Renames a project.
    ///
    /// # Errors
    /// Returns an error if the persistence operation fails.
    #[inline]
    pub fn rename(&self, slug: &str, new_title: &str) -> CoreResult<()> {
        let mut project = self.find_by(slug)?;
        project.title = Some(new_title.to_owned());
        self.repo.save_project(&project)
    }

    /// Changes the slug of a project.
    ///
    /// # Errors
    /// - [`CoreError::ProjectNotFound`] if no project exists with the current slug.
    /// - [`CoreError::ProjectAlreadyExists`] if `new_slug` is already taken.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn reslug(&self, slug: &str, new_slug: &str) -> CoreResult<()> {
        if self.repo.find_project_by_slug(new_slug)?.is_some() {
            return Err(CoreError::ProjectAlreadyExists {
                slug: new_slug.to_owned(),
            });
        }
        let mut project = self.find_by(slug)?;
        new_slug.clone_into(&mut project.slug);
        self.repo.save_project(&project)
    }

    /// Deletes a project by slug.
    ///
    /// Task cascade is handled at the db level.
    ///
    /// # Errors
    /// - [`CoreError::ProjectNotFound`] if no project exists with this slug.
    /// - Returns an error if the persistence operation fails.
    #[inline]
    pub fn delete(&self, slug: &str) -> CoreResult<()> {
        let project_id = &self
            .repo
            .find_project_by_slug(slug)?
            .ok_or(CoreError::ProjectNotFound {
                slug: slug.to_owned(),
            })?
            .id;
        self.repo.delete_project(project_id)
    }
}
