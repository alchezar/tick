//! Integration tests for `ReportService` helpers and `Report::render`.

mod fake;

use chrono::{Duration, NaiveDate, Utc};

use domain::model::{Status, StatusChange, Task};
use domain::repository::TaskRepository;
use domain::service::{self, Report, ReportService};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn datetime(date: NaiveDate, hour: u32) -> chrono::DateTime<Utc> {
    date.and_hms_opt(hour, 0, 0).unwrap().and_utc()
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
    let today = Utc::now().date_naive();

    let mut root = Task::new("Milestone", None);
    root.order = Some(0);
    root.update_status(Status::InProgress).unwrap();

    let mut child = Task::new("Task", Some(root.id));
    child.order = Some(0);

    let mut grandchild = Task::new("Subtask", Some(child.id));
    grandchild.order = Some(0);
    grandchild.update_status(Status::InProgress).unwrap();
    grandchild.update_status(Status::Done).unwrap();

    let report = Report::new(vec![grandchild], vec![root, child], today);

    let rendered = report.render();

    assert!(rendered.contains("Previously:\n"));
    assert!(rendered.contains(" - ✅ Subtask\n"));
    // Today section — morning plan: all created today -> ❌
    assert!(rendered.contains("Today:\n"));
    assert!(rendered.contains("Today:\n - ❌ Milestone\n - - ❌ Task\n"));
    // Current section — real icons
    assert!(rendered.contains("Current:\n"));
    assert!(rendered.contains("Current:\n - 🔄 Milestone\n - - ❌ Task\n"));
}

#[test]
fn render_real_world_report() {
    use chrono::Duration;

    let today = Utc::now().date_naive();
    let yesterday = (today - Duration::days(1))
        .and_hms_opt(10, 0, 0)
        .unwrap()
        .and_utc();

    // Shared root tasks — created yesterday (existing tasks)
    let mut task1 = Task::new("Task 1: runtime token validation", None);
    task1.created = yesterday;
    task1.update_status(Status::InProgress).unwrap();
    task1.order = Some(0);

    let mut ip = Task::new("IP token encryption", None);
    ip.created = yesterday;
    ip.update_status(Status::InProgress).unwrap();
    ip.order = Some(1);

    let mut ci = Task::new("CI build fix", None);
    ci.created = yesterday;
    ci.update_status(Status::InProgress).unwrap();
    ci.order = Some(2);

    let mut task7 = Task::new("Task 7", None);
    task7.created = yesterday;
    task7.update_status(Status::InProgress).unwrap();
    task7.order = Some(3);

    // Previously: done children (created yesterday)
    let make_done = |title, parent, order| {
        let mut t = Task::new(title, Some(parent));
        t.created = yesterday;
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

    // Today: new children (created today)
    let make_todo = |title, parent, order| {
        let mut t = Task::new(title, Some(parent));
        t.order = Some(order);
        t
    };

    let mut t1 = make_todo("replace error_tools with thiserror", ci.id, 3);
    t1.update_status(Status::InProgress).unwrap();
    t1.update_status(Status::Done).unwrap();
    let t2 = make_todo("move build jobs limit to ci workflow", ci.id, 4);
    let t3 = make_todo("workspace feature flags rule", ci.id, 5);
    let t4 = make_todo(
        "stub ufw in deploy tests and lint targets in Makefile",
        ci.id,
        6,
    );
    let t5 = make_todo("IF NOT EXISTS to migration 002 guard table", ci.id, 7);
    let t6 = make_todo("apply fmt and clippy fix for related modules", task7.id, 0);

    let report = Report::new(
        vec![
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
        vec![task1, ip, ci, t1, t2, t3, t4, t5, task7, t6],
        today,
    );

    // Root tasks created yesterday -> keep 🔄 in Today
    // Child tasks created today -> ❌ in Today
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
        "\n",
        "Current:\n",
        " - 🔄 Task 1: runtime token validation\n",
        " - 🔄 IP token encryption\n",
        " - 🔄 CI build fix\n",
        " - - ✅ replace error_tools with thiserror\n",
        " - - ❌ move build jobs limit to ci workflow\n",
        " - - ❌ workspace feature flags rule\n",
        " - - ❌ stub ufw in deploy tests and lint targets in Makefile\n",
        " - - ❌ IF NOT EXISTS to migration 002 guard table\n",
        " - 🔄 Task 7\n",
        " - - ❌ apply fmt and clippy fix for related modules\n",
    );

    assert_eq!(report.render(), expected);
}

#[test]
fn today_section_shows_old_done_as_planned() {
    use chrono::Duration;

    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    // Task created yesterday, completed today -> Today: ❌, Current: ✅
    let mut done_today = Task::new("Finished task", None);
    done_today.order = Some(0);
    done_today.created = yesterday.and_hms_opt(10, 0, 0).unwrap().and_utc();
    done_today.update_status(Status::InProgress).unwrap();
    done_today.update_status(Status::Done).unwrap();

    // Task created yesterday, still in progress -> Today: 🔄, Current: 🔄
    let mut still_active = Task::new("Active task", None);
    still_active.order = Some(1);
    still_active.created = yesterday.and_hms_opt(10, 0, 0).unwrap().and_utc();
    still_active.update_status(Status::InProgress).unwrap();

    // Task created today, not started → Today: ❌, Current: ❌
    let mut new_task = Task::new("New task", None);
    new_task.order = Some(2);

    let report = Report::new(vec![], vec![done_today, still_active, new_task], today);

    let expected = concat!(
        "Today:\n",
        " - ❌ Finished task\n",
        " - 🔄 Active task\n",
        " - ❌ New task\n",
        "\n",
        "Current:\n",
        " - ✅ Finished task\n",
        " - 🔄 Active task\n",
        " - ❌ New task\n",
    );

    assert_eq!(report.render(), expected);
}

#[test]
fn generate_today_includes_active_and_changed() {
    let repo = fake::FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());

    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    // Task A: created yesterday, started yesterday, still active
    let mut a = Task::new("Task A", None);
    a.created = datetime(yesterday, 9);
    a.order = Some(0);
    repo.save(&a).unwrap();
    let mut ch = StatusChange::new(a.id, Status::NotStarted, Status::InProgress);
    ch.changed_at = datetime(yesterday, 10);
    repo.save_status_change(&ch).unwrap();

    // Task B: created yesterday, completed today
    let mut b = Task::new("Task B", None);
    b.created = datetime(yesterday, 9);
    b.order = Some(1);
    repo.save(&b).unwrap();
    let mut ch1 = StatusChange::new(b.id, Status::NotStarted, Status::InProgress);
    ch1.changed_at = datetime(yesterday, 11);
    repo.save_status_change(&ch1).unwrap();
    let mut ch2 = StatusChange::new(b.id, Status::InProgress, Status::Done);
    ch2.changed_at = datetime(today, 14);
    repo.save_status_change(&ch2).unwrap();

    let report = report_svc.generate(today).unwrap();

    // Current: A = InProgress, B = Done (changed today)
    assert_eq!(report.current.len(), 2);
    let task_a = report.current.iter().find(|t| t.id == a.id).unwrap();
    let task_b = report.current.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(task_a.status(), Status::InProgress);
    assert_eq!(task_b.status(), Status::Done);
}

