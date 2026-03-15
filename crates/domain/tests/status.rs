//! Integration tests for `Status` transitions and classification methods.

use domain::model::Status;

#[test]
fn allowed_transitions() {
    assert!(Status::NotStarted.can_transit(&Status::InProgress));
    assert!(Status::NotStarted.can_transit(&Status::Blocked));
    assert!(Status::InProgress.can_transit(&Status::Done));
    assert!(Status::InProgress.can_transit(&Status::Blocked));
    assert!(Status::Blocked.can_transit(&Status::InProgress));
}

#[test]
fn reset_allowed_from_any_status() {
    assert!(Status::InProgress.can_transit(&Status::NotStarted));
    assert!(Status::Blocked.can_transit(&Status::NotStarted));
    assert!(Status::Done.can_transit(&Status::NotStarted));
    assert!(Status::Abandoned.can_transit(&Status::NotStarted));
}

#[test]
fn abandon_allowed_from_any_status() {
    assert!(Status::NotStarted.can_transit(&Status::Abandoned));
    assert!(Status::InProgress.can_transit(&Status::Abandoned));
    assert!(Status::Done.can_transit(&Status::Abandoned));
    assert!(Status::Blocked.can_transit(&Status::Abandoned));
}

#[test]
fn forbidden_transitions() {
    assert!(!Status::NotStarted.can_transit(&Status::Done));
    assert!(!Status::Blocked.can_transit(&Status::Done));
    assert!(!Status::Done.can_transit(&Status::InProgress));
    assert!(!Status::Done.can_transit(&Status::Blocked));
}

#[test]
fn is_active() {
    assert!(Status::NotStarted.is_active());
    assert!(Status::InProgress.is_active());
    assert!(!Status::Done.is_active());
    assert!(!Status::Blocked.is_active());
    assert!(!Status::Abandoned.is_active());
}

#[test]
fn is_closed() {
    assert!(Status::Done.is_closed());
    assert!(Status::Blocked.is_closed());
    assert!(Status::Abandoned.is_closed());
    assert!(!Status::NotStarted.is_closed());
    assert!(!Status::InProgress.is_closed());
}

#[test]
fn is_reportable() {
    assert!(Status::NotStarted.is_reportable());
    assert!(Status::InProgress.is_reportable());
    assert!(Status::Done.is_reportable());
    assert!(Status::Blocked.is_reportable());
    assert!(!Status::Abandoned.is_reportable());
}

#[test]
fn icons() {
    assert_eq!(Status::NotStarted.icon(), "❌");
    assert_eq!(Status::InProgress.icon(), "🔄");
    assert_eq!(Status::Done.icon(), "✅");
    assert_eq!(Status::Blocked.icon(), "🛑");
    assert_eq!(Status::Abandoned.icon(), "🚫");
}
