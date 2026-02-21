# tick

Task tracker with automatic daily standup report generation for team chats.

## What it does

You manage tasks and subtasks in `tick`. At the end of the day, you run one command and get a formatted text block ready to paste into your project chat:

```
Previously:
 - 🔄 Milestone name #1
 - - ✅ task name #1
 - - - ✅ subtask name #1
 - 🔄 Milestone name #2
 - - ❌ task name #1

Today:
 - 🔄 Milestone name #1
 - 🔄 Milestone name #2
 - - ✅ task name #1
 - 🛑 Milestone name #3
```

## Status Icons

| Icon | Meaning     |
|------|-------------|
| 🔄   | In progress |
| ✅    | Done        |
| ❌    | Not started |
| 🛑   | Blocked     |

## Roadmap

| Version | Description                         | Status |
|---------|-------------------------------------|--------|
| v0.1    | CLI + SQLite, single user           | 🔄     |
| v0.2    | TUI frontend                        | ⏳      |
| v0.3    | React frontend with Kanban view     | ⏳      |
| v0.4    | PostgreSQL, multi-user, roles, auth | ⏳      |

## Usage (v0.1 CLI)

```bash
# Tasks
tick -t -a "Fix login bug"
tick -t -a "Fix login bug" -p <id>
tick -t -l
tick -t -d <id>
tick -t -b <id>
tick -t --remove <id>

# Report
tick -r        # print today's report
tick -r -c     # copy to clipboard
```

## Setup

```bash
cp .env.example .env
make run
```