#[test]
fn generate_past_date_reconstructs_status() {
    let repo = fake::FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());

    let monday = date(2026, 2, 23);
    let tuesday = date(2026, 2, 24);

    // Task: created monday, started monday, done tuesday
    let mut task = Task::new("Feature X", None);
    task.created = datetime(monday, 9);
    task.order = Some(0);
    repo.save(&task).unwrap();

    let mut ch1 = StatusChange::new(task.id, Status::NotStarted, Status::InProgress);
    ch1.changed_at = datetime(monday, 10);
    repo.save_status_change(&ch1).unwrap();

    let mut ch2 = StatusChange::new(task.id, Status::InProgress, Status::Done);
    ch2.changed_at = datetime(tuesday, 14);
    repo.save_status_change(&ch2).unwrap();

    // Report for monday: task was InProgress
    let report_mon = report_svc.generate(monday).unwrap();
    assert_eq!(report_mon.current.len(), 1);
    assert_eq!(report_mon.current[0].status(), Status::InProgress);

    // Report for tuesday: task was Done (appears because it changed that day)
    let report_tue = report_svc.generate(tuesday).unwrap();
    assert_eq!(report_tue.current.len(), 1);
    assert_eq!(report_tue.current[0].status(), Status::Done);
}

#[test]
fn generate_excludes_tasks_created_after_date() {
    let repo = fake::FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());

    let monday = date(2026, 2, 23);
    let tuesday = date(2026, 2, 24);

    // Task created on tuesday
    let mut task = Task::new("Future task", None);
    task.created = datetime(tuesday, 9);
    task.order = Some(0);
    repo.save(&task).unwrap();

    // Report for monday: task doesn't exist yet
    let report = report_svc.generate(monday).unwrap();
    assert!(report.current.is_empty());
    assert!(report.prev.is_empty());
}

#[test]
fn generate_block_cascade_in_historical_report() {
    use domain::service::TaskService;

    let repo = fake::FakeRepo::default();
    let task_svc = TaskService::new(repo.clone());
    let report_svc = ReportService::new(repo.clone());

    let today = Utc::now().date_naive();

    // Create parent + child, start both, then block parent (cascades)
    let parent = task_svc.create("Parent", None).unwrap();
    let child = task_svc.create("Child", Some(&parent.id)).unwrap();
    task_svc.start(&parent.id).unwrap();
    task_svc.start(&child.id).unwrap();
    task_svc.block(&parent.id).unwrap();

    let report = report_svc.generate(today).unwrap();

    // Both should appear (status changed today) and both should be Blocked
    let parent_task = report.current.iter().find(|t| t.id == parent.id).unwrap();
    let child_task = report.current.iter().find(|t| t.id == child.id).unwrap();
    assert_eq!(parent_task.status(), Status::Blocked);
    assert_eq!(child_task.status(), Status::Blocked);
}
