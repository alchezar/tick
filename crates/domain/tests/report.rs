//! Integration tests for `ReportService` helpers and `Report::render`.

mod common;

use std::collections::HashSet;

use chrono::{Duration, Utc};

use domain::{
    model::{Project, Status, StatusChange, Task},
    repository::{ProjectRepository, TaskRepository},
    service::{self, Report, ReportService},
};

use common::fake::FakeRepo;

#[test]
fn monday_returns_friday() {
    // 2025-02-17 is Monday
    assert_eq!(
        service::prev_workday(common::date(2026, 2, 23)),
        common::date(2026, 2, 20)
    );
}

#[test]
fn tuesday_returns_monday() {
    assert_eq!(
        service::prev_workday(common::date(2026, 2, 17)),
        common::date(2026, 2, 16)
    );
}

#[test]
fn friday_returns_thursday() {
    assert_eq!(
        service::prev_workday(common::date(2026, 2, 20)),
        common::date(2026, 2, 19)
    );
}

#[test]
fn render_formats_hierarchy() {
    let project = Project::default();
    let mut root = Task::new("Milestone", None, project.id);
    root.order = Some(0);
    root.update_status(Status::InProgress, None).unwrap();

    let mut child = Task::new("Task", Some(root.id), project.id);
    child.order = Some(0);

    let mut grandchild = Task::new("Subtask", Some(child.id), project.id);
    grandchild.order = Some(0);
    grandchild.update_status(Status::Done, None).unwrap();

    // Morning plan: both created today -> NotStarted
    let morning_root = root.clone().with_status(Status::NotStarted);
    let morning_child = child.clone().with_status(Status::NotStarted);

    let report = Report::new(
        "default",
        None,
        vec![grandchild],
        vec![],
        vec![morning_root, morning_child],
        vec![root, child],
    );

    let rendered = report.render(true, true);

    assert!(rendered.contains("default\n\n"));
    assert!(rendered.contains("Previously:\n"));
    assert!(rendered.contains(" - ✅ Subtask\n"));
    // Today section - morning plan: all created today -> ❌
    assert!(rendered.contains("Today:\n"));
    assert!(rendered.contains("Today:\n - ❌ Milestone\n - - ❌ Task\n"));
    // Current section - real icons
    assert!(rendered.contains("Current:\n"));
    assert!(rendered.contains("Current:\n - 🔄 Milestone\n - - ❌ Task\n"));
}

