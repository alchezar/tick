# tick - Technical Specification

## Overview

`tick` is a task tracker designed to generate formatted daily standup reports. Tasks support up to 3 levels of nesting: task -> subtask -> sub-subtask.

---

## Data Model

### Project

| Field        | Type     | Description                                       |
|--------------|----------|---------------------------------------------------|
| `id`         | UUID     | Primary key                                       |
| `slug`       | TEXT     | Unique short identifier used in CLI (e.g. `work`) |
| `title`      | TEXT     | Optional display title (e.g. `Work Projects`)     |
| `created_at` | DATETIME | Creation timestamp                                |

`slug` is the primary identifier in all CLI commands. `title` is shown in listings but never typed. Projects must be created explicitly before adding tasks. All tasks belong to exactly one project.

### Task

| Field        | Type     | Description                                        |
|--------------|----------|----------------------------------------------------|
| `id`         | UUID     | Primary key                                        |
| `project_id` | UUID     | Foreign key to `Project`                           |
| `title`      | TEXT     | Task name                                          |
| `status`     | TEXT     | `not_started` / `in_progress` / `done` / `blocked` |
| `parent_id`  | UUID?    | Reference to parent task (nullable)                |
| `order`      | INTEGER  | Display order among siblings                       |
| `created_at` | DATETIME | Creation timestamp                                 |
| `updated_at` | DATETIME | Last status change timestamp                       |

### StatusChange

| Field        | Type     | Description                  |
|--------------|----------|------------------------------|
| `id`         | UUID     | Primary key                  |
| `task_id`    | UUID     | Foreign key to `Task`        |
| `old_status` | TEXT     | Status before the transition |
| `new_status` | TEXT     | Status after the transition  |
| `changed_at` | DATETIME | When the transition occurred |

Every status transition is recorded automatically. Used to reconstruct historical task state for `--date` reports.

### Constraints

- Max nesting depth: 3 levels
- A task with children cannot be marked as `done` - returns an error if any child is still active
- Blocking a task cascades to all active descendants
- `order` is maintained per parent scope (siblings only)
- Tasks cannot be moved across projects

---

## Status Transitions

```
not_started -> in_progress -> done
not_started -> blocked
in_progress -> blocked
blocked     -> in_progress
any         -> not_started  (reset)
```

---

## Report Format

### Previously

Tasks that were active on the previous **workday** or had a status change on that day, with statuses reconstructed from the change log.

Weekend logic:

- Monday/Saturday/Sunday: previous workday is Friday
- Tuesday-Friday: previous day

### Today

"Morning plan" view - the same task set as Current, but with modified icons to simulate the state at the beginning of the workday:

| Condition                           | Icon      |
|-------------------------------------|-----------|
| Task created today (any status)     | ❌         |
| Task created earlier, status `done` | ❌         |
| Task created earlier, other status  | real icon |

This allows adding new tasks throughout the day while maintaining a stable "planned" view.

### Current

Actual state of today's tasks with real status icons. Uses the same task set as Today.

A task appears if it was active on `date` or had a status change on `date`. A task can appear in both Previously and Current simultaneously - e.g. a task started yesterday will show in Previously (status changed) and in Current (still active).

Implementation: `tasks_snapshot(date)` - reconstructs task statuses from the status change log.

### Output Rules

- Each nesting level adds one ` - ` prefix segment
- Format: `[indent] [icon] [title]`
- Indent level 1: ` - 🔄 Milestone`
- Indent level 2: ` - - ✅ Task`
- Indent level 3: ` - - - ❌ Subtask`
- Tasks are sorted by `order` within their parent scope
- Parent tasks are shown even if only some children match the filter

---

## CLI Interface

### Flags

| Flag             | Short | Description                      |
|------------------|-------|----------------------------------|
| `--project`      | `-p`  | Project scope or management mode |
| `--task`         | `-t`  | Task management mode             |
| `--report`       | `-r`  | Report mode                      |
| `--add`          | `-a`  | Add a task or project            |
| `--list`         | `-l`  | List tasks or projects           |
| `--start`        | `-s`  | Set status: in_progress          |
| `--done`         | `-d`  | Set status: done                 |
| `--block`        | `-b`  | Set status: blocked              |
| `--move`         | `-m`  | Move or reorder a task           |
| `--rename`       |       | Rename a task                    |
| `--remove`       |       | Delete task or project           |
| `--reset`        |       | Set status: not_started          |
| `--under <id>`   | `-u`  | Parent task id                   |
| `--order <n>`    | `-o`  | Sibling position                 |
| `--title <text>` |       | Display title for a project      |
| `--all`          |       | Include done/blocked             |
| `--copy`         | `-c`  | Copy output to clipboard         |
| `--previously`   |       | Only Previously section          |
| `--today`        |       | Only Today section               |
| `--current`      |       | Only Current section             |
| `--date <date>`  |       | Report for specific date         |

### Project Management

