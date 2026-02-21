# tick — Technical Specification

## Overview

`tick` is a task tracker designed to generate formatted daily standup reports. Tasks support up to 3 levels of nesting: task → subtask → sub-subtask.

---

## Data Model

### Task

| Field        | Type     | Description                                        |
|--------------|----------|----------------------------------------------------|
| `id`         | UUID     | Primary key                                        |
| `title`      | TEXT     | Task name                                          |
| `status`     | TEXT     | `not_started` / `in_progress` / `done` / `blocked` |
| `parent_id`  | UUID?    | Reference to parent task (nullable)                |
| `order`      | INTEGER  | Display order among siblings                       |
| `created_at` | DATETIME | Creation timestamp                                 |
| `updated_at` | DATETIME | Last update timestamp                              |

### Constraints

- Max nesting depth: 3 levels
- A task with children cannot be directly marked as `done` — children must be completed first (warn, not hard block)
- `order` is maintained per parent scope (siblings only)

---

## Status Transitions

```
not_started → in_progress → done
not_started → blocked
in_progress → blocked
blocked     → in_progress
any         → not_started  (reset)
```

---

## Report Format

### Previously

Tasks that were `done` or `blocked` on the previous **workday**.

Weekend logic:

- Monday: includes Friday + Saturday + Sunday
- Tuesday–Friday: includes previous day only

### Today

Tasks that are `not_started` or `in_progress` as of today, shown in full hierarchy.

### Output Rules

- Each nesting level adds one ` - ` prefix segment
- Format: `[indent] [icon] [title]`
- Indent level 1: ` - 🔄 Milestone`
- Indent level 2: ` - - ✅ Task`
- Indent level 3: ` - - - ❌ Subtask`
- Tasks are sorted by `order` within their parent scope
- Parent tasks are shown even if only some children match the filter

---

## CLI Interface (v0.1)

### Flags

| Flag            | Short | Description              |
|-----------------|-------|--------------------------|
| `--task`        | `-t`  | Task management mode     |
| `--report`      | `-r`  | Report mode              |
| `--add`         | `-a`  | Add a task               |
| `--list`        | `-l`  | List tasks               |
| `--start`       | `-s`  | Set status: in_progress  |
| `--done`        | `-d`  | Set status: done         |
| `--block`       | `-b`  | Set status: blocked      |
| `--move`        | `-m`  | Move or reorder a task   |
| `--remove`      |       | Delete task              |
| `--reset`       |       | Set status: not_started  |
| `--parent <id>` | `-p`  | Parent task id           |
| `--order <n>`   | `-o`  | Sibling position         |
| `--all`         |       | Include done/blocked     |
| `--copy`        | `-c`  | Copy output to clipboard |
| `--previously`  |       | Only Previously section  |
| `--today`       |       | Only Today section       |
| `--date <date>` |       | Report for specific date |

### Task Management

```
tick -t -a <title>                     Add a root task
tick -t -a <title> -p <id>             Add a child task
tick -t -l                             List active tasks (tree view)
tick -t -l --all                       List all tasks including done/blocked
tick -t -s <id>                        Set status to in_progress
tick -t -d <id>                        Set status to done
tick -t -b <id>                        Set status to blocked
tick -t --reset <id>                   Set status to not_started
tick -t -m <id> -p <id>                Move task under a new parent
tick -t -m <id> -o <n>                 Change display order
tick -t --rename <id> <title>          Rename a task
tick -t --remove <id>                  Delete task (and its children)
```

### Report

```
tick -r                                Print standup report to stdout
tick -r --previously                   Print only the Previously section
tick -r --today                        Print only the Today section
tick -r -c                             Copy report to clipboard (macOS: pbcopy)
tick -r --date <YYYY-MM-DD>            Generate report for a specific date
```

### Other

```
tick --help / -h                       Show help
tick --version / -V                    Show version
```

---

## Architecture

### v0.1 — CLI + SQLite

```
crates/
  cli/        — argument parsing (clap), output formatting
  core/       — domain logic: task CRUD, report generation, date logic
  db/         — SQLite persistence via rusqlite or sqlx
```

Single binary, no server. Database stored at `~/.local/share/tick/tick.db` (XDG).

### v0.2 — TUI

Add `crates/tui/` using `ratatui`. Core logic stays unchanged.

### v0.3 — Web (React + REST API)

Add `crates/api/` with axum. Frontend in a separate repo or `web/` directory. SQLite remains for single-user mode.

### v0.4 — Multi-user

- Migrate to PostgreSQL
- Add `users`, `roles`, `sessions` tables
- JWT authentication
- Per-user task isolation

---

## Environment

```env
DATABASE_URL=sqlite://~/.local/share/tick/tick.db
```

---

## Open Questions

- [ ] Should `tick report` show tasks with no activity today? (e.g. carry-over not_started tasks)
- [ ] How to handle tasks created and completed on the same day in Previously?
- [ ] Should `order` be auto-assigned (append) or require manual input?
