//! Integration tests for [`RemoveGuard`].

use std::io::Cursor;

use cli::guard::{Confirm, RemoveGuard};

#[test]
fn confirm_accepts_y() {
    let mut guard = RemoveGuard::with_io(Cursor::new(b"y\n"), Vec::new());
    assert!(guard.confirm("task abc123").is_ok());
}

#[test]
fn confirm_accepts_uppercase_y() {
    let mut guard = RemoveGuard::with_io(Cursor::new(b"Y\n"), Vec::new());
    assert!(guard.confirm("task abc123").is_ok());
}

#[test]
fn confirm_rejects_n() {
    let mut guard = RemoveGuard::with_io(Cursor::new(b"n\n"), Vec::new());
    assert!(guard.confirm("task abc123").is_err());
}

#[test]
fn confirm_rejects_empty() {
    let mut guard = RemoveGuard::with_io(Cursor::new(b"\n"), Vec::new());
    assert!(guard.confirm("task abc123").is_err());
}

#[test]
fn prompt_contains_label() {
    let mut output = Vec::new();
    let mut guard = RemoveGuard::with_io(Cursor::new(b"n\n"), &mut output);
    let _ = guard.confirm("project foo");
    let prompt = String::from_utf8(output).unwrap();
    assert!(prompt.contains("project foo"));
}