#[test]
fn render_real_world_report() {
    let today = Utc::now().date_naive();
    let yesterday = (today - Duration::days(1))
        .and_hms_opt(10, 0, 0)
        .unwrap()
        .and_utc();

    // Shared root tasks - created yesterday (existing tasks)
    let project = Project::default();
    let mut task1 = Task::new("Task 1: runtime token validation", None, project.id);
    task1.created = yesterday;
    task1.update_status(Status::InProgress, None).unwrap();
    task1.order = Some(0);
    task1.pull_request_number = Some(42);

    let mut ip = Task::new("IP token encryption", None, project.id);
    ip.created = yesterday;
    ip.update_status(Status::InProgress, None).unwrap();
    ip.order = Some(1);
    ip.pull_request_number = Some(15);

    let mut ci = Task::new("CI build fix", None, project.id);
    ci.created = yesterday;
    ci.update_status(Status::InProgress, None).unwrap();
    ci.order = Some(2);

    let mut task7 = Task::new("Task 7", None, project.id);
    task7.created = yesterday;
    task7.update_status(Status::InProgress, None).unwrap();
    task7.order = Some(3);

    // Previously: done children (created yesterday)
    let make_done = |title, parent, order| {
        let mut t = Task::new(title, Some(parent), project.id);
        t.created = yesterday;
        t.update_status(Status::Done, None).unwrap();
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
        let mut t = Task::new(title, Some(parent), project.id);
        t.order = Some(order);
        t
    };

    let mut t1 = make_todo("replace error_tools with thiserror", ci.id, 3);
    t1.update_status(Status::Done, None).unwrap();
    let t2 = make_todo("move build jobs limit to ci workflow", ci.id, 4);
    let t3 = make_todo("workspace feature flags rule", ci.id, 5);
    let t4 = make_todo(
        "stub ufw in deploy tests and lint targets in Makefile",
        ci.id,
        6,
    );
    let t5 = make_todo("IF NOT EXISTS to migration 002 guard table", ci.id, 7);
    let t6 = make_todo("apply fmt and clippy fix for related modules", task7.id, 0);

    // Morning plan: root tasks were InProgress yesterday, new children NotStarted
    let morning_t1 = make_todo("replace error_tools with thiserror", ci.id, 3);
    let morning_t2 = make_todo("move build jobs limit to ci workflow", ci.id, 4);
    let morning_t3 = make_todo("workspace feature flags rule", ci.id, 5);
    let morning_t4 = make_todo(
        "stub ufw in deploy tests and lint targets in Makefile",
        ci.id,
        6,
    );
    let morning_t5 = make_todo("IF NOT EXISTS to migration 002 guard table", ci.id, 7);
    let morning_t6 = make_todo("apply fmt and clippy fix for related modules", task7.id, 0);

    let report = Report::new(
        "default",
        Some("https://github.com/owner/repo".to_owned()),
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
        vec![],
        vec![
            task1.clone(),
            ip.clone(),
            ci.clone(),
            morning_t1,
            morning_t2,
            morning_t3,
            morning_t4,
            morning_t5,
            task7.clone(),
            morning_t6,
        ],
        vec![task1, ip, ci, t1, t2, t3, t4, t5, task7, t6],
    );

    // Root tasks created yesterday -> keep 🔄 in Today
    // Child tasks created today -> ❌ in Today
    let expected = concat!(
        "default\n\n",
        "https://github.com/owner/repo/pull/15\n",
        "https://github.com/owner/repo/pull/42\n",
        "\n",
        " Previously:\n",
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
        " Today:\n",
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
        " Current:\n",
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

    assert_eq!(report.render(true, true), expected);
}

#[test]
fn today_section_shows_morning_status() {
    let yesterday = Utc::now().date_naive() - Duration::days(1);

    let project = Project::default();

    // Task created yesterday, completed today -> Today: 🔄 (was InProgress yesterday), Current: ✅
    let mut done_today = Task::new("Finished task", None, project.id);
    done_today.order = Some(0);
    done_today.created = yesterday.and_hms_opt(10, 0, 0).unwrap().and_utc();
    done_today.update_status(Status::InProgress, None).unwrap();
    done_today.update_status(Status::Done, None).unwrap();

    // Task created yesterday, still in progress -> Today: 🔄, Current: 🔄
    let mut still_active = Task::new("Active task", None, project.id);
    still_active.order = Some(1);
    still_active.created = yesterday.and_hms_opt(10, 0, 0).unwrap().and_utc();
    still_active
        .update_status(Status::InProgress, None)
        .unwrap();

    // Task created today, not started -> Today: ❌, Current: ❌
    let mut new_task = Task::new("New task", None, project.id);
    new_task.order = Some(2);

    // Morning statuses: done_today was InProgress yesterday, new_task didn't exist
    let morning_done = done_today.clone().with_status(Status::InProgress);
    let morning_active = still_active.clone();
    let morning_new = new_task.clone();

    let report = Report::new(
        "default",
        None,
        vec![],
        vec![],
        vec![morning_done, morning_active, morning_new],
        vec![done_today, still_active, new_task],
    );

    let expected = concat!(
        "default\n\n",
        " Today:\n",
        " - 🔄 Finished task\n",
        " - 🔄 Active task\n",
        " - ❌ New task\n",
        "\n",
        " Current:\n",
        " - ✅ Finished task\n",
        " - 🔄 Active task\n",
        " - ❌ New task\n",
    );

    assert_eq!(report.render(true, true), expected);
}

#[tokio::test]
async fn generate_today_includes_active_and_changed() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());

    let today = Utc::now().date_naive();
    let yesterday = today - Duration::days(1);

    let project = Project::default();

    // Task A: created yesterday, started yesterday, still active
    let mut a = Task::new("Task A", None, project.id);
    a.created = common::datetime(yesterday, 9);
    a.order = Some(0);
    repo.save_task(&a).await.unwrap();
    let mut ch = StatusChange::new(a.id, Status::NotStarted, Status::InProgress, None);
    ch.changed_at = common::datetime(yesterday, 10);
    repo.save_task_change(&ch).await.unwrap();

    // Task B: created yesterday, completed today
    let mut b = Task::new("Task B", None, project.id);
    b.created = common::datetime(yesterday, 9);
    b.order = Some(1);
    repo.save_task(&b).await.unwrap();
    let mut ch1 = StatusChange::new(b.id, Status::NotStarted, Status::InProgress, None);
    ch1.changed_at = common::datetime(yesterday, 11);
    repo.save_task_change(&ch1).await.unwrap();
    let mut ch2 = StatusChange::new(b.id, Status::InProgress, Status::Done, None);
    ch2.changed_at = common::datetime(today, 14);
    repo.save_task_change(&ch2).await.unwrap();

    let report = report_svc.generate(today, &project).await.unwrap();

    // Current: A = InProgress, B = Done (changed today)
    assert_eq!(report.current.len(), 2);
    let task_a = report.current.iter().find(|t| t.id == a.id).unwrap();
    let task_b = report.current.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(task_a.status(), Status::InProgress);
    assert_eq!(task_b.status(), Status::Done);

    // Today (morning): A = InProgress (unchanged), B = InProgress (was InProgress yesterday)
    assert_eq!(report.today.len(), 2);
    let morning_a = report.today.iter().find(|t| t.id == a.id).unwrap();
    let morning_b = report.today.iter().find(|t| t.id == b.id).unwrap();
    assert_eq!(morning_a.status(), Status::InProgress);
    assert_eq!(morning_b.status(), Status::InProgress);
}