```
tick -p                                Show active project slug and title
tick -p -l                             List all projects (slug + title)
tick -p -a <slug>                      Create a new project
tick -p -a <slug> --title "Full title" Create a project with a display title
tick -p <slug>                         Switch active project
tick -p --rename <slug> <new-title>     Change project display title
tick -p --reslug <slug> <new-slug>     Change project slug
tick -p --remove <slug>                Delete project and all its tasks
```

The active project is stored in `~/.local/share/tick/config.toml`. All `-t` and `-r` commands operate on the active project unless `-p <slug>` is prepended.

### Task Management

```
tick -t -a <title>                     Add a root task
tick -t -a <title> -u <id>             Add a child task
tick -t -l                             List active tasks (tree view)
tick -t -l --all                       List all tasks including done/blocked
tick -t -s <id>                        Set status to in_progress
tick -t -d <id>                        Set status to done
tick -t -b <id>                        Set status to blocked
tick -t --reset <id>                   Set status to not_started
tick -t -m <id> -u <id>                Move task under a new parent
tick -t -m <id> -o <n>                 Change display order
tick -t --rename <id> <title>          Rename a task
tick -t --remove <id>                  Delete task (and its children)

tick -p <slug> -t -l                   List tasks in a specific project
```

### Report

```
tick -r                                Print standup report to stdout
tick -r --previously                   Print only the Previously section
tick -r --today                        Print only the Today section
tick -r --current                      Print only the Current section
tick -r -c                             Copy report to clipboard (macOS: pbcopy)
tick -r --date <YYYY-MM-DD>            Generate report for a specific date

tick -p <slug> -r                      Report for a specific project
tick -p <slug> -r -c                   Copy report for a specific project
```

### Other

```
tick --help / -h                       Show help
tick --version / -V                    Show version
```

---

## Technical Debt

- `TaskRepository::list_all(project_id)` is used in `tasks_snapshot()` and `TaskService::create()` for different purposes. As the task count grows, this becomes inefficient. Replace with specialized queries: `list_roots(project_id)` (for order calculation) and `list_until(project_id, date)` (for report snapshots).

---

## Architecture

### v0.1 - Projects

Introduce multi-project support:

- Add `projects` table with `id`, `slug`, `title`, `created_at`
- Add `project_id` column to `tasks`
- Add `-p` / `--project` flag to all commands
- Active project persisted in config
- `tick -p` management commands: add, list, switch, remove
- Projects must be created explicitly before adding tasks

### v0.2 - CLI + SQLite

```
crates/
  cli/        - argument parsing (clap), output formatting
  domain/     - domain logic: task CRUD, report generation, date logic
  db/         - SQLite persistence via rusqlite or sqlx
```

Single binary, no server. Database stored at `~/.local/share/tick/tick.db` (XDG). Active project stored at `~/.local/share/tick/config.toml`.

### v0.3 - TUI

Add `crates/tui/` using `ratatui`. Core logic stays unchanged. Project switcher panel included from the start.

### v0.4 - Web (React + REST API)

Add `crates/api/` with axum. Frontend in a separate repo or `web/` directory. SQLite remains for single-user mode.

### v0.5 - Multi-user

- Migrate to PostgreSQL
- Add `users`, `roles`, `sessions` tables
- JWT authentication
- Per-user project isolation

### v0.x - Hubstaff Integration (potential)

Optional integration with [Hubstaff API v2](https://developer.hubstaff.com/docs/hubstaff_v2)
via Personal Access Token. No server-side OAuth required for single-user CLI.

**Key facts (verified):**

- Personal Access Token generated at `https://developer.hubstaff.com/personal_access_tokens`
  is a **refresh token** - must be exchanged for an access token before use
- Token exchange endpoint: `POST https://account.hubstaff.com/access_tokens`
- Access token lifetime: **24 hours**. Refresh token lifetime: **~8 days**
- `GET /v2/organizations/{id}/projects` is accessible to regular Members and returns only their assigned projects - not all organization projects

**Token lifecycle:**

```
setup:    HUBSTAFF_REFRESH_TOKEN -> exchange -> access_token + expiry -> save to config.toml
runtime:  check expiry -> if expired, re-exchange -> use access_token for API calls
```

**Config storage (`~/.local/share/tick/config.toml`):**

```toml
[hubstaff]
access_token = "eyJ..."
access_token_expires_at = "2026-02-23T15:00:00Z"
organization_id = 12345   # fetched once during setup, never again
```

**Setup command:**

```
tick hubstaff setup   # prompts for refresh token, fetches org_id, saves to config
```

**Potential use cases:**

| Use case          | Description                                         |
|-------------------|-----------------------------------------------------|
| Project linking   | Bind a tick project slug to a Hubstaff project id   |
| Report enrichment | Show tracked time per task alongside standup report |

**CLI sketch:**

```
tick hubstaff setup                                # one-time auth setup
tick -p work --hubstaff-id <hubstaff_project_id>   # link tick project to Hubstaff project
tick -r --with-time                                # report with tracked hours
```

**Data model additions:**

- `Project.hubstaff_project_id: Option<i64>`
- `Task.hubstaff_task_id: Option<i64>`

