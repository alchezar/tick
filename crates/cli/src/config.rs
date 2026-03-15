//! Persistent CLI configuration (active project, etc.).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CliError, CliResult};

/// File name for the config inside the data directory.
pub const CONFIG_FILE: &str = "config.toml";

/// Application directory name under XDG data dir.
const APP_DIR: &str = "tick";

/// Persistent configuration stored at `~/.local/share/tick/config.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    /// Slug of the currently active project.
    pub active_project: Option<String>,
}

impl Config {
    /// Loads config from the default XDG data directory.
    ///
    /// # Errors
    /// Returns [`CliError::ConfigRead`] if the file exists but cannot be read or parsed.
    pub fn load() -> CliResult<Self> {
        Self::load_from(&Self::default_path()?)
    }

    /// Writes config to the default XDG data directory.
    ///
    /// # Errors
    /// Returns [`CliError::ConfigWrite`] if the file cannot be written.
    pub fn save(&self) -> CliResult<()> {
        self.save_to(&Self::default_path()?)
    }

    /// Loads config from a specific path, returning defaults if the file does not exist.
    ///
    /// # Errors
    /// Returns [`CliError::ConfigRead`] if the file exists but cannot be read or parsed.
    pub fn load_from(path: &Path) -> CliResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| CliError::ConfigRead {
            path: path.to_path_buf(),
            source: e.to_string(),
        })?;

        toml::from_str(&content).map_err(|e| CliError::ConfigRead {
            path: path.to_path_buf(),
            source: e.to_string(),
        })
    }

    /// Writes config to a specific path, creating directories as needed.
    ///
    /// # Errors
    /// Returns [`CliError::ConfigWrite`] if the file cannot be written.
    pub fn save_to(&self, path: &Path) -> CliResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CliError::ConfigWrite {
                path: path.to_path_buf(),
                source: e.to_string(),
            })?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| CliError::ConfigWrite {
            path: path.to_path_buf(),
            source: e.to_string(),
        })?;

        fs::write(path, content).map_err(|e| CliError::ConfigWrite {
            path: path.to_path_buf(),
            source: e.to_string(),
        })
    }

    /// Sets the active project slug and saves to disk.
    ///
    /// # Errors
    /// Returns an error if the config cannot be saved.
    pub fn set_active(&mut self, slug: &str) -> CliResult<()> {
        self.active_project = Some(slug.to_owned());
        self.save()
    }

    /// Returns the active project slug, if set.
    #[must_use]
    pub fn active_project(&self) -> Option<&str> {
        self.active_project.as_deref()
    }

    /// Returns the default config file path (`~/.local/share/tick/config.toml`).
    fn default_path() -> CliResult<PathBuf> {
        dirs::data_dir()
            .map(|d| d.join(APP_DIR).join(CONFIG_FILE))
            .ok_or(CliError::NoDataDir)
    }
}