#[tokio::test]
async fn generate_past_date_reconstructs_status() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let monday = common::date(2026, 2, 23);
    let tuesday = common::date(2026, 2, 24);

    // Task: created monday, started monday, done tuesday
    let mut task = Task::new("Feature X", None, project.id);
    task.created = common::datetime(monday, 9);
    task.order = Some(0);
    repo.save_task(&task).await.unwrap();

    let mut ch1 = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    ch1.changed_at = common::datetime(monday, 10);
    repo.save_task_change(&ch1).await.unwrap();

    let mut ch2 = StatusChange::new(task.id, Status::InProgress, Status::Done, None);
    ch2.changed_at = common::datetime(tuesday, 14);
    repo.save_task_change(&ch2).await.unwrap();

    // Report for monday: task was InProgress
    let report_mon = report_svc.generate(monday, &project).await.unwrap();
    assert_eq!(report_mon.current.len(), 1);
    assert_eq!(report_mon.current[0].status(), Status::InProgress);

    // Report for tuesday: task was Done (appears because it changed that day)
    let report_tue = report_svc.generate(tuesday, &project).await.unwrap();
    assert_eq!(report_tue.current.len(), 1);
    assert_eq!(report_tue.current[0].status(), Status::Done);
}

#[tokio::test]
async fn generate_excludes_tasks_created_after_date() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let monday = common::date(2026, 2, 23);
    let tuesday = common::date(2026, 2, 24);

    // Task created on tuesday
    let mut task = Task::new("Future task", None, project.id);
    task.created = common::datetime(tuesday, 9);
    task.order = Some(0);
    repo.save_task(&task).await.unwrap();

    // Report for monday: task doesn't exist yet
    let report = report_svc.generate(monday, &project).await.unwrap();
    assert!(report.current.is_empty());
    assert!(report.prev.is_empty());
}

#[tokio::test]
async fn generate_block_cascade_in_historical_report() {
    use domain::service::TaskService;

    let repo = FakeRepo::default();
    let task_svc = TaskService::new(repo.clone());
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let today = Utc::now().date_naive();

    // Create parent + child, start both, then block parent (cascades)
    let parent = task_svc
        .create("Parent", None, project.id, None, None, None)
        .await
        .unwrap();
    let child = task_svc
        .create("Child", Some(parent.id), project.id, None, None, None)
        .await
        .unwrap();
    task_svc.start(&parent.id, None).await.unwrap();
    task_svc.start(&child.id, None).await.unwrap();
    task_svc.block(&parent.id, None).await.unwrap();

    let report = report_svc.generate(today, &project).await.unwrap();

    // Both should appear (status changed today) and both should be Blocked
    let parent_task = report.current.iter().find(|t| t.id == parent.id).unwrap();
    let child_task = report.current.iter().find(|t| t.id == child.id).unwrap();
    assert_eq!(parent_task.status(), Status::Blocked);
    assert_eq!(child_task.status(), Status::Blocked);
}

#[test]
fn saturday_returns_friday() {
    // 2026-02-28 is Saturday
    assert_eq!(
        service::prev_workday(common::date(2026, 2, 28)),
        common::date(2026, 2, 27)
    );
}

#[test]
fn sunday_returns_friday() {
    // 2026-03-01 is Sunday - should return Friday, not Saturday
    assert_eq!(
        service::prev_workday(common::date(2026, 3, 1)),
        common::date(2026, 2, 27)
    );
}

