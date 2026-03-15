//! Integration tests for Config.

use std::fs;

use cli::config::{CONFIG_FILE, Config};

#[test]
fn load_returns_default_when_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(CONFIG_FILE);

    let config = Config::load_from(&path).unwrap();

    assert!(config.active_project().is_none());
}

#[test]
fn save_and_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(CONFIG_FILE);

    let config = Config {
        active_project: Some("work".to_owned()),
    };
    config.save_to(&path).unwrap();

    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.active_project(), Some("work"));
}

#[test]
fn save_creates_parent_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("dir").join(CONFIG_FILE);

    let config = Config::default();
    config.save_to(&path).unwrap();

    assert!(path.exists());
}

#[test]
fn active_project_returns_none_by_default() {
    let config = Config::default();
    assert!(config.active_project().is_none());
}

#[test]
fn load_parses_existing_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(CONFIG_FILE);

    fs::write(&path, r#"active_project = "my-proj""#).unwrap();

    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.active_project(), Some("my-proj"));
}

#[test]
fn load_handles_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(CONFIG_FILE);

    fs::write(&path, "").unwrap();

    let config = Config::load_from(&path).unwrap();
    assert!(config.active_project().is_none());
}

#[test]
fn load_returns_error_for_invalid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(CONFIG_FILE);

    fs::write(&path, "not valid { toml !!!").unwrap();

    let result = Config::load_from(&path);
    assert!(result.is_err());
}
