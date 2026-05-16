//! Integration tests for `Status` transitions and classification methods.

use domain::model::Status::*;

#[test]
fn allowed_transitions() {
    assert!(NotStarted.can_transit(&InProgress));
    assert!(NotStarted.can_transit(&Done));
    assert!(NotStarted.can_transit(&Blocked));
    assert!(InProgress.can_transit(&Done));
    assert!(InProgress.can_transit(&Blocked));
    assert!(Blocked.can_transit(&InProgress));
}

#[test]
fn reset_allowed_from_any_status() {
    assert!(InProgress.can_transit(&NotStarted));
    assert!(Blocked.can_transit(&NotStarted));
    assert!(Done.can_transit(&NotStarted));
    assert!(Abandoned.can_transit(&NotStarted));
}

#[test]
fn abandon_allowed_from_any_status() {
    assert!(NotStarted.can_transit(&Abandoned));
    assert!(InProgress.can_transit(&Abandoned));
    assert!(Done.can_transit(&Abandoned));
    assert!(Blocked.can_transit(&Abandoned));
}

#[test]
fn forbidden_transitions() {
    assert!(!Blocked.can_transit(&Done));
    assert!(!Done.can_transit(&InProgress));
    assert!(!Done.can_transit(&Blocked));
}

#[test]
fn is_active() {
    assert!(NotStarted.is_active());
    assert!(InProgress.is_active());
    assert!(!Done.is_active());
    assert!(Blocked.is_active());
    assert!(!Abandoned.is_active());
}

#[test]
fn is_closed() {
    assert!(Done.is_closed());
    assert!(!Blocked.is_closed());
    assert!(Abandoned.is_closed());
    assert!(!NotStarted.is_closed());
    assert!(!InProgress.is_closed());
}

#[test]
fn is_reportable() {
    assert!(NotStarted.is_reportable());
    assert!(InProgress.is_reportable());
    assert!(Done.is_reportable());
    assert!(Blocked.is_reportable());
    assert!(!Abandoned.is_reportable());
}

#[test]
fn icons() {
    assert_eq!(NotStarted.icon(), "❌");
    assert_eq!(InProgress.icon(), "🔄");
    assert_eq!(Done.icon(), "✅");
    assert_eq!(Blocked.icon(), "🛑");
    assert_eq!(Abandoned.icon(), "🚫");
}