#[tokio::test]
async fn weekend_on_sunday_includes_saturday_changes() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let d_fri = common::date(2026, 2, 27);
    let d_sat = common::date(2026, 2, 28);
    let d_sun = common::date(2026, 3, 1);

    let mut on_fri = Task::new("Friday task", None, project.id);
    on_fri.created = common::datetime(d_fri, 8);
    on_fri.order = Some(0);
    repo.save_task(&on_fri).await.unwrap();
    let mut ch_done = StatusChange::new(on_fri.id, Status::NotStarted, Status::Done, None);
    ch_done.changed_at = common::datetime(d_fri, 15);
    repo.save_task_change(&ch_done).await.unwrap();

    let mut on_sat = Task::new("Saturday task", None, project.id);
    on_sat.created = common::datetime(d_sat, 8);
    on_sat.order = Some(1);
    repo.save_task(&on_sat).await.unwrap();
    let mut ch_started = StatusChange::new(on_sat.id, Status::NotStarted, Status::InProgress, None);
    ch_started.changed_at = common::datetime(d_sat, 11);
    repo.save_task_change(&ch_started).await.unwrap();

    let report = report_svc.generate(d_sun, &project).await.unwrap();

    let prev_ids = report.prev.iter().map(|t| t.id).collect::<HashSet<_>>();
    let weekend_ids = report.weekend.iter().map(|t| t.id).collect::<HashSet<_>>();

    assert!(prev_ids.contains(&on_fri.id), "Friday task in Previously");
    assert!(!prev_ids.contains(&on_sat.id));
    assert!(weekend_ids.contains(&on_sat.id), "Saturday task in Weekend");
    assert!(!weekend_ids.contains(&on_fri.id));
}

#[tokio::test]
async fn weekend_on_monday_includes_saturday_and_sunday_changes() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let d_fri = common::date(2026, 2, 27);
    let d_sat = common::date(2026, 2, 28);
    let d_sun = common::date(2026, 3, 1);
    let d_mon = common::date(2026, 3, 2);

    let mut created = Vec::new();
    for (idx, (title, day)) in [("Fri", d_fri), ("Sat", d_sat), ("Sun", d_sun)]
        .into_iter()
        .enumerate()
    {
        let mut task = Task::new(title, None, project.id);
        task.created = common::datetime(day, 8);
        task.order = Some(idx);
        repo.save_task(&task).await.unwrap();
        let mut ch = StatusChange::new(task.id, Status::NotStarted, Status::Done, None);
        ch.changed_at = common::datetime(day, 15);
        repo.save_task_change(&ch).await.unwrap();
        created.push((title, task));
    }

    let report = report_svc.generate(d_mon, &project).await.unwrap();

    let prev_ids = report.prev.iter().map(|t| t.id).collect::<HashSet<_>>();
    let weekend_ids = report.weekend.iter().map(|t| t.id).collect::<HashSet<_>>();

    for (label, task) in &created {
        match *label {
            "Fri" => assert!(prev_ids.contains(&task.id), "Fri in Previously"),
            "Sat" | "Sun" => assert!(weekend_ids.contains(&task.id), "{label} in Weekend"),
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn weekend_empty_on_weekday_reports() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let tuesday = common::date(2026, 2, 24);
    let report = report_svc.generate(tuesday, &project).await.unwrap();
    assert!(report.weekend.is_empty());
}

#[tokio::test]
async fn weekend_empty_when_no_weekend_changes() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let d_fri = common::date(2026, 2, 27);
    let d_mon = common::date(2026, 3, 2);

    let mut task = Task::new("Fri only", None, project.id);
    task.created = common::datetime(d_fri, 8);
    task.order = Some(0);
    repo.save_task(&task).await.unwrap();
    let mut ch = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    ch.changed_at = common::datetime(d_fri, 15);
    repo.save_task_change(&ch).await.unwrap();

    let report = report_svc.generate(d_mon, &project).await.unwrap();
    assert!(report.weekend.is_empty());
}

