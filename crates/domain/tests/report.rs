//! Integration tests for `ReportService` helpers and `Report::render`.

use chrono::NaiveDate;

use domain::model::{Status, Task};
use domain::service::{self, Report};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[test]
fn monday_returns_friday() {
    // 2025-02-17 is Monday
    assert_eq!(service::prev_workday(date(2026, 2, 23)), date(2026, 2, 20));
}

#[test]
fn tuesday_returns_monday() {
    assert_eq!(service::prev_workday(date(2026, 2, 17)), date(2026, 2, 16));
}

#[test]
fn friday_returns_thursday() {
    assert_eq!(service::prev_workday(date(2026, 2, 20)), date(2026, 2, 19));
}

#[test]
fn render_formats_hierarchy() {
    let mut root = Task::new("Milestone", None);
    root.order = Some(0);
    root.update_status(Status::InProgress).unwrap();

    let mut child = Task::new("Task", Some(root.id));
    child.order = Some(0);

    let mut grandchild = Task::new("Subtask", Some(child.id));
    grandchild.order = Some(0);
    grandchild.update_status(Status::InProgress).unwrap();
    grandchild.update_status(Status::Done).unwrap();

    let report = Report {
        prev: vec![grandchild],
        today: vec![root, child],
    };

    let rendered = report.render();

    assert!(rendered.contains("Previously:\n"));
    assert!(rendered.contains(" - ✅ Subtask\n"));
    assert!(rendered.contains('\n'));
    assert!(rendered.contains("Today:\n"));
    assert!(rendered.contains(" - 🔄 Milestone\n"));
    assert!(rendered.contains(" - - ❌ Task\n"));
}

#[test]
fn render_real_world_report() {
    // Shared root tasks
    let mut task1 = Task::new("Task 1: runtime token validation", None);
    task1.update_status(Status::InProgress).unwrap();
    task1.order = Some(0);

    let mut ip = Task::new("IP token encryption", None);
    ip.update_status(Status::InProgress).unwrap();
    ip.order = Some(1);

    let mut ci = Task::new("CI build fix", None);
    ci.update_status(Status::InProgress).unwrap();
    ci.order = Some(2);

    let mut task7 = Task::new("Task 7", None);
    task7.update_status(Status::InProgress).unwrap();
    task7.order = Some(3);

    // Previously: done children
    let make_done = |title, parent, order| {
        let mut t = Task::new(title, Some(parent));
        t.update_status(Status::InProgress).unwrap();
        t.update_status(Status::Done).unwrap();
        t.order = Some(order);
        t
    };

    let c1 = make_done("consolidate dependencies into workspace", task1.id, 0);
    let c2 = make_done("jti claim optional", task1.id, 1);
    let c3 = make_done("test helpers and validate success path", task1.id, 2);
    let i1 = make_done("5-section bug-reproducer format", ip.id, 0);
    let i2 = make_done("eliminating ProviderKey duplication", ip.id, 1);
    let i3 = make_done("startup validation for IP_TOKEN_KEY", ip.id, 2);
    let b1 = make_done("stale docstrings in migrations", ci.id, 0);
    let b2 = make_done("misleading agents schema", ci.id, 1);
    let b3 = make_done("tab indentation and gitignore local config", ci.id, 2);

    // Today: active children
    let make_todo = |title, parent, order| {
        let mut t = Task::new(title, Some(parent));
        t.order = Some(order);
        t
    };

    let t1 = make_todo("replace error_tools with thiserror", ci.id, 3);
    let t2 = make_todo("move build jobs limit to ci workflow", ci.id, 4);
    let t3 = make_todo("workspace feature flags rule", ci.id, 5);
    let t4 = make_todo(
        "stub ufw in deploy tests and lint targets in Makefile",
        ci.id,
        6,
    );
    let t5 = make_todo("IF NOT EXISTS to migration 002 guard table", ci.id, 7);
    let t6 = make_todo("apply fmt and clippy fix for related modules", task7.id, 0);

    let report = Report {
        prev: vec![
            task1.clone(),
            c1,
            c2,
            c3,
            ip.clone(),
            i1,
            i2,
            i3,
            ci.clone(),
            b1,
            b2,
            b3,
        ],
        today: vec![task1, ip, ci, t1, t2, t3, t4, t5, task7, t6],
    };

    let expected = concat!(
        "Previously:\n",
        " - 🔄 Task 1: runtime token validation\n",
        " - - ✅ consolidate dependencies into workspace\n",
        " - - ✅ jti claim optional\n",
        " - - ✅ test helpers and validate success path\n",
        " - 🔄 IP token encryption\n",
        " - - ✅ 5-section bug-reproducer format\n",
        " - - ✅ eliminating ProviderKey duplication\n",
        " - - ✅ startup validation for IP_TOKEN_KEY\n",
        " - 🔄 CI build fix\n",
        " - - ✅ stale docstrings in migrations\n",
        " - - ✅ misleading agents schema\n",
        " - - ✅ tab indentation and gitignore local config\n",
        "\n",
        "Today:\n",
        " - 🔄 Task 1: runtime token validation\n",
        " - 🔄 IP token encryption\n",
        " - 🔄 CI build fix\n",
        " - - ❌ replace error_tools with thiserror\n",
        " - - ❌ move build jobs limit to ci workflow\n",
        " - - ❌ workspace feature flags rule\n",
        " - - ❌ stub ufw in deploy tests and lint targets in Makefile\n",
        " - - ❌ IF NOT EXISTS to migration 002 guard table\n",
        " - 🔄 Task 7\n",
        " - - ❌ apply fmt and clippy fix for related modules\n",
    );

    assert_eq!(report.render(), expected);
}