#[tokio::test]
async fn weekend_section_includes_ancestors() {
    let repo = FakeRepo::default();
    let report_svc = ReportService::new(repo.clone());
    let project = Project::default();

    let d_fri = common::date(2026, 2, 27);
    let d_sat = common::date(2026, 2, 28);
    let d_sun = common::date(2026, 3, 1);

    // Parent: created Friday, started Friday (no weekend change on itself).
    let mut parent = Task::new("Parent", None, project.id);
    parent.created = common::datetime(d_fri, 8);
    parent.order = Some(0);
    repo.save_task(&parent).await.unwrap();
    let mut parent_start =
        StatusChange::new(parent.id, Status::NotStarted, Status::InProgress, None);
    parent_start.changed_at = common::datetime(d_fri, 10);
    repo.save_task_change(&parent_start).await.unwrap();

    // Child: completed on Saturday.
    let mut child = Task::new("Child", Some(parent.id), project.id);
    child.created = common::datetime(d_fri, 9);
    child.order = Some(0);
    repo.save_task(&child).await.unwrap();
    let mut child_done = StatusChange::new(child.id, Status::NotStarted, Status::Done, None);
    child_done.changed_at = common::datetime(d_sat, 12);
    repo.save_task_change(&child_done).await.unwrap();

    let report = report_svc.generate(d_sun, &project).await.unwrap();
    let weekend_ids = report.weekend.iter().map(|t| t.id).collect::<HashSet<_>>();

    assert!(
        weekend_ids.contains(&parent.id),
        "parent must be included for hierarchy context"
    );
    assert!(weekend_ids.contains(&child.id));
}

#[test]
fn render_all_combines_multiple_projects() {
    let mut task_a = Task::new("Task A", None, Project::default().id);
    task_a.order = Some(0);

    let mut task_b = Task::new("Task B", None, Project::default().id);
    task_b.order = Some(0);

    let r1 = Report::new(
        "Work",
        None,
        vec![],
        vec![],
        vec![task_a.clone()],
        vec![task_a],
    );
    let r2 = Report::new(
        "Personal",
        None,
        vec![],
        vec![],
        vec![task_b.clone()],
        vec![task_b],
    );

    let output = service::render_all(&[r1, r2], true);

    assert!(output.contains("Work\n\n"));
    assert!(output.contains("Personal\n\n"));
    assert!(output.contains("Task A"));
    assert!(output.contains("Task B"));
}

#[test]
fn render_all_skips_empty_reports() {
    let mut task = Task::new("Task", None, Project::default().id);
    task.order = Some(0);

    let empty = Report::new("Empty", None, vec![], vec![], vec![], vec![]);
    let filled = Report::new(
        "Filled",
        None,
        vec![],
        vec![],
        vec![task.clone()],
        vec![task],
    );

    let output = service::render_all(&[empty, filled], true);

    assert!(!output.contains("Empty"));
    assert!(output.contains("Filled\n\n"));
}

#[test]
fn render_all_empty_input() {
    let output = service::render_all(&[], true);
    assert!(output.is_empty());
}

#[tokio::test]
async fn generate_all_returns_reports_for_all_projects() {
    let repo = FakeRepo::default();

    let work = Project::new("work", Some("Work Projects"));
    let personal = Project::new("personal", None::<String>);
    repo.save_project(&work).await.unwrap();
    repo.save_project(&personal).await.unwrap();

    let today = Utc::now().date_naive();

    let mut t1 = Task::new("Work task", None, work.id);
    t1.order = Some(0);
    repo.save_task(&t1).await.unwrap();
    let mut ch1 = StatusChange::new(t1.id, Status::NotStarted, Status::InProgress, None);
    ch1.changed_at = common::datetime(today, 9);
    repo.save_task_change(&ch1).await.unwrap();

    let mut t2 = Task::new("Personal task", None, personal.id);
    t2.order = Some(0);
    repo.save_task(&t2).await.unwrap();
    let mut ch2 = StatusChange::new(t2.id, Status::NotStarted, Status::InProgress, None);
    ch2.changed_at = common::datetime(today, 10);
    repo.save_task_change(&ch2).await.unwrap();

    let report_svc = ReportService::new(repo);
    let reports = report_svc.generate_all(today).await.unwrap();

    assert_eq!(reports.len(), 2);

    let titles = reports.iter().map(|r| r.title.as_str()).collect::<Vec<_>>();
    assert!(titles.contains(&"Work Projects"));
    assert!(titles.contains(&"personal"));
}

#[tokio::test]
async fn generate_all_empty_project_produces_empty_report() {
    let repo = FakeRepo::default();

    let project = Project::new("empty", None::<String>);
    repo.save_project(&project).await.unwrap();

    let today = Utc::now().date_naive();
    let report_svc = ReportService::new(repo);
    let reports = report_svc.generate_all(today).await.unwrap();

    assert_eq!(reports.len(), 1);
    assert!(reports[0].prev.is_empty());
    assert!(reports[0].today.is_empty());
    assert!(reports[0].current.is_empty());
    assert!(reports[0].render(true, true).is_empty());
}

#[tokio::test]
async fn generate_all_no_projects() {
    let repo = FakeRepo::default();
    let today = Utc::now().date_naive();
    let report_svc = ReportService::new(repo);

    let reports = report_svc.generate_all(today).await.unwrap();
    assert!(reports.is_empty());
}

#[test]
fn render_includes_pr_links_when_github_url_set() {
    let project = Project::default();

    let mut task = Task::new("Feature", None, project.id);
    task.order = Some(0);
    task.pull_request_number = Some(42);
    let task = task.with_status(Status::InProgress);

    let report = Report::new(
        "Work",
        Some("https://github.com/owner/repo".to_owned()),
        vec![],
        vec![],
        vec![task.clone()],
        vec![task],
    );

    let output = report.render(true, true);
    assert!(output.contains("https://github.com/owner/repo/pull/42"));
}

#[test]
fn render_no_pr_links_without_github_url() {
    let project = Project::default();

    let mut task = Task::new("Feature", None, project.id);
    task.order = Some(0);
    task.pull_request_number = Some(42);
    let task = task.with_status(Status::InProgress);

    let report = Report::new("Work", None, vec![], vec![], vec![task.clone()], vec![task]);

    let output = report.render(true, true);
    assert!(!output.contains("pull/42"));
}

#[test]
fn render_no_pr_links_for_closed_tasks() {
    let project = Project::default();

    let mut task = Task::new("Done task", None, project.id);
    task.order = Some(0);
    task.pull_request_number = Some(10);
    let task = task.with_status(Status::Done);

    let report = Report::new(
        "Work",
        Some("https://github.com/owner/repo".to_owned()),
        vec![],
        vec![],
        vec![task.clone()],
        vec![task],
    );

    let output = report.render(true, true);
    assert!(!output.contains("pull/10"));
}

#[test]
fn render_pr_links_deduped_and_sorted() {
    let project = Project::default();

    let mut t1 = Task::new("Task A", None, project.id);
    t1.order = Some(0);
    t1.pull_request_number = Some(99);
    let t1 = t1.with_status(Status::InProgress);

    let mut t2 = Task::new("Task B", None, project.id);
    t2.order = Some(1);
    t2.pull_request_number = Some(10);
    let t2 = t2.with_status(Status::InProgress);

    let mut t3 = Task::new("Task C", None, project.id);
    t3.order = Some(2);
    t3.pull_request_number = Some(99);
    let t3 = t3.with_status(Status::Blocked);

    let report = Report::new(
        "Work",
        Some("https://github.com/owner/repo".to_owned()),
        vec![],
        vec![],
        vec![t1.clone(), t2.clone(), t3.clone()],
        vec![t1, t2, t3],
    );

    let output = report.render(true, true);
    let pr_section: String = output
        .lines()
        .take_while(|l| !l.starts_with(' '))
        .filter(|l| l.contains("pull/"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(pr_section.contains("pull/10"));
    assert!(pr_section.contains("pull/99"));
    // pull/10 should come before pull/99 (sorted)
    let pos_10 = pr_section.find("pull/10").unwrap();
    let pos_99 = pr_section.find("pull/99").unwrap();
    assert!(pos_10 < pos_99);
}

#[tokio::test]
async fn generate_includes_github_url_in_report() {
    let repo = FakeRepo::default();

    let mut project = Project::new("work", Some("Work"));
    project.github_url = Some("https://github.com/owner/repo".to_owned());
    repo.save_project(&project).await.unwrap();

    let today = Utc::now().date_naive();

    let mut task = Task::new("Feature", None, project.id);
    task.order = Some(0);
    task.pull_request_number = Some(55);
    repo.save_task(&task).await.unwrap();

    let mut ch = StatusChange::new(task.id, Status::NotStarted, Status::InProgress, None);
    ch.changed_at = common::datetime(today, 9);
    repo.save_task_change(&ch).await.unwrap();

    let report_svc = ReportService::new(repo);
    let report = report_svc.generate(today, &project).await.unwrap();

    assert_eq!(
        report.github_url.as_deref(),
        Some("https://github.com/owner/repo")
    );
    let output = report.render(true, true);
    assert!(output.contains("https://github.com/owner/repo/pull/55"));
}
